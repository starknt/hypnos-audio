use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::ToastNotificationManager;

pub fn show(title: &str, body: &str, tag: Option<&str>) {
    let icon_xml = icon_xml();

    let xml = format!(
        r#"<toast>
  <visual>
    <binding template="ToastGeneric">
      {}
      <text>{}</text>
      <text>{}</text>
    </binding>
  </visual>
</toast>"#,
        icon_xml,
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

    if let Some(t) = tag {
        let _ = toast.SetTag(&windows::core::HSTRING::from(t));
    }

    let notifier = match ToastNotificationManager::CreateToastNotifierWithId(
        &windows::core::HSTRING::from("Hypnos Audio"),
    ) {
        Ok(n) => n,
        Err(_) => return,
    };

    let _ = notifier.Show(&toast);
}

fn icon_xml() -> String {
    let Some(path) = icon_path() else {
        return String::new();
    };
    format!(
        r#"<image placement="appLogoOverride" hint-crop="circle" src="{}"/>"#,
        xml_escape(&path)
    )
}

fn icon_path() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;

    for rel in ["assets/icon.png", "assets/icon.ico", "icon.png"] {
        let candidate = dir.join(rel);
        if candidate.exists() {
            let abs = candidate.canonicalize().unwrap_or(candidate);
            let path = abs.to_string_lossy().replace("\\", "/");
            let path = path.strip_prefix("//?/").unwrap_or(&path);
            return Some("file:///".to_string() + &path);
        }
    }
    None
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
}
