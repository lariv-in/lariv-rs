//! Background IMAP IDLE listener — triages new inbox messages via [`super::email_pipeline`].

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use async_imap::Session;
use async_imap::extensions::idle::IdleResponse;
use async_imap::imap_proto::Address;
use futures_util::StreamExt;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use super::email_pipeline::process_inbound_email;
use super::entities::LlmAssistantPreferences;
use super::preferences::{load_preferences, mail_encryption_or_default};
use super::state::LlmAssistantState;

type ImapStream = TlsStream<TcpStream>;
type ImapSession = Session<ImapStream>;

const LOG_TARGET: &str = "llm_assistant::imap";

macro_rules! imap_status {
    ($($tt:tt)*) => {
        // Deployments often default to `warn`; use warn so connection status is visible.
        tracing::warn!(target: LOG_TARGET, $($tt)*)
    };
}

const RECONNECT_DELAY: Duration = Duration::from_secs(30);
const CONFIG_POLL_DELAY: Duration = Duration::from_secs(60);
const IDLE_TIMEOUT: Duration = Duration::from_secs(28 * 60);

/// Signals the background listener to drop its session and reconnect.
#[derive(Clone)]
pub struct EmailListenerHandle {
    state: Arc<OnceLock<Arc<LlmAssistantState>>>,
    restart: Arc<Notify>,
    started: Arc<AtomicBool>,
}

impl EmailListenerHandle {
    /// Register the shared assistant state used by the background task.
    pub fn bind(&self, state: Arc<LlmAssistantState>) {
        let _ = self.state.set(state);
    }

    /// Shared assistant state once [`Self::bind`] has run.
    pub fn shared_state(&self) -> Option<Arc<LlmAssistantState>> {
        self.state.get().cloned()
    }

    /// Ensure the background task is running (idempotent).
    pub fn ensure_started(&self) {
        if self
            .started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        let state_slot = Arc::clone(&self.state);
        let restart = self.restart.clone();
        tokio::spawn(async move {
            imap_status!("IMAP listener task started");
            loop {
                let Some(state) = state_slot.get().cloned() else {
                    tracing::error!(target: LOG_TARGET, "email listener started before state bind");
                    tokio::time::sleep(RECONNECT_DELAY).await;
                    continue;
                };
                match run_session(&state, &restart).await {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::error!(target: LOG_TARGET, "IMAP listener error: {e:#}");
                        if wait_or_restart(&restart, RECONNECT_DELAY).await {
                            continue;
                        }
                    }
                }
            }
        });
    }

    /// Interrupt the current IMAP session so it reconnects with fresh preferences.
    pub fn restart(&self) {
        self.ensure_started();
        imap_status!("restarting IMAP listener");
        self.restart.notify_waiters();
    }
}

/// Create a handle; call [`EmailListenerHandle::bind`] then [`EmailListenerHandle::ensure_started`].
pub fn new_handle() -> EmailListenerHandle {
    EmailListenerHandle {
        state: Arc::new(OnceLock::new()),
        restart: Arc::new(Notify::new()),
        started: Arc::new(AtomicBool::new(false)),
    }
}

/// Backwards-compatible alias for [`new_handle`].
pub fn new() -> EmailListenerHandle {
    new_handle()
}

async fn wait_or_restart(restart: &Arc<Notify>, delay: Duration) -> bool {
    tokio::select! {
        () = tokio::time::sleep(delay) => false,
        () = restart.notified() => true,
    }
}

/// IMAP connection settings resolved from preferences.
#[derive(Debug, Clone)]
struct EmailImapConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
    tls_mode: ImapTlsMode,
}

#[derive(Debug, Clone, Copy)]
enum ImapTlsMode {
    /// TLS from the first byte (typical port 993).
    Implicit,
    /// Plain connect then STARTTLS (typical port 143).
    StartTls,
}

impl ImapTlsMode {
    fn resolve(encryption: &str, port: u16) -> Self {
        match port {
            // Standard ports take precedence over the encryption preference.
            993 => Self::Implicit,
            143 => Self::StartTls,
            _ if encryption == "ssl" => Self::Implicit,
            _ => Self::StartTls,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Implicit => "SSL",
            Self::StartTls => "STARTTLS",
        }
    }
}

impl EmailImapConfig {
    fn from_prefs(prefs: &LlmAssistantPreferences) -> Result<Self, MissingImapConfig> {
        let host = prefs.imap_server.trim();
        let username = prefs.email.trim();
        let password = prefs.password.trim();
        if host.is_empty() {
            return Err(MissingImapConfig::ImapServer);
        }
        if username.is_empty() {
            return Err(MissingImapConfig::Email);
        }
        if password.is_empty() {
            return Err(MissingImapConfig::Password);
        }

        let encryption = mail_encryption_or_default(&prefs.mail_encryption);
        let default_port = if encryption == "tls" { 143 } else { 993 };
        let port = prefs
            .imap_port
            .trim()
            .parse::<u16>()
            .unwrap_or(default_port);
        let tls_mode = ImapTlsMode::resolve(&encryption, port);

        if encryption == "tls" && port == 993 {
            tracing::warn!(
                target: LOG_TARGET,
                "encryption is TLS but port 993 requires implicit SSL; connecting with SSL"
            );
        } else if encryption == "ssl" && port == 143 {
            tracing::warn!(
                target: LOG_TARGET,
                "encryption is SSL but port 143 typically uses STARTTLS; connecting with STARTTLS"
            );
        }

        Ok(Self {
            host: host.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            tls_mode,
            port,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum MissingImapConfig {
    ImapServer,
    Email,
    Password,
}

impl MissingImapConfig {
    fn field(self) -> &'static str {
        match self {
            Self::ImapServer => "IMAP server",
            Self::Email => "email",
            Self::Password => "password",
        }
    }
}

async fn run_session(state: &LlmAssistantState, restart: &Arc<Notify>) -> anyhow::Result<()> {
    let prefs = load_preferences(&state.db).await?;
    let config = match EmailImapConfig::from_prefs(&prefs) {
        Ok(config) => config,
        Err(missing) => {
            imap_status!("IMAP listener waiting — {} not configured", missing.field());
            if wait_or_restart(restart, CONFIG_POLL_DELAY).await {
                return Ok(());
            }
            return Ok(());
        }
    };

    imap_status!(
        "connecting to IMAP {}:{} as {} ({})",
        config.host,
        config.port,
        config.username,
        config.tls_mode.label()
    );

    let mut session = connect(&config).await?;
    session.select("INBOX").await?;
    let mut last_uid = highest_uid(&mut session).await?;

    imap_status!(
        "IMAP IDLE listening on {}:{} as {} (last uid {last_uid})",
        config.host,
        config.port,
        config.username
    );

    loop {
        let mut handle = session.idle();
        handle.init().await?;
        let (idle_wait, stop) = handle.wait_with_timeout(IDLE_TIMEOUT);
        let interrupt = tokio::spawn({
            let restart = restart.clone();
            async move {
                restart.notified().await;
                drop(stop);
            }
        });
        let idle_result = idle_wait.await?;
        interrupt.abort();
        session = handle.done().await?;

        match idle_result {
            IdleResponse::NewData(_) => {
                process_new_messages(&mut session, &mut last_uid, state).await?;
            }
            IdleResponse::Timeout => {}
            IdleResponse::ManualInterrupt => return Ok(()),
        }
    }
}

async fn connect(config: &EmailImapConfig) -> anyhow::Result<ImapSession> {
    match config.tls_mode {
        ImapTlsMode::Implicit => connect_implicit(config).await,
        ImapTlsMode::StartTls => connect_starttls(config).await,
    }
}

async fn connect_implicit(config: &EmailImapConfig) -> anyhow::Result<ImapSession> {
    let tcp = TcpStream::connect((config.host.as_str(), config.port)).await?;
    let tls = build_tls_connector()?;
    let server_name = ServerName::try_from(config.host.clone())
        .map_err(|_| anyhow::anyhow!("invalid IMAP server name: {}", config.host))?;
    let stream = tls.connect(server_name, tcp).await?;
    login(config, stream).await
}

async fn connect_starttls(config: &EmailImapConfig) -> anyhow::Result<ImapSession> {
    let tcp = TcpStream::connect((config.host.as_str(), config.port)).await?;
    let mut plain_client = async_imap::Client::new(tcp);
    let _greeting = plain_client.read_response().await?.ok_or_else(|| {
        anyhow::anyhow!(
            "no greeting from IMAP server on {}:{} (check port: 143 for STARTTLS, 993 for SSL)",
            config.host,
            config.port
        )
    })?;
    plain_client
        .run_command_and_check_ok("STARTTLS", None)
        .await?;
    let tcp = plain_client.into_inner();
    let tls = build_tls_connector()?;
    let server_name = ServerName::try_from(config.host.clone())
        .map_err(|_| anyhow::anyhow!("invalid IMAP server name: {}", config.host))?;
    let stream = tls.connect(server_name, tcp).await?;
    login(config, stream).await
}

async fn login(config: &EmailImapConfig, stream: ImapStream) -> anyhow::Result<ImapSession> {
    let client = async_imap::Client::new(stream);
    client
        .login(&config.username, &config.password)
        .await
        .map_err(|(err, _)| anyhow::anyhow!("IMAP login failed: {err}"))
}

fn build_tls_connector() -> anyhow::Result<TlsConnector> {
    let mut roots = RootCertStore::empty();
    let certs = rustls_native_certs::load_native_certs();
    for cert in certs.certs {
        roots.add(cert)?;
    }
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

async fn highest_uid(session: &mut ImapSession) -> anyhow::Result<u32> {
    let uids = session.uid_search("ALL").await?;
    Ok(uids.into_iter().max().unwrap_or(0))
}

async fn process_new_messages(
    session: &mut ImapSession,
    last_uid: &mut u32,
    state: &LlmAssistantState,
) -> anyhow::Result<()> {
    let query = format!("UID {}:*", last_uid.saturating_add(1));
    let mut uids: Vec<u32> = session.uid_search(&query).await?.into_iter().collect();
    uids.sort_unstable();
    if uids.is_empty() {
        return Ok(());
    }

    let uid_set = uids
        .iter()
        .map(|uid| uid.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let mut stream = session.uid_fetch(&uid_set, "(UID ENVELOPE RFC822)").await?;
    while let Some(fetch) = stream.next().await {
        let fetch = fetch?;
        let uid = fetch.uid.unwrap_or(0);
        if uid <= *last_uid {
            continue;
        }
        *last_uid = (*last_uid).max(uid);

        let from = fetch
            .envelope()
            .and_then(|env| env.from.as_ref())
            .map(|addrs| format_addresses(addrs.as_slice()))
            .unwrap_or_else(|| "(unknown)".to_string());
        let subject = fetch
            .envelope()
            .and_then(|env| env.subject.as_ref())
            .map(cow_to_string)
            .unwrap_or_else(|| "(no subject)".to_string());
        let raw = fetch.body().map(|b| b.to_vec()).unwrap_or_default();

        imap_status!("new email uid={uid} from={from} subject={subject}");
        if let Some(state_arc) = state.email_listener.shared_state() {
            process_inbound_email(state_arc, uid, from, subject, raw);
        }
    }

    Ok(())
}

fn format_addresses(addrs: &[Address<'_>]) -> String {
    addrs
        .iter()
        .map(format_address)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_address(addr: &Address<'_>) -> String {
    let mailbox = addr.mailbox.as_deref().map(cow_bytes_to_str).unwrap_or("");
    let host = addr.host.as_deref().map(cow_bytes_to_str).unwrap_or("");
    if mailbox.is_empty() && host.is_empty() {
        "(unknown)".to_string()
    } else if host.is_empty() {
        mailbox.to_string()
    } else {
        format!("{mailbox}@{host}")
    }
}

fn cow_to_string(raw: &std::borrow::Cow<'_, [u8]>) -> String {
    String::from_utf8_lossy(raw).into_owned()
}

fn cow_bytes_to_str(raw: &[u8]) -> &str {
    std::str::from_utf8(raw).unwrap_or("")
}
