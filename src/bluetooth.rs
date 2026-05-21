use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use windows::Devices::Enumeration::{DeviceInformation, DeviceInformationUpdate, DeviceWatcher};
use windows::Foundation::TypedEventHandler;
use windows::core::Ref;

const BLUETOOTH_PROTOCOL_GUID: &str = "{E0CBF06C-CD8B-4647-BB8A-263B43F0F974}";
const DISCONNECT_DEBOUNCE_MS: u64 = 500;
const EVENT_CHANNEL_CAPACITY: usize = 8;

pub enum DeviceEvent {
    HeadsetConnected,
    HeadsetDisconnected,
}

pub struct BluetoothWatcher {
    watcher: DeviceWatcher,
}

impl BluetoothWatcher {
    pub fn new() -> Result<(Self, mpsc::Receiver<DeviceEvent>)> {
        let (tx, rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let tx_added = tx.clone();
        let tx_removed = tx;

        let filter = windows::core::HSTRING::from(format!(
            r#"System.Devices.Aep.ProtocolId:="{}""#,
            BLUETOOTH_PROTOCOL_GUID
        ));

        let watcher = DeviceInformation::CreateWatcherAqsFilter(&filter)?;

        watcher.Added(&TypedEventHandler::new(
            move |_sender: Ref<'_, _>, info: Ref<'_, DeviceInformation>| {
                if let Some(info) = info.as_ref() {
                    if let Ok(name) = info.Name() {
                        tracing::info!(%name, "bluetooth device added");
                    }
                    if let Err(e) = tx_added.try_send(DeviceEvent::HeadsetConnected) {
                        tracing::warn!(error = %e, "dropped bluetooth connect event");
                    }
                }
                Ok(())
            },
        ))?;

        watcher.Removed(&TypedEventHandler::new(
            move |_sender: Ref<'_, _>, info: Ref<'_, DeviceInformationUpdate>| {
                if let Some(_info) = info.as_ref()
                    && let Err(e) = tx_removed.try_send(DeviceEvent::HeadsetDisconnected)
                {
                    tracing::warn!(error = %e, "dropped bluetooth disconnect event");
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
                if let Some(saved) = state.take_if_saved() {
                    if let Err(e) = audio.restore_state(saved) {
                        tracing::error!(error = %e, "failed to restore audio state");
                    } else {
                        tracing::info!(
                            was_muted = saved.was_muted,
                            volume = saved.volume,
                            "restored audio state"
                        );
                    }
                }
            }
            DeviceEvent::HeadsetDisconnected => {
                // Abort any previous pending disconnect before starting a new one
                if let Some(handle) = disconnect_debounce.take() {
                    handle.abort();
                    tracing::debug!("aborted previous disconnect debounce");
                }

                let state = Arc::clone(&state);
                let audio = Arc::clone(&audio);
                let handle = tokio::spawn(async move {
                    sleep(Duration::from_millis(DISCONNECT_DEBOUNCE_MS)).await;
                    if let Ok(snapshot) = audio.get_state() {
                        state.save(snapshot);
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
