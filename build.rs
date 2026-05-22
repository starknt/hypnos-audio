use std::io;
use std::path::Path;

fn version_quad() -> String {
    let v = env!("CARGO_PKG_VERSION");
    let mut parts: Vec<u16> = v.split('.').filter_map(|s| s.parse().ok()).collect();
    while parts.len() < 4 {
        parts.push(0);
    }
    format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3])
}

fn main() -> io::Result<()> {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return Ok(());
    }

    let mut res = winresource::WindowsResource::new();

    if Path::new("assets/icon.ico").exists() {
        res.set_icon("assets/icon.ico");
    }

    res.set("FileDescription", "Hypnos Audio - Automatic headset mute");
    res.set("ProductName", "Hypnos Audio");
    res.set("LegalCopyright", "Copyright (c) 2025");

    let v = version_quad();
    res.set("FileVersion", &v);
    res.set("ProductVersion", &v);

    res.compile()
}
