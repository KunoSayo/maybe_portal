use kira::manager::{AudioManager, AudioManagerSettings};
use kira::manager::backend::cpal::CpalBackend;

pub struct AudioData {
    pub manager: AudioManager<CpalBackend>,
}


impl AudioData {
    pub fn new() -> anyhow::Result<AudioData> {
        Ok(Self {
            manager: AudioManager::new(AudioManagerSettings::default())?
        })
    }
}


impl AudioData {}