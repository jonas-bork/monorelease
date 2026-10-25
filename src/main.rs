use clap::Parser;
use config::{Config, ConfigError, Environment, File};
use monorelease::{
    config::Config as MonoReleaseConfig,
    logging::{Log, LogConfig},
};
use tracing::error;

use crate::cli::Cli;
use anyhow::Result;

pub mod cli;

/// Loads the configuration.
pub fn load_config() -> Result<MonoReleaseConfig, ConfigError> {
    let builder = Config::builder()
        .add_source(File::with_name("monorelease.yaml").required(true))
        .add_source(Environment::with_prefix("MONORELEASE").separator("__"));
    builder.build()?.try_deserialize()
}

fn main() {
    let result = run();

    if let Err(err) = result {
        error!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let log = Log::init(&LogConfig::default())?;

    let args = cli::Args::parse();
    Cli::run(args, &log)
}
