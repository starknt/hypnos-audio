use anyhow::{Context, Result};
use windows::Win32::Media::Audio::{
    eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};

pub struct AudioController;

impl AudioController {
    pub fn new() -> Result<Self> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
        }
        Ok(Self)
    }

    pub fn is_muted(&self) -> Result<bool> {
        unsafe {
            let volume = self.endpoint_volume()?;
            let muted = volume.GetMute()?;
            Ok(muted.as_bool())
        }
    }

    pub fn set_mute(&self, muted: bool) -> Result<()> {
        unsafe {
            let volume = self.endpoint_volume()?;
            volume.SetMute(muted, std::ptr::null())?;
            tracing::info!(muted, "system audio mute state changed");
            Ok(())
        }
    }

    pub fn get_master_volume(&self) -> Result<f32> {
        unsafe {
            let volume = self.endpoint_volume()?;
            let level = volume.GetMasterVolumeLevelScalar()?;
            Ok(level)
        }
    }

    pub fn set_master_volume(&self, level: f32) -> Result<()> {
        unsafe {
            let volume = self.endpoint_volume()?;
            volume.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null())?;
            Ok(())
        }
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
