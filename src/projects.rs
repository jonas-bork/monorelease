use anyhow::{Context, Result, bail};
use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{MonoRelease, waves::Wave};

pub type ProjectName = String;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    pub name: ProjectName,
    pub dependencies: HashMap<ProjectName, DependencyConfig>,
    pub releases: HashMap<String, Wave>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct DependencyConfig {
    pub magnitude: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub config_path: PathBuf,
    pub config: ProjectConfig,
}

impl Project {
    /// Returns the wave of the current release of the project.
    /// Returns 0 if there are no releases.
    fn current_release_wave(&self) -> Wave {
        todo!()
    }

    /// Saves the current configuration to the config path
    fn save_config(&self) -> Result<()> {
        todo!()
    }
}

impl MonoRelease {
    pub fn bump_project(&self, project_id: String) -> Result<()> {
        if self.next_changes_dir_is_empty()? {
            bail!(
                "bumping project when changes are pending. If MonoRelease is used correctly, then these change files represent changes already applied in the files. Therefore, bumping the release inside MonoRelease without these pending change files in a wave, the actual release will include the changes, but the changelog and state of MonoRelease will not"
            );
        }

        let mut project = self.get_project_from_config_file(project_id)?;
        let last_release_wave = project.current_release_wave();
        let current_release_wave = self.find_current_wave()?;

        if last_release_wave == current_release_wave {
            bail!("no new change files since current project release");
        } else if last_release_wave > current_release_wave {
            bail!(
                "current project release is later than the currently last wave. This should not be possible through regular use of MonoRelease - something is wrong with the project's configuration or the waves configuration"
            )
        }

        let project_versions =
            self.next_project_versions(last_release_wave, current_release_wave)?;

        let version = project_versions
            .get(&project.id)
            .context("no change files targeting the project")?;

        project
            .config
            .releases
            .insert(version.clone(), current_release_wave)
            .context("version already exists for project")?;

        project.save_config()?;

        Ok(())
    }

    fn get_project_from_config_file(&self, project_id: String) -> Result<Project> {
        // Hint: find the relevant project file from the dir with path config::PROJECTS_DIR_PATH
        //       It should then read the contents of the project's config file into a `ProjectConfig` struct.
        //       This `ProjectConfig` struct should then be placed in a `Project` struct.
        // Note: The ID of the `Project` struct is the name of the file (without the .yml or .yaml at the end).
        todo!()
    }

    /// Walks through the waves from `last_release_wave` to `current_release_wave`, and counts up the version bumps from the YAML frontmatter from the change files for each project that it encounters.
    /// Each project is only bumped once, meaning if multiple change files bump the same project, only one number will ever be bumped.
    fn next_project_versions(
        &self,
        last_release_wave: u32,
        current_release_wave: u32,
    ) -> Result<HashMap<ProjectName, String>> {
        todo!()
    }
}
