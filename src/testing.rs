macro_rules! my_test {
    (fn $name:ident() $(-> $ret:ty)? $body:block) => {
        #[test]
        #[tracing::instrument(err(Debug))]
        fn $name() $(-> $ret)? {
            $crate::logging::tests::init_test_logging();
            $body
        }
    };
}

use anyhow::Context;
pub(crate) use my_test;
use tempfile::tempdir;
use tracing::info;

use crate::{MonoRelease, config::Config};

pub fn default_monorelease_env() -> anyhow::Result<MonoRelease> {
    let temp_dir = tempdir().context("Failed to create temp dir")?;
    let root_path = temp_dir.path().to_path_buf();

    info!("setting up monorelease");
    Config::ensure_file_exists_in_dir(&root_path)?;
    let config = Config::load_from_dir(&root_path)?;
    Ok(MonoRelease::new(root_path, config))
}
