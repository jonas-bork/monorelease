use std::fs::{self, OpenOptions};
use std::io::Write;

use crate::{MonoRelease, ProjectId, config::NEXT_DIR_PATH};
use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChange {
    pub project_id: ProjectId,
    pub version_bump: String,
}

impl MonoRelease {
    pub fn add_change(&self, name: &str, message: &str, projects: &[ProjectChange]) -> Result<()> {
        let dir_path = self.releases_dir.join(NEXT_DIR_PATH);
        fs::create_dir_all(&dir_path)?;

        let file_path = dir_path.join(format!("{name}.md"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&file_path)
            .with_context(|| format!("Change file {name}.md already exists"))?;

        let frontmatter = project_frontmatter(projects);
        writeln!(file, "{frontmatter}\n{message}")?;
        Ok(())
    }
}

fn project_frontmatter(projects: &[ProjectChange]) -> String {
    let mut frontmatter = "---\n".to_string();
    for project_str in projects
        .iter()
        .map(|project| format!("{}: {}", project.project_id, project.version_bump))
    {
        frontmatter += &project_str;
        frontmatter += "\n";
    }
    frontmatter += "---";
    frontmatter
}

#[cfg(test)]
mod tests {
    use crate::testing::{default_monorelease_env, my_test};

    use super::*;
    use anyhow::Result;
    use std::fs;

    my_test! {
        fn test_add_creates_change_file_with_correct_content() -> Result<()> {
            let monorelease = default_monorelease_env()?;

            let projects = vec![
                ProjectChange {
                    project_id: ProjectId::new("frontend")?,
                    version_bump: "minor".to_string(),
                },
                ProjectChange {
                    project_id: ProjectId::new("backend")?,
                    version_bump: "patch".to_string(),
                },
            ];

            monorelease.add_change("my-feature", "Added new feature functionality", &projects)?;

            let file_path = monorelease.root_dir
                .join(&monorelease.config.releases_dir)
                .join(crate::config::NEXT_DIR_PATH)
                .join("my-feature.md");

            assert!(file_path.exists());

            let content = fs::read_to_string(&file_path)?;
            let expected_content = "---\nfrontend: minor\nbackend: patch\n---\nAdded new feature functionality\n";

            assert_eq!(content, expected_content);

            Ok(())
        }
    }

    my_test! {
        fn test_two_changes_with_same_name() -> Result<()> {
            let monorelease = default_monorelease_env()?;

            let projects = vec![
                ProjectChange {
                    project_id: ProjectId::new("frontend")?,
                    version_bump: "minor".to_string(),
                },
                ProjectChange {
                    project_id: ProjectId::new("backend")?,
                    version_bump: "patch".to_string(),
                },
            ];

            let file_path = monorelease.root_dir
                .join(&monorelease.config.releases_dir)
                .join(crate::config::NEXT_DIR_PATH)
                .join(format!("{NAME}.md"));

            const NAME: &str = "my-feature";
            monorelease.add_change(NAME, "Added new feature functionality", &projects)?;
            assert!(file_path.exists());
            let err = monorelease.add_change(NAME, "Added new feature", &projects).unwrap_err();
            assert_eq!(err.to_string(), format!("Change file {NAME}.md already exists"));


            assert!(file_path.exists());

            // It should have the first ones content because the second one should not have gone through
            let content = fs::read_to_string(&file_path)?;
            let expected_content = "---\nfrontend: minor\nbackend: patch\n---\nAdded new feature functionality\n";

            assert_eq!(content, expected_content);

            Ok(())
        }
    }

    my_test! {
        fn test_project_frontmatter_formatting() -> Result<()> {
            let projects = vec![
                ProjectChange {
                    project_id: ProjectId::new("api_client")?,
                    version_bump: "major".to_string(),
                }
            ];

            let result = project_frontmatter(&projects);
            let expected = "---\napi_client: major\n---";

            assert_eq!(result, expected);

            Ok(())
        }
    }

    my_test! {
        fn test_project_frontmatter_empty() -> Result<()> {
            let projects: Vec<ProjectChange> = vec![];

            let result = project_frontmatter(&projects);
            let expected = "---\n---";

            assert_eq!(result, expected);

            Ok(())
        }
    }
}
