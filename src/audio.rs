use anyhow::{Context, Result};
use windows::Win32::Media::Audio::{
    eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};

use crate::state::AudioState;

pub struct AudioController;

impl AudioController {
    pub fn new() -> Result<Self> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
        }
        Ok(Self)
    }

    pub fn get_state(&self) -> Result<AudioState> {
        let muted = self.is_muted()?;
        let volume = self.get_master_volume()?;
        Ok(AudioState {
            was_muted: muted,
            volume,
        })
    }

    pub fn restore_state(&self, state: AudioState) -> Result<()> {
        self.set_mute(state.was_muted)?;
        self.set_master_volume(state.volume)?;
        Ok(())
    }

    pub fn is_muted(&self) -> Result<bool> {
        self.with_volume(|volume| {
            let muted = unsafe { volume.GetMute()? };
            Ok(muted.as_bool())
        })
    }

    pub fn set_mute(&self, muted: bool) -> Result<()> {
        self.with_volume(|volume| {
            unsafe { volume.SetMute(muted, std::ptr::null())? };
            tracing::info!(muted, "system audio mute state changed");
            Ok(())
        })
    }

    pub fn get_master_volume(&self) -> Result<f32> {
        self.with_volume(|volume| {
            let level = unsafe { volume.GetMasterVolumeLevelScalar()? };
            Ok(level)
        })
    }

    pub fn set_master_volume(&self, level: f32) -> Result<()> {
        self.with_volume(|volume| {
            unsafe { volume.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null())? };
            Ok(())
        })
    }

    fn with_volume<T>(&self, f: impl FnOnce(&IAudioEndpointVolume) -> Result<T>) -> Result<T> {
        let volume = unsafe { self.endpoint_volume()? };
        f(&volume)
    }

    unsafe fn endpoint_volume(&self) -> Result<IAudioEndpointVolume> {
        let enumerator: IMMDeviceEnumerator = unsafe {
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .context("failed to create MMDeviceEnumerator")?
        };
        let device = unsafe {
            enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .context("failed to get default audio endpoint")?
        };
        let volume = unsafe {
            device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .context("failed to activate IAudioEndpointVolume")?
        };
        Ok(volume)
    }
}
