use std::{
    fs::{self},
    path::Path,
};

use anyhow::Result;
use tracing::{debug, info};

use crate::MonoRelease;

impl MonoRelease {
    /// Initialises the folder structure.
    pub fn init(&self, dir: &Path) -> Result<()> {
        info!("initialising at {}", dir.display());

        let config = &self.config;

        fs::create_dir_all(config.base_path.clone())?;
        let dirs_to_create = [
            crate::config::NEXT_DIR_PATH,
            crate::config::PROJECTS_DIR_PATH,
            crate::config::WAVE_DIR_PATH,
        ];
        for dir_path in dirs_to_create {
            let path = dir.join(&config.base_path).join(dir_path);
            debug!("Creating {}", path.display());
            fs::create_dir_all(path)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::Config, testing::my_test};

    use super::*;
    use anyhow::{Context, Result};
    use tempfile::tempdir;

    my_test! {
        fn test_init_creates_directories() -> Result<()> {
            let temp_dir = tempdir().context("Failed to create temp dir")?;
            let root_path = temp_dir.path().to_path_buf();

            info!("setting up monorelease");
            Config::ensure_file_exists_in_dir(&root_path)?;
            let config = Config::load_from_dir(&root_path)?;
            let monorelease = MonoRelease::new(config);

            info!("initialising monorelease");
            monorelease.init(&root_path)?;

            let base_path = root_path.join(crate::config::DEFAULT_BASE_PATH);

            let paths_to_check_exists = [
                crate::config::NEXT_DIR_PATH,
                crate::config::PROJECTS_DIR_PATH,
                crate::config::WAVE_DIR_PATH,
            ];

            for path in paths_to_check_exists {
                debug!("Checking {}", base_path.join(&path).display());
                assert!(base_path.join(&path).exists());
            }

            Ok(())
        }
    }
}
