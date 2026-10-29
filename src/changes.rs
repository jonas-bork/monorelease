use crate::MonoRelease;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChange {
    pub project_name: String,
    pub version_bump: String,
}

impl MonoRelease {
    pub fn add(&self, name: &str, message: &str, projects: &[ProjectChange]) -> Result<()> {
        todo!()
    }
}
