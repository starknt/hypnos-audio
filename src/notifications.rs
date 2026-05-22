use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::ToastNotificationManager;

pub fn show(title: &str, body: &str) {
    let xml = format!(
        r#"<toast>
  <visual>
    <binding template="ToastText02">
      <text id="1">{}</text>
      <text id="2">{}</text>
    </binding>
  </visual>
</toast>"#,
        xml_escape(title),
        xml_escape(body)
    );

    let doc = match XmlDocument::new() {
        Ok(d) => d,
        Err(_) => return,
    };

    if doc.LoadXml(&windows::core::HSTRING::from(&xml)).is_err() {
        return;
    }

    let toast = match windows::UI::Notifications::ToastNotification::CreateToastNotification(&doc) {
        Ok(t) => t,
        Err(_) => return,
    };

    let notifier = match ToastNotificationManager::CreateToastNotifierWithId(&windows::core::HSTRING::from("Hypnos Audio")) {
        Ok(n) => n,
        Err(_) => return,
    };

    let _ = notifier.Show(&toast);
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
