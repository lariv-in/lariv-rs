//! Request-scoped URI path for path-aware shell chrome (multi-PWA, etc.).

use std::future::Future;

tokio::task_local! {
    static REQUEST_PATH: String;
}

/// Path of the current HTTP request (`req.uri().path()`).
///
/// `None` outside a request (unit tests, background work).
pub fn request_path() -> Option<String> {
    REQUEST_PATH.try_with(|p| p.clone()).ok()
}

/// Run `fut` with [`request_path`] set to `path`.
pub async fn scope_request_path<F>(path: impl Into<String>, fut: F) -> F::Output
where
    F: Future,
{
    REQUEST_PATH.scope(path.into(), fut).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_path_defaults_none_outside_scope() {
        assert!(request_path().is_none());
    }

    #[tokio::test]
    async fn scope_sets_request_path() {
        assert!(request_path().is_none());
        scope_request_path("/kiosk/orders", async {
            assert_eq!(request_path().as_deref(), Some("/kiosk/orders"));
        })
        .await;
        assert!(request_path().is_none());
    }
}
