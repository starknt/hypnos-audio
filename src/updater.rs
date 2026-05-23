use crate::Result;
use velopack::{UpdateCheck, UpdateManager, sources};

const GITHUB_REPO: &str = "starknt/hypnos-audio";

fn create_manager() -> Result<UpdateManager> {
    let repo_url = format!("https://github.com/{}", GITHUB_REPO);
    let source = sources::GithubSource::new(&repo_url, None, false);
    Ok(UpdateManager::new(source, None, None)?)
}

/// Check for updates and download them silently.
/// Returns `true` if an update was found and downloaded.
pub fn check_and_download() -> Result<bool> {
    let um = create_manager()?;

    match um.check_for_updates() {
        Ok(UpdateCheck::UpdateAvailable(updates)) => {
            tracing::info!(version = %updates.TargetFullRelease.Version, "update available, downloading");
            um.download_updates(&updates, None)?;
            tracing::info!("update downloaded, will apply on next restart");
            Ok(true)
        }
        Ok(_) => {
            tracing::info!("no updates available");
            Ok(false)
        }
        Err(e) => {
            tracing::warn!(error = %e, "update check failed");
            Ok(false)
        }
    }
}

/// Check for updates, download them, and show a notification to the user.
pub fn check_and_download_notify() {
    match check_and_download() {
        Ok(true) => {
            crate::notifications::show(
                "Hypnos Audio",
                "发现新版本，已下载，将在下次启动时更新",
                None,
            );
        }
        Ok(false) => {
            crate::notifications::show("Hypnos Audio", "当前已是最新版本", None);
        }
        Err(e) => {
            tracing::warn!(error = %e, "update check failed");
            crate::notifications::show("Hypnos Audio", "更新检查失败，请稍后再试", None);
        }
    }
}
