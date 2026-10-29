use crate::config::Config;

pub mod changes;
pub mod config;
pub mod init;
pub mod logging;
#[cfg(test)]
pub(crate) mod testing;

#[derive(Debug, Clone)]
pub struct MonoRelease {
    config: Config,
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

impl MonoRelease {
    #[must_use]
    pub const fn new(config: Config) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn version() -> String {
        VERSION.to_string()
    }
}
