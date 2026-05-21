use anyhow::{Context, Result};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ, RegCloseKey, RegDeleteValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::HSTRING;

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APP_NAME: &str = "HypnosAudio";

/// Check whether the app is registered to launch on Windows startup.
pub fn is_enabled() -> Result<bool> {
    unsafe {
        let mut hkey = HKEY::default();
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(RUN_KEY_PATH),
            None,
            KEY_READ,
            &mut hkey,
        )
        .ok()?;

        let result = RegQueryValueExW(hkey, &HSTRING::from(APP_NAME), None, None, None, None);

        let _ = RegCloseKey(hkey);

        match result.0 {
            0 => Ok(true),  // ERROR_SUCCESS
            2 => Ok(false), // ERROR_FILE_NOT_FOUND
            _ => Err(result.ok().unwrap_err().into()),
        }
    }
}

/// Register or unregister the app to launch on Windows startup.
pub fn set_enabled(enabled: bool) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(RUN_KEY_PATH),
            None,
            KEY_WRITE,
            &mut hkey,
        )
        .ok()?;

        if enabled {
            let exe_path =
                std::env::current_exe().context("failed to get current executable path")?;
            let path_str = exe_path.to_string_lossy();
            let path_wstr = HSTRING::from(path_str.as_ref());

            let bytes = std::slice::from_raw_parts(
                path_wstr.as_ptr() as *const u8,
                (path_wstr.len() + 1) * std::mem::size_of::<u16>(),
            );

            RegSetValueExW(hkey, &HSTRING::from(APP_NAME), None, REG_SZ, Some(bytes)).ok()?;
        } else {
            let result = RegDeleteValueW(hkey, &HSTRING::from(APP_NAME));
            if result.0 != 0 && result.0 != 2 {
                result.ok()?;
            }
        }

        let _ = RegCloseKey(hkey);
        Ok(())
    }
}
