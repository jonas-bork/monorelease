use std::path::{Path, PathBuf};

use crate::{
    MonoRelease,
    config::{NEXT_DIR_PATH, WAVES_DIR_PATH},
};
use anyhow::{Context, Result};
use tracing::warn;

pub type Wave = u32;

impl MonoRelease {
    pub fn create_wave(&self) -> Result<()> {
        if self.next_changes_dir_is_empty()? {
            warn!("creating a wave with no changes");
        }

        let current_wave = self.find_current_wave()?;
        let next_wave = calculate_next_wave(current_wave)?;
        let next_wave_dir = self.create_wave_dir(next_wave)?;

        move_changes(&self.next_changes_dir(), &next_wave_dir)
    }

    #[must_use]
    pub fn waves_dir(&self) -> PathBuf {
        self.releases_dir.join(WAVES_DIR_PATH)
    }

    pub fn find_current_wave(&self) -> Result<Wave> {
        let waves_dir = self.waves_dir();
        let entries = std::fs::read_dir(&waves_dir)
            .with_context(|| format!("failed to read waves directory '{}'", waves_dir.display()))?;
        let mut max_wave: Option<Wave> = None;
        for entry in entries {
            let entry = entry.with_context(|| {
                format!(
                    "failed to read entry in waves directory '{}'",
                    waves_dir.display()
                )
            })?;
            let file_name = entry.file_name();
            if let Some(wave) = file_name.to_str().and_then(|s| s.parse::<Wave>().ok()) {
                max_wave = Some(max_wave.map_or(wave, |m: Wave| m.max(wave)));
            }
        }
        Ok(max_wave.unwrap_or(0))
    }

    fn create_wave_dir(&self, wave: Wave) -> Result<PathBuf> {
        let wave_dir = self.waves_dir().join(wave.to_string());
        std::fs::create_dir_all(&wave_dir)
            .with_context(|| format!("failed to create wave directory '{}'", wave_dir.display()))?;
        Ok(wave_dir)
    }

    fn next_changes_dir(&self) -> PathBuf {
        self.releases_dir.join(NEXT_DIR_PATH)
    }

    pub fn next_changes_dir_is_empty(&self) -> Result<bool> {
        let next_dir = self.next_changes_dir();
        Ok(std::fs::read_dir(&next_dir)?.next().is_none())
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
    let entries = std::fs::read_dir(from)
        .with_context(|| format!("failed to read changes directory '{}'", from.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| {
            format!(
                "failed to read entry in changes directory '{}'",
                from.display()
            )
        })?;
        let source = entry.path();
        let dest = to.join(entry.file_name());
        std::fs::rename(&source, &dest).with_context(|| {
            format!(
                "failed to move '{}' to '{}'",
                source.display(),
                dest.display()
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        changes::ProjectChange,
        testing::{default_monorelease_env, my_test},
    };
    use anyhow::Result;
    use std::fs;

    // --- calculate_next_wave ---

    #[test]
    fn test_calculate_next_wave_increments() {
        assert_eq!(calculate_next_wave(0).unwrap(), 1);
        assert_eq!(calculate_next_wave(1).unwrap(), 2);
        assert_eq!(calculate_next_wave(99).unwrap(), 100);
    }

    #[test]
    fn test_calculate_next_wave_overflow() {
        let err = calculate_next_wave(Wave::MAX).unwrap_err();
        assert!(err.to_string().contains(&Wave::MAX.to_string()));
    }

    // --- create_wave ---

    my_test! {
        fn test_create_wave_creates_first_wave_dir() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            monorelease.init()?;

            monorelease.create_wave()?;

            assert!(monorelease.waves_dir().join("1").exists());
            Ok(())
        }
    }

    my_test! {
        fn test_create_wave_moves_changes_to_wave_dir() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            monorelease.init()?;

            let projects = vec![ProjectChange {
                project_name: "frontend".to_string(),
                version_bump: "minor".to_string(),
            }];
            monorelease.add_change("my-change", "A change", &projects)?;

            monorelease.create_wave()?;

            assert!(monorelease.waves_dir().join("1").join("my-change.md").exists());
            Ok(())
        }
    }

    my_test! {
        fn test_create_wave_clears_next_dir() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            monorelease.init()?;

            let projects = vec![ProjectChange {
                project_name: "frontend".to_string(),
                version_bump: "minor".to_string(),
            }];
            monorelease.add_change("my-change", "A change", &projects)?;

            monorelease.create_wave()?;

            let next_dir = monorelease.next_changes_dir();
            assert_eq!(fs::read_dir(&next_dir)?.count(), 0);
            Ok(())
        }
    }

    my_test! {
        fn test_create_wave_increments_wave_number() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            monorelease.init()?;

            monorelease.create_wave()?;
            monorelease.create_wave()?;

            assert!(monorelease.waves_dir().join("1").exists());
            assert!(monorelease.waves_dir().join("2").exists());
            Ok(())
        }
    }

    my_test! {
        fn test_create_wave_with_no_changes() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            monorelease.init()?;

            monorelease.create_wave()?;

            let wave_1_dir = monorelease.waves_dir().join("1");
            assert!(wave_1_dir.exists());
            assert_eq!(fs::read_dir(&wave_1_dir)?.count(), 0);
            Ok(())
        }
    }
}
