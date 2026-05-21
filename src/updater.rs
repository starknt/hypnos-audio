use anyhow::Result;
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

    if let UpdateCheck::UpdateAvailable(updates) = um.check_for_updates()? {
        tracing::info!(version = %updates.TargetFullRelease.Version, "update available, downloading");
        um.download_updates(&updates, None)?;
        tracing::info!("update downloaded, restarting to apply");
        um.apply_updates_and_restart(&updates)?;
    } else {
        tracing::info!("no updates available");
    }

    Ok(())
}
