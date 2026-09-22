//! Embedded `moq-relay` listener (QUIC + HTTPS/WebTransport + WebSocket fallback).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use moq_relay::{Config, Relay};

use super::config::TransportConfig;
use super::moq_auth::MoqAuthState;
use super::state::MeetsState;

/// Spawn the MoQ relay and store its cluster on [`MeetsState`] for recording.
pub async fn start(state: Arc<MeetsState>) -> anyhow::Result<()> {
    let transport = &state.config.transport;
    if !transport.enabled {
        tracing::info!("meets: MoQ relay disabled");
        return Ok(());
    }

    let addr: SocketAddr = transport
        .bind
        .parse()
        .map_err(|e| anyhow::anyhow!("meets.transport.bind: {e}"))?;

    let mut config = Config::default();
    config.server.bind = Some(transport.bind.clone());
    configure_tls(transport, addr, &mut config)?;
    configure_auth(&state.moq_auth, &mut config)?;

    let relay = Relay::load(config).await?;
    {
        let mut guard = state.relay_cluster.write().await;
        *guard = Some(relay.cluster.clone());
    }

    tokio::spawn(async move {
        if let Err(err) = relay.run().await {
            tracing::error!(error = %err, "meets: MoQ relay stopped");
        }
    });

    tracing::info!(%addr, "meets: MoQ relay listening");
    Ok(())
}

fn configure_tls(
    transport: &TransportConfig,
    addr: SocketAddr,
    config: &mut Config,
) -> anyhow::Result<()> {
    if !transport.cert_file.is_empty() && !transport.key_file.is_empty() {
        let cert = PathBuf::from(&transport.cert_file);
        let key = PathBuf::from(&transport.key_file);
        config.server.tls.cert = vec![cert.clone()];
        config.server.tls.key = vec![key.clone()];
        config.web.https.listen = Some(addr);
        config.web.https.cert = vec![cert];
        config.web.https.key = vec![key];
        return Ok(());
    }

    tracing::warn!(
        "meets: no transport certFile/keyFile; generating self-signed certificate for development"
    );
    config.server.tls.generate = vec!["localhost".into(), "127.0.0.1".into()];
    config.web.https.listen = Some(addr);
    config.web.https.cert = config.server.tls.cert.clone();
    config.web.https.key = config.server.tls.key.clone();
    Ok(())
}

fn configure_auth(auth: &MoqAuthState, config: &mut Config) -> anyhow::Result<()> {
    config.auth.key_dir = Some(auth.key_dir().to_string_lossy().into_owned());
    Ok(())
}
