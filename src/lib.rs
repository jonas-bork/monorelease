use std::path::PathBuf;

use crate::config::Config;

pub mod changes;
pub mod config;
pub mod init;
pub mod logging;
#[cfg(test)]
pub(crate) mod testing;

#[derive(Debug, Clone)]
pub struct MonoRelease {
    root_path: PathBuf,
    config: Config,
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

impl MonoRelease {
    #[must_use]
    pub const fn new(root_path: PathBuf, config: Config) -> Self {
        Self { root_path, config }
    }

    #[must_use]
    pub fn version() -> String {
        VERSION.to_string()
    }
}
