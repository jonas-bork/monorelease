use std::fs::{self, File};
use std::io::Write;

use crate::{MonoRelease, config::NEXT_DIR_PATH};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChange {
    pub project_name: String,
    pub version_bump: String,
}

impl MonoRelease {
    pub fn add(&self, name: &str, message: &str, projects: &[ProjectChange]) -> Result<()> {
        let dir_path = self
            .root_path
            .join(&self.config.base_path)
            .join(NEXT_DIR_PATH);
        fs::create_dir_all(&dir_path)?;

        let file_path = dir_path.join(format!("{name}.md"));
        let mut file = File::create(file_path)?;

        let frontmatter = project_frontmatter(projects);
        writeln!(file, "{frontmatter}\n{message}")?;
        Ok(())
    }
}

fn project_frontmatter(projects: &[ProjectChange]) -> String {
    let mut frontmatter = "---\n".to_string();
    for project_str in projects
        .into_iter()
        .map(|project| format!("{}: {}", project.project_name, project.version_bump))
    {
        frontmatter += &project_str;
        frontmatter += "\n";
    }
    frontmatter += "---";
    frontmatter
}
