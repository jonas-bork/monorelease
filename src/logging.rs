use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::error;
use tracing_subscriber::{
    EnvFilter, Registry,
    filter::LevelFilter,
    layer::{Layered, SubscriberExt},
    reload,
    util::SubscriberInitExt,
};

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

type FilterLayer = reload::Layer<EnvFilter, Registry>;
type FmtSubscriber = Layered<FilterLayer, Registry>;
type FmtLayer = tracing_subscriber::fmt::Layer<FmtSubscriber>;

pub struct Log {
    filter_reload_handle: reload::Handle<EnvFilter, Registry>,
    fmt_reload_handle: reload::Handle<FmtLayer, FmtSubscriber>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LogConfig {
    level: LogLevel,
    debug_mode: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::default(),
            debug_mode: false,
        }
    }
}

impl Log {
    #[must_use]
    pub fn init(config: &LogConfig) -> Result<Self> {
        let filter = EnvFilter::builder().from_env_lossy();
        let (filter_layer, reload_handle) = reload::Layer::new(filter);

        let fmt_layer = tracing_subscriber::fmt::layer();
        let (reloadable_fmt_layer, fmt_handle) = reload::Layer::new(fmt_layer);

        tracing_subscriber::registry()
            .with(filter_layer)
            .with(reloadable_fmt_layer)
            .init();

        let log = Self {
            filter_reload_handle: reload_handle,
            fmt_reload_handle: fmt_handle,
        };
        log.apply_config(config)?;
        Ok(log)
    }

    pub fn set_log_level(&self, log_level: &LogLevel) -> Result<()> {
        let level_filter: LevelFilter = log_level.into();

        let new_filter = EnvFilter::builder()
            .with_default_directive(level_filter.into())
            .from_env_lossy();

        self.filter_reload_handle
            .modify(|filter| *filter = new_filter)
            .map_err(|err| {
                error!("failed to update log level: {err}");
                anyhow::Error::from(err)
            })
    }

    pub fn apply_config(&self, config: &LogConfig) -> Result<()> {
        // filter
        let level_filter: LevelFilter = (&config.level).into();

        let new_filter = EnvFilter::builder()
            .with_default_directive(level_filter.into())
            .from_env_lossy();

        self.filter_reload_handle
            .modify(|filter| *filter = new_filter)
            .map_err(|err| {
                error!("failed to update log level: {err}");
                anyhow::Error::from(err)
            })?;

        // fmt
        let new_fmt = tracing_subscriber::fmt::layer()
            .with_target(config.debug_mode)
            .with_file(config.debug_mode)
            .with_line_number(config.debug_mode);

        self.fmt_reload_handle
            .modify(|fmt| *fmt = new_fmt)
            .map_err(|err| {
                error!("failed to update format layer: {err}");
                anyhow::Error::from(err)
            })
    }
}

impl From<&LogLevel> for LevelFilter {
    fn from(level: &LogLevel) -> Self {
        match level {
            LogLevel::Trace => Self::TRACE,
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Info => Self::INFO,
            LogLevel::Warn => Self::WARN,
            LogLevel::Error => Self::ERROR,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::EnvFilter;

    pub fn init_test_logging() {
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env_lossy();

        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    }
}
