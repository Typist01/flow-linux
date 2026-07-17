/// Show a desktop notification for a user-visible error.
///
/// Runs on a dedicated thread so notify-rust/zbus cannot nest a Tokio runtime
/// inside the daemon's async runtime (which panics).
pub fn notify_error(body: impl Into<String>) {
    let body = body.into();
    let _ = std::thread::Builder::new()
        .name("flow-notify".into())
        .spawn(move || {
            match notify_rust::Notification::new()
                .summary("Flow Linux")
                .body(&body)
                .appname("Flow Linux")
                .timeout(notify_rust::Timeout::Milliseconds(5000))
                .show()
            {
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "failed to show desktop notification"),
            }
        });
}
