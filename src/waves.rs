use std::path::{Path, PathBuf};

use crate::{MonoRelease, config::WAVES_DIR_PATH};
use anyhow::{Context, Result};

type Wave = u32;

impl MonoRelease {
    pub fn create_wave(&self) -> Result<()> {
        let current_wave = self.find_current_wave();
        let next_wave = calculate_next_wave(current_wave)?;
        let next_wave_dir = self.create_wave_folder(next_wave)?;
        move_changes(&(self.next_changes_dir()), &next_wave_dir)
    }

    #[must_use]
    pub fn waves_dir(&self) -> PathBuf {
        self.releases_dir.join(WAVES_DIR_PATH)
    }

    fn find_current_wave(&self) -> Wave {
        todo!()
    }

    fn create_wave_folder(&self, wave: Wave) -> Result<PathBuf> {
        todo!()
    }

    fn next_changes_dir(&self) -> PathBuf {
        todo!()
    }
}

fn calculate_next_wave(wave: Wave) -> Result<Wave> {
    wave.checked_add(1).with_context(|| {
        format!(
            "next wave number exceeds {} which is not supported",
            Wave::MAX
        )
    })
}

fn move_changes(from: &Path, to: &Path) -> Result<()> {
    todo!()
}
