use crate::Result;
use crate::state::AudioState;
use std::sync::mpsc::{self, Sender};
use windows::Win32::Media::Audio::{
    Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PeekMessageW, TranslateMessage, PM_REMOVE,
};

#[allow(dead_code)]
enum Command {
    GetMute(Sender<Result<bool>>),
    SetMute(bool, Sender<Result<()>>),
    GetVolume(Sender<Result<f32>>),
    SetVolume(f32, Sender<Result<()>>),
    GetState(Sender<Result<AudioState>>),
    RestoreState(AudioState, Sender<Result<()>>),
    GetDeviceState(String, Sender<Result<AudioState>>),
    RestoreDeviceState(String, AudioState, Sender<Result<()>>),
    Shutdown,
}

pub struct AudioController {
    tx: Sender<Command>,
}

impl AudioController {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::channel::<Command>();

        std::thread::spawn(move || {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
            }

            loop {
                match rx.try_recv() {
                    Ok(Command::Shutdown) => break,
                    Ok(cmd) => Self::dispatch(cmd),
                    Err(mpsc::TryRecvError::Empty) => {
                        unsafe {
                            let mut msg = MSG::default();
                            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                                let _ = TranslateMessage(&msg);
                                DispatchMessageW(&msg);
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(mpsc::TryRecvError::Disconnected) => break,
                }
            }
        });

        Ok(Self { tx })
    }

    fn dispatch(cmd: Command) {
        match cmd {
            Command::GetMute(reply) => {
                let _ = reply.send(Self::exec_get_mute());
            }
            Command::SetMute(muted, reply) => {
                let _ = reply.send(Self::exec_set_mute(muted));
            }
            Command::GetVolume(reply) => {
                let _ = reply.send(Self::exec_get_volume());
            }
            Command::SetVolume(level, reply) => {
                let _ = reply.send(Self::exec_set_volume(level));
            }
            Command::GetState(reply) => {
                let _ = reply.send(Self::exec_get_state());
            }
            Command::RestoreState(state, reply) => {
                let _ = reply.send(Self::exec_restore_state(state));
            }
            Command::GetDeviceState(device_id, reply) => {
                let _ = reply.send(Self::exec_get_device_state(&device_id));
            }
            Command::RestoreDeviceState(device_id, state, reply) => {
                let _ = reply.send(Self::exec_restore_device_state(&device_id, state));
            }
            Command::Shutdown => {}
        }
    }

    pub fn get_state(&self) -> Result<AudioState> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::GetState(tx))
            .map_err(|e| format!("audio worker disconnected: {e}"))?;
        rx.recv()
            .map_err(|e| format!("audio worker hung up: {e}"))?
    }

    #[allow(dead_code)]
    pub fn restore_state(&self, state: AudioState) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::RestoreState(state, tx))
            .map_err(|e| format!("audio worker disconnected: {e}"))?;
        rx.recv()
            .map_err(|e| format!("audio worker hung up: {e}"))?
    }

    pub fn get_device_state_by_id(&self, device_id: &str) -> Result<AudioState> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::GetDeviceState(device_id.to_string(), tx))
            .map_err(|e| format!("audio worker disconnected: {e}"))?;
        rx.recv()
            .map_err(|e| format!("audio worker hung up: {e}"))?
    }

    pub fn restore_device_state_by_id(&self, device_id: &str, state: AudioState) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::RestoreDeviceState(device_id.to_string(), state, tx))
            .map_err(|e| format!("audio worker disconnected: {e}"))?;
        rx.recv()
            .map_err(|e| format!("audio worker hung up: {e}"))?
    }

    #[allow(dead_code)]
    pub fn is_muted(&self) -> Result<bool> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::GetMute(tx))
            .map_err(|e| format!("audio worker disconnected: {e}"))?;
        rx.recv()
            .map_err(|e| format!("audio worker hung up: {e}"))?
    }

    pub fn set_mute(&self, muted: bool) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::SetMute(muted, tx))
            .map_err(|e| format!("audio worker disconnected: {e}"))?;
        rx.recv()
            .map_err(|e| format!("audio worker hung up: {e}"))?
    }

    #[allow(dead_code)]
    pub fn get_master_volume(&self) -> Result<f32> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::GetVolume(tx))
            .map_err(|e| format!("audio worker disconnected: {e}"))?;
        rx.recv()
            .map_err(|e| format!("audio worker hung up: {e}"))?
    }

    #[allow(dead_code)]
    pub fn set_master_volume(&self, level: f32) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::SetVolume(level, tx))
            .map_err(|e| format!("audio worker disconnected: {e}"))?;
        rx.recv()
            .map_err(|e| format!("audio worker hung up: {e}"))?
    }

    fn exec_get_mute() -> Result<bool> {
        Self::with_volume(|volume| {
            let muted = unsafe { volume.GetMute()? };
            Ok(muted.as_bool())
        })
    }

    fn exec_set_mute(muted: bool) -> Result<()> {
        Self::with_volume(|volume| {
            unsafe { volume.SetMute(muted, std::ptr::null())? };
            tracing::info!(muted, "system audio mute state changed");
            Ok(())
        })
    }

    fn exec_get_volume() -> Result<f32> {
        Self::with_volume(|volume| {
            let level = unsafe { volume.GetMasterVolumeLevelScalar()? };
            Ok(level)
        })
    }

    fn exec_set_volume(level: f32) -> Result<()> {
        Self::with_volume(|volume| {
            unsafe {
                volume.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null())?
            };
            Ok(())
        })
    }

    fn exec_get_state() -> Result<AudioState> {
        let muted = Self::exec_get_mute()?;
        let volume = Self::exec_get_volume()?;
        Ok(AudioState {
            was_muted: muted,
            volume,
        })
    }

    fn exec_restore_state(state: AudioState) -> Result<()> {
        Self::exec_set_mute(state.was_muted)?;
        Self::exec_set_volume(state.volume)?;
        Ok(())
    }

    fn exec_get_device_state(device_id: &str) -> Result<AudioState> {
        Self::with_device_volume(device_id, |volume| {
            let muted = unsafe { volume.GetMute()? };
            let level = unsafe { volume.GetMasterVolumeLevelScalar()? };
            Ok(AudioState {
                was_muted: muted.as_bool(),
                volume: level,
            })
        })
    }

    fn exec_restore_device_state(device_id: &str, state: AudioState) -> Result<()> {
        Self::with_device_volume(device_id, |volume| {
            unsafe { volume.SetMute(state.was_muted, std::ptr::null())? };
            unsafe {
                volume.SetMasterVolumeLevelScalar(state.volume.clamp(0.0, 1.0), std::ptr::null())?
            };
            Ok(())
        })
    }

    fn with_volume<T>(f: impl FnOnce(&IAudioEndpointVolume) -> Result<T>) -> Result<T> {
        let volume = unsafe { Self::endpoint_volume()? };
        f(&volume)
    }

    fn with_device_volume<T>(
        device_id: &str,
        f: impl FnOnce(&IAudioEndpointVolume) -> Result<T>,
    ) -> Result<T> {
        let volume = unsafe { Self::device_endpoint_volume(device_id)? };
        f(&volume)
    }

    unsafe fn endpoint_volume() -> Result<IAudioEndpointVolume> {
        let enumerator: IMMDeviceEnumerator = unsafe {
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("failed to create MMDeviceEnumerator: {e}"))?
        };
        let device = unsafe {
            enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| format!("failed to get default audio endpoint: {e}"))?
        };
        let volume = unsafe {
            device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(|e| format!("failed to activate IAudioEndpointVolume: {e}"))?
        };
        Ok(volume)
    }

    unsafe fn device_endpoint_volume(device_id: &str) -> Result<IAudioEndpointVolume> {
        let enumerator: IMMDeviceEnumerator = unsafe {
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("failed to create MMDeviceEnumerator: {e}"))?
        };
        let device_id_wide: Vec<u16> = device_id.encode_utf16().chain(std::iter::once(0)).collect();
        let device_id_pcwstr = windows::core::PCWSTR::from_raw(device_id_wide.as_ptr());
        let device = unsafe {
            enumerator
                .GetDevice(device_id_pcwstr)
                .map_err(|e| format!("failed to get device by id: {e}"))?
        };
        let volume = unsafe {
            device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(|e| format!("failed to activate IAudioEndpointVolume: {e}"))?
        };
        Ok(volume)
    }
}

impl Drop for AudioController {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
    }
}
