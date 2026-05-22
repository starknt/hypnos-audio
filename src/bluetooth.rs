use crate::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use windows::Win32::Media::Audio::{
    DEVICE_STATE, DEVICE_STATE_ACTIVE, DEVICE_STATE_NOTPRESENT, DEVICE_STATE_UNPLUGGED, EDataFlow,
    ERole, EndpointFormFactor, Headphones, Headset, IMMDeviceEnumerator, IMMNotificationClient,
    MMDeviceEnumerator, PKEY_AudioEndpoint_FormFactor,
};
use windows::Win32::System::Com::STGM_READ;
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};

const DISCONNECT_DEBOUNCE_MS: u64 = 500;
const EVENT_CHANNEL_CAPACITY: usize = 8;

pub enum DeviceEvent {
    HeadsetConnected,
    HeadsetDisconnected,
}

/// Check whether the given audio endpoint is a headphone or headset.
unsafe fn is_headphone_device(device_id: &windows::core::PCWSTR) -> Result<bool> {
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
    let device = unsafe { enumerator.GetDevice(*device_id)? };
    let store = unsafe { device.OpenPropertyStore(STGM_READ)? };
    let value = unsafe { store.GetValue(&PKEY_AudioEndpoint_FormFactor)? };

    if value.vt() == windows::Win32::System::Variant::VT_UI4 {
        let ulval = unsafe { value.Anonymous.Anonymous.Anonymous.ulVal };
        let form_factor = EndpointFormFactor(ulval as i32);
        Ok(form_factor == Headphones || form_factor == Headset)
    } else {
        Ok(false)
    }
}

#[windows::core::implement(IMMNotificationClient)]
struct AudioDeviceNotificationClient {
    tx: mpsc::Sender<DeviceEvent>,
}

impl windows::Win32::Media::Audio::IMMNotificationClient_Impl
    for AudioDeviceNotificationClient_Impl
{
    fn OnDeviceStateChanged(
        &self,
        device_id: &windows::core::PCWSTR,
        new_state: DEVICE_STATE,
    ) -> windows::core::Result<()> {
        let is_connect = new_state == DEVICE_STATE_ACTIVE;
        let is_disconnect =
            new_state == DEVICE_STATE_NOTPRESENT || new_state == DEVICE_STATE_UNPLUGGED;

        if !is_connect && !is_disconnect {
            return Ok(());
        }

        match unsafe { is_headphone_device(device_id) } {
            Ok(true) => {
                let event = if is_connect {
                    DeviceEvent::HeadsetConnected
                } else {
                    DeviceEvent::HeadsetDisconnected
                };
                if let Err(e) = self.tx.try_send(event) {
                    tracing::warn!(error = %e, "dropped audio device event");
                }
            }
            Ok(false) => {}
            Err(e) => {
                tracing::debug!(error = %e, "failed to check device form factor");
            }
        }

        Ok(())
    }

    fn OnDeviceAdded(&self, _device_id: &windows::core::PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDeviceRemoved(&self, _device_id: &windows::core::PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        _flow: EDataFlow,
        _role: ERole,
        _device_id: &windows::core::PCWSTR,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _device_id: &windows::core::PCWSTR,
        _key: &windows::Win32::Foundation::PROPERTYKEY,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

pub struct BluetoothWatcher {
    shutdown_tx: std::sync::mpsc::Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl BluetoothWatcher {
    pub fn new() -> Result<(Self, mpsc::Receiver<DeviceEvent>)> {
        let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();

        let thread = std::thread::spawn(move || {
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

            let Ok(enumerator) = (unsafe {
                CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
            }) else {
                tracing::error!("failed to create MMDeviceEnumerator");
                return;
            };

            let client = AudioDeviceNotificationClient { tx: event_tx };
            let client_interface: IMMNotificationClient = client.into();

            if let Err(e) =
                unsafe { enumerator.RegisterEndpointNotificationCallback(&client_interface) }
            {
                tracing::error!(error = %e, "failed to register endpoint notification callback");
                return;
            }

            tracing::info!("audio endpoint watcher started");

            // Block until shutdown
            let _ = shutdown_rx.recv();

            if let Err(e) =
                unsafe { enumerator.UnregisterEndpointNotificationCallback(&client_interface) }
            {
                tracing::warn!(error = %e, "failed to unregister endpoint notification callback");
            }

            tracing::info!("audio endpoint watcher stopped");
        });

        Ok((
            Self {
                shutdown_tx,
                thread: Some(thread),
            },
            event_rx,
        ))
    }

    pub fn start(&self) -> Result<()> {
        // Watcher thread is already running and registered; nothing to do.
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let _ = self.shutdown_tx.send(());
        Ok(())
    }
}

impl Drop for BluetoothWatcher {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub async fn run_event_loop(
    mut rx: mpsc::Receiver<DeviceEvent>,
    state: Arc<crate::state::StateManager>,
    audio: Arc<crate::audio::AudioController>,
) {
    let mut disconnect_debounce: Option<tokio::task::AbortHandle> = None;
    let event_gen = Arc::new(AtomicU64::new(0));

    while let Some(event) = rx.recv().await {
        let generation = event_gen.fetch_add(1, Ordering::SeqCst);

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
                        crate::notifications::show(
                            "Hypnos Audio",
                            "耳机已连接，音量已恢复",
                            Some("headset-connected"),
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

                // 立即保存当前状态（默认设备此时仍是耳机）
                if let Ok(snapshot) = audio.get_state() {
                    state.save(snapshot);
                }

                let audio = Arc::clone(&audio);
                let event_gen = Arc::clone(&event_gen);
                let handle = tokio::spawn(async move {
                    sleep(Duration::from_millis(DISCONNECT_DEBOUNCE_MS)).await;

                    // 如果已有新事件使本次 debounce 失效，直接放弃
                    if event_gen.load(Ordering::SeqCst) != generation {
                        tracing::debug!("disconnect debounce stale, skipping mute");
                        return;
                    }

                    if let Err(e) = audio.set_mute(true) {
                        tracing::error!(error = %e, "failed to mute audio");
                    } else {
                        tracing::info!("system muted after headset disconnect");
                        crate::notifications::show(
                            "Hypnos Audio",
                            "耳机已断开，系统已静音",
                            Some("headset-disconnected"),
                        );
                    }
                });
                disconnect_debounce = Some(handle.abort_handle());
            }
        }
    }
}
