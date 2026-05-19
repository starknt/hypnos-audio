mod audio;
mod bluetooth;
mod state;
mod tray;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    tracing::info!("hypnos-audio starting...");

    let audio = Arc::new(audio::AudioController::new()?);
    let state = Arc::new(state::StateManager::new());

    let (bt_watcher, bt_rx) = bluetooth::BluetoothWatcher::new()?;
    bt_watcher.start()?;

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

    let state_clone = Arc::clone(&state);
    let audio_clone = Arc::clone(&audio);
    let bt_handle = tokio::spawn(async move {
        bluetooth::run_event_loop(bt_rx, state_clone, audio_clone).await;
    });

    // Run tray on a dedicated thread because winit requires its own event loop
    let tray_shutdown_tx = shutdown_tx.clone();
    let tray_handle = std::thread::spawn(move || {
        if let Err(e) = tray::run_tray(tray_shutdown_tx) {
            tracing::error!(error = %e, "tray error");
        }
    });

    // Wait for shutdown signal
    shutdown_rx.recv().await;
    tracing::info!("shutting down...");

    bt_watcher.stop()?;
    bt_handle.abort();

    // Give tray thread a moment to exit gracefully
    let _ = tray_handle.join();

    Ok(())
}
