use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use windows::core::Ref;
use windows::Devices::Enumeration::{
    DeviceInformation, DeviceInformationUpdate, DeviceWatcher,
};
use windows::Foundation::TypedEventHandler;

pub enum DeviceEvent {
    HeadsetConnected,
    HeadsetDisconnected,
}

pub struct BluetoothWatcher {
    watcher: DeviceWatcher,
}

impl BluetoothWatcher {
    pub fn new() -> Result<(Self, mpsc::Receiver<DeviceEvent>)> {
        let (tx, rx) = mpsc::channel(8);
        let tx_added = tx.clone();
        let tx_removed = tx;

        let filter = windows::core::HSTRING::from(
            "System.Devices.Aep.ProtocolId:=\"{E0CBF06C-CD8B-4647-BB8A-263B43F0F974}\""
        ); // Bluetooth protocol GUID

        let watcher = DeviceInformation::CreateWatcherAqsFilter(&filter)?;

        watcher.Added(&TypedEventHandler::new(
            move |_sender: Ref<'_, _>, info: Ref<'_, DeviceInformation>| {
                if let Some(info) = info.as_ref() {
                    if let Ok(name) = info.Name() {
                        tracing::info!(%name, "bluetooth device added");
                    }
                    let _ = tx_added.try_send(DeviceEvent::HeadsetConnected);
                }
                Ok(())
            },
        ))?;

        watcher.Removed(&TypedEventHandler::new(
            move |_sender: Ref<'_, _>, info: Ref<'_, DeviceInformationUpdate>| {
                if let Some(_info) = info.as_ref() {
                    let _ = tx_removed.try_send(DeviceEvent::HeadsetDisconnected);
                }
                Ok(())
            },
        ))?;

        Ok((Self { watcher }, rx))
    }

    pub fn start(&self) -> Result<()> {
        self.watcher.Start()?;
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.watcher.Stop()?;
        Ok(())
    }
}

pub async fn run_event_loop(
    mut rx: mpsc::Receiver<DeviceEvent>,
    state: Arc<crate::state::StateManager>,
    audio: Arc<crate::audio::AudioController>,
) {
    let mut disconnect_debounce: Option<tokio::task::AbortHandle> = None;

    while let Some(event) = rx.recv().await {
        match event {
            DeviceEvent::HeadsetConnected => {
                // Cancel any pending disconnect action
                if let Some(handle) = disconnect_debounce.take() {
                    handle.abort();
                    tracing::debug!("cancelled pending disconnect due to reconnect");
                }
                if state.is_saved() {
                    if let Some(saved) = state.take() {
                        let _ = audio.set_mute(saved.was_muted);
                        let _ = audio.set_master_volume(saved.volume);
                        tracing::info!(was_muted = saved.was_muted, volume = saved.volume, "restored audio state");
                    }
                }
            }
            DeviceEvent::HeadsetDisconnected => {
                let state = Arc::clone(&state);
                let audio = Arc::clone(&audio);
                let handle = tokio::spawn(async move {
                    sleep(Duration::from_millis(1500)).await;
                    if let Ok(was_muted) = audio.is_muted() {
                        if let Ok(volume) = audio.get_master_volume() {
                            state.save(crate::state::AudioState { was_muted, volume });
                        }
                    }
                    if let Err(e) = audio.set_mute(true) {
                        tracing::error!(error = %e, "failed to mute audio");
                    } else {
                        tracing::info!("system muted after headset disconnect");
                    }
                });
                disconnect_debounce = Some(handle.abort_handle());
            }
        }
    }
}
