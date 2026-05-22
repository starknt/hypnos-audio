pub fn show(title: &str, body: &str) {
    if let Err(e) = notify_rust::Notification::new()
        .appname("Hypnos Audio")
        .summary(title)
        .body(body)
        .show()
    {
        tracing::warn!(error = %e, "failed to show notification");
    }
}
