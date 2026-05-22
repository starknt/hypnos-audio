use crate::Result;
use velopack::{UpdateCheck, UpdateManager, sources};

pub fn check_and_apply() -> Result<()> {
    let repo = match std::env::var("HYPNOS_GITHUB_REPO") {
        Ok(r) => r,
        Err(_) => {
            tracing::warn!("HYPNOS_GITHUB_REPO not set, skipping update check");
            return Ok(());
        }
    };

    let repo_url = format!("https://github.com/{}", repo);
    let source = sources::GithubSource::new(&repo_url, None, false);
    let um = UpdateManager::new(source, None, None)?;

    match um.check_for_updates() {
        Ok(UpdateCheck::UpdateAvailable(updates)) => {
            tracing::info!(version = %updates.TargetFullRelease.Version, "update available, downloading");
            crate::notifications::show("Hypnos Audio", "发现新版本，正在下载并安装", None);
            um.download_updates(&updates, None)?;
            tracing::info!("update downloaded, restarting to apply");
            um.apply_updates_and_restart(&updates)?;
        }
        Ok(_) => {
            tracing::info!("no updates available");
        }
        Err(e) => {
            crate::notifications::show("Hypnos Audio", "更新检查失败，请稍后再试", None);
            return Err(e.into());
        }
    }

    Ok(())
}
