use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::logging::LogConfig;

pub const CONFIG_PATH: &str = "monorelease.yaml";
pub const NEXT_DIR_PATH: &str = "next/";
pub const WAVE_DIR_PATH: &str = "waves/";
pub const PROJECTS_DIR_PATH: &str = "projects/";

pub const DEFAULT_BASE_PATH: &str = "releases/";

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Config {
    #[allow(clippy::struct_field_names, unused)]
    pub config_version: u16,
    pub base_path: PathBuf,
    #[serde(default, skip_serializing_if = "is_default")]
    pub log: LogConfig,
}

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    *t == T::default()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_version: 1,
            base_path: DEFAULT_BASE_PATH.into(),
            log: LogConfig::default(),
        }
    }
}

impl Config {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let file = File::open(path).context(format!("could not find file '{}'", path.display()))?;
        let reader = BufReader::new(file);
        Ok(serde_yaml::from_reader(reader)?)
    }

    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        Self::load_from_file(&dir.join(CONFIG_PATH))
    }

    pub fn ensure_file_exists_in_dir(dir: &Path) -> Result<()> {
        let config_path = dir.join(CONFIG_PATH);
        if config_path.exists() {
            info!("existing config file detected at {}", config_path.display());
        } else {
            info!(
                "config file does not exist - creating default at {}",
                config_path.display()
            );
            let config = Self::default();
            let file = File::create(config_path)?;
            serde_yaml::to_writer(file, &config)?;
        }
        Ok(())
    }
}
