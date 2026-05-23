use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, Default)]
pub struct AudioState {
    pub was_muted: bool,
    pub volume: f32,
}

pub struct StateManager {
    saved: Mutex<HashMap<String, AudioState>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            saved: Mutex::new(HashMap::new()),
        }
    }

    pub fn save_for_device(&self, device_id: String, state: AudioState) {
        let mut saved = self.saved.lock().unwrap();
        saved.insert(device_id.clone(), state);
        tracing::info!(device_id, ?state, "saved audio state for device");
    }

    /// Take the saved state for a specific device, returning None if not found.
    pub fn take_for_device(&self, device_id: &str) -> Option<AudioState> {
        self.saved.lock().unwrap().remove(device_id)
    }
}
