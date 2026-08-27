//! Helpers for reporting ignored [`Result`]s without changing call-site UX.

/// On [`Err`], log and return [`None`]; on [`Ok`], return the inner option unchanged.
///
/// Use for find/load paths that already treat missing rows as empty / redirect / 404.
pub fn opt_or_log<T, E: std::fmt::Display>(result: Result<Option<T>, E>, msg: &str) -> Option<T> {
    match result {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "{msg}");
            None
        }
    }
}

/// Log [`Err`] and discard; ignore [`Ok`].
pub fn log_err<E: std::fmt::Display>(result: Result<(), E>, msg: &str) {
    if let Err(e) = result {
        tracing::error!(error = %e, "{msg}");
    }
}

/// Log [`Err`] at warn level (peer disconnect, best-effort notify, etc.).
pub fn log_warn<E: std::fmt::Display>(result: Result<(), E>, msg: &str) {
    if let Err(e) = result {
        tracing::warn!(error = %e, "{msg}");
    }
}
