use std::sync::Mutex;

#[derive(Debug, Clone, Copy, Default)]
pub struct AudioState {
    pub was_muted: bool,
    pub volume: f32,
}

pub struct StateManager {
    saved: Mutex<Option<AudioState>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            saved: Mutex::new(None),
        }
    }

    pub fn save(&self, state: AudioState) {
        let mut saved = self.saved.lock().unwrap();
        if saved.is_none() {
            *saved = Some(state);
            tracing::info!(?state, "saved audio state");
        }
    }

    /// Atomically take the saved state if it exists, returning None otherwise.
    pub fn take_if_saved(&self) -> Option<AudioState> {
        self.saved.lock().unwrap().take()
    }
}
