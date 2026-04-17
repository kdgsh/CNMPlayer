use crate::tmplayer::app::state::PlayMode;
use crate::tmplayer::playback::local_player::LocalPlayer;

pub struct ModeManager {
    pub local: LocalPlayer,
}

impl ModeManager {
    pub fn new() -> Self {
        Self {
            local: LocalPlayer::new(),
        }
    }

    pub fn pause_other(&mut self, target: PlayMode) {
        match target {
            PlayMode::LocalPlayback => (),
            PlayMode::Idle => {
                let _ = self.local.pause();
            }
        }
    }
}
