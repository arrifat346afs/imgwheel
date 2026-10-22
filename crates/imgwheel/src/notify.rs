use notify_rust::Notification;
use tracing::{info, warn};

pub fn send_notification(summary: &str, body: &str, is_error: bool) {
    let mut notification = Notification::new();
    notification.appname("imgwheel").summary(summary).body(body);

    if is_error {
        notification.urgency(notify_rust::Urgency::Critical);
    } else {
        notification.urgency(notify_rust::Urgency::Normal);
    }

    if let Err(e) = notification.show() {
        warn!("Failed to display desktop notification: {}", e);
    } else {
        info!("Notification sent: {} - {}", summary, body);
    }
}
