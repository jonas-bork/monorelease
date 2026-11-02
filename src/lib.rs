use std::path::PathBuf;

use crate::config::Config;

pub mod changes;
pub mod config;
pub mod init;
pub mod logging;
#[cfg(test)]
pub(crate) mod testing;
pub mod waves;

#[derive(Debug, Clone)]
pub struct MonoRelease {
    root_dir: PathBuf,
    releases_dir: PathBuf,
    config: Config,
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

impl MonoRelease {
    #[must_use]
    pub fn new(root_dir: PathBuf, config: Config) -> Self {
        let releases_dir = root_dir.join(config.releases_dir.clone());
        Self {
            root_dir,
            releases_dir,
            config,
        }
    }

    #[must_use]
    pub fn version() -> String {
        VERSION.to_string()
    }
}
