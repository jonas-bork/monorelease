use anyhow::{Context, Result, bail};
use std::{
    borrow::Borrow,
    collections::{HashMap, hash_map::Entry},
    fmt,
    path::PathBuf,
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{MonoRelease, config::PROJECTS_DIR_PATH, waves::Wave};

pub type ProjectName = String;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    /// Creates a new project ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the ID is empty or contains characters outside
    /// `[A-Za-z0-9_-]`.
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();

        if id.is_empty() {
            bail!("project ID must not be empty");
        }

        if !id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        }) {
            bail!("invalid project ID '{id}': expected only ASCII letters, digits, '_', and '-'");
        }

        Ok(Self(id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for ProjectId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ProjectId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for ProjectId {
    type Err = anyhow::Error;

    fn from_str(id: &str) -> Result<Self> {
        Self::new(id)
    }
}

impl<'de> Deserialize<'de> for ProjectId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        Self::new(id).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum VersionBump {
    Patch,
    Minor,
    Major,
}

impl VersionBump {
    fn from_magnitude(magnitude: &str) -> Result<Self> {
        match magnitude {
            "patch" => Ok(Self::Patch),
            "minor" => Ok(Self::Minor),
            "major" => Ok(Self::Major),
            other => bail!(
                "unknown version bump '{other}'; expected one of 'patch', 'minor', or 'major'"
            ),
        }
    }

    fn apply(self, version: &str) -> Result<String> {
        let mut components = version.split('.');
        let major: u64 = components
            .next()
            .context("version is missing a major component")?
            .parse()
            .context("invalid major version component")?;
        let minor: u64 = components
            .next()
            .context("version is missing a minor component")?
            .parse()
            .context("invalid minor version component")?;
        let patch: u64 = components
            .next()
            .context("version is missing a patch component")?
            .parse()
            .context("invalid patch version component")?;
        if components.next().is_some() {
            bail!("invalid version '{version}': expected 'major.minor.patch'");
        }

        let next = match self {
            Self::Major => format!("{}.0.0", increment(major, "major")?),
            Self::Minor => format!("{}.{}.0", major, increment(minor, "minor")?),
            Self::Patch => format!("{}.{}.{}", major, minor, increment(patch, "patch")?),
        };
        Ok(next)
    }
}

fn increment(component: u64, name: &str) -> Result<u64> {
    component
        .checked_add(1)
        .with_context(|| format!("{name} version component overflowed"))
}

fn parse_change_frontmatter(contents: &str) -> Result<HashMap<ProjectName, String>> {
    let after_opening = contents
        .strip_prefix("---")
        .context("change file is missing its YAML frontmatter")?;
    let (frontmatter, _) = after_opening
        .split_once("\n---")
        .context("change file frontmatter is not terminated with '---'")?;
    serde_yaml::from_str(frontmatter).context("failed to parse YAML frontmatter")
}

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
        self.config
            .releases
            .values()
            .copied()
            .max()
            .unwrap_or_default()
    }

    /// Returns the version of the current release of the project.
    /// Returns `None` if there are no releases.
    fn current_version(&self) -> Option<&str> {
        self.config
            .releases
            .iter()
            .max_by(|left, right| left.1.cmp(right.1))
            .map(|(version, _)| version.as_str())
    }

    /// Saves the current configuration to the config path
    fn save_config(&self) -> Result<()> {
        let contents = serde_yaml::to_string(&self.config).with_context(|| {
            format!(
                "failed to serialize project config '{}'",
                self.config_path.display()
            )
        })?;
        std::fs::write(&self.config_path, contents).with_context(|| {
            format!(
                "failed to write project config '{}'",
                self.config_path.display()
            )
        })?;
        Ok(())
    }
}

impl MonoRelease {
    pub fn bump_project(&self, project_id: String) -> Result<()> {
        if !self.next_changes_dir_is_empty()? {
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

        if project
            .config
            .releases
            .insert(version.clone(), current_release_wave)
            .is_some()
        {
            bail!("version already exists for project");
        }

        project.save_config()?;

        Ok(())
    }

    fn get_project_from_config_file(&self, project_id: String) -> Result<Project> {
        let projects_dir = self.releases_dir.join(PROJECTS_DIR_PATH);
        let yaml_path = projects_dir.join(format!("{project_id}.yaml"));
        let yml_path = projects_dir.join(format!("{project_id}.yml"));

        let config_path = match (yaml_path.is_file(), yml_path.is_file()) {
            (true, true) => bail!(
                "found both '{}' and '{}' for project '{project_id}'; remove one so MonoRelease knows which config to read",
                yaml_path.display(),
                yml_path.display()
            ),
            (true, false) => yaml_path,
            (false, true) => yml_path,
            (false, false) => bail!(
                "could not find a config file for project '{project_id}' in '{}'",
                projects_dir.display()
            ),
        };

        let contents = std::fs::read_to_string(&config_path).with_context(|| {
            format!("failed to read project config '{}'", config_path.display())
        })?;
        let config = serde_yaml::from_str(&contents).with_context(|| {
            format!("failed to parse project config '{}'", config_path.display())
        })?;

        Ok(Project {
            id: project_id,
            config_path,
            config,
        })
    }

    /// Walks through the waves from `last_release_wave` to `current_release_wave`, and counts up the version bumps from the YAML frontmatter from the change files for each project that it encounters.
    /// Each project is only bumped once, meaning if multiple change files bump the same project, only one number will ever be bumped.
    fn next_project_versions(
        &self,
        last_release_wave: u32,
        current_release_wave: u32,
    ) -> Result<HashMap<ProjectName, String>> {
        let mut bumps: HashMap<ProjectName, VersionBump> = HashMap::new();
        let first_wave = last_release_wave
            .checked_add(1)
            .context("wave number overflowed")?;

        for wave in first_wave..=current_release_wave {
            let wave_dir = self.waves_dir().join(wave.to_string());

            let entries = std::fs::read_dir(&wave_dir).with_context(|| {
                format!("failed to read wave directory '{}'", wave_dir.display())
            })?;
            for entry in entries {
                let entry = entry.with_context(|| {
                    format!(
                        "failed to read entry in wave directory '{}'",
                        wave_dir.display()
                    )
                })?;
                let path = entry.path();
                if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
                    bail!(
                        "unexpected non-Markdown file '{}' in wave directory '{}'",
                        path.display(),
                        wave_dir.display()
                    );
                }

                let contents = std::fs::read_to_string(&path)
                    .with_context(|| format!("failed to read change file '{}'", path.display()))?;
                let changes = parse_change_frontmatter(&contents)
                    .with_context(|| format!("failed to parse change file '{}'", path.display()))?;

                for (project, magnitude) in changes {
                    let bump = VersionBump::from_magnitude(&magnitude).with_context(|| {
                        format!("invalid version bump in change file '{}'", path.display())
                    })?;
                    match bumps.entry(project) {
                        Entry::Occupied(mut entry) => {
                            if bump > *entry.get() {
                                entry.insert(bump);
                            }
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(bump);
                        }
                    }
                }
            }
        }

        let mut versions = HashMap::new();
        for (project_id, bump) in bumps {
            let project = self.get_project_from_config_file(project_id.clone())?;
            let current_version = project.current_version().unwrap_or("0.0.0");
            versions.insert(project_id, bump.apply(current_version)?);
        }

        Ok(versions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::NEXT_DIR_PATH,
        testing::{default_monorelease_env, my_test},
    };
    use std::fs;

    // --- helpers ---

    fn projects_dir(monorelease: &MonoRelease) -> PathBuf {
        monorelease.releases_dir.join(PROJECTS_DIR_PATH)
    }

    fn project_yaml(name: &str, releases: &[(&str, Wave)]) -> String {
        if releases.is_empty() {
            return format!("name: {name}\ndependencies: {{}}\nreleases: {{}}\n");
        }
        let mut yaml = format!("name: {name}\ndependencies: {{}}\nreleases:\n");
        for (version, wave) in releases {
            yaml.push_str(&format!("  \"{version}\": {wave}\n"));
        }
        yaml
    }

    fn write_config(monorelease: &MonoRelease, project_id: &str, extension: &str, contents: &str) {
        let dir = projects_dir(monorelease);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{project_id}.{extension}")), contents).unwrap();
    }

    fn write_project(monorelease: &MonoRelease, project_id: &str, releases: &[(&str, Wave)]) {
        write_config(
            monorelease,
            project_id,
            "yaml",
            &project_yaml(project_id, releases),
        );
    }

    fn wave_dir(monorelease: &MonoRelease, wave: Wave) -> PathBuf {
        monorelease.waves_dir().join(wave.to_string())
    }

    fn write_wave_entry(monorelease: &MonoRelease, wave: Wave, file_name: &str, contents: &str) {
        let dir = wave_dir(monorelease, wave);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(file_name), contents).unwrap();
    }

    fn write_change_file(
        monorelease: &MonoRelease,
        wave: Wave,
        file_name: &str,
        entries: &[(&str, &str)],
    ) {
        let mut frontmatter = String::new();
        for (project, magnitude) in entries {
            frontmatter.push_str(&format!("{project}: {magnitude}\n"));
        }
        write_wave_entry(
            monorelease,
            wave,
            &format!("{file_name}.md"),
            &format!("---\n{frontmatter}---\nbody"),
        );
    }

    fn setup_env() -> Result<MonoRelease> {
        let monorelease = default_monorelease_env()?;
        monorelease.init()?;
        Ok(monorelease)
    }

    fn sample_project(releases: &[(&str, Wave)]) -> Project {
        Project {
            id: "sample".to_string(),
            config_path: PathBuf::from("sample.yaml"),
            config: ProjectConfig {
                name: "sample".to_string(),
                dependencies: HashMap::new(),
                releases: releases
                    .iter()
                    .map(|(version, wave)| ((*version).to_string(), *wave))
                    .collect(),
            },
        }
    }

    my_test! {
        fn test_get_project_from_config_file_reads_yaml() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_project(&monorelease, "frontend", &[]);

            let project = monorelease.get_project_from_config_file("frontend".to_string())?;

            assert_eq!(project.id, "frontend");
            assert_eq!(
                project.config_path,
                projects_dir(&monorelease).join("frontend.yaml")
            );
            assert_eq!(project.config.name, "frontend");
            Ok(())
        }
    }

    my_test! {
        fn test_get_project_from_config_file_reads_yml() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_config(
                &monorelease,
                "frontend",
                "yml",
                &project_yaml("frontend", &[]),
            );

            let project = monorelease.get_project_from_config_file("frontend".to_string())?;

            assert_eq!(project.id, "frontend");
            assert_eq!(
                project.config_path,
                projects_dir(&monorelease).join("frontend.yml")
            );
            assert_eq!(project.config.name, "frontend");
            Ok(())
        }
    }

    my_test! {
        fn test_get_project_from_config_file_bails_when_both_exist() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_project(&monorelease, "frontend", &[]);
            write_config(
                &monorelease,
                "frontend",
                "yml",
                &project_yaml("frontend", &[]),
            );

            let err = monorelease
                .get_project_from_config_file("frontend".to_string())
                .unwrap_err();

            let message = err.to_string();
            assert!(message.contains("found both"));
            assert!(message.contains("frontend.yaml"));
            assert!(message.contains("frontend.yml"));
            Ok(())
        }
    }

    my_test! {
        fn test_get_project_from_config_file_bails_when_neither_exists() -> Result<()> {
            let monorelease = default_monorelease_env()?;

            let err = monorelease
                .get_project_from_config_file("frontend".to_string())
                .unwrap_err();

            assert_eq!(
                err.to_string(),
                format!(
                    "could not find a config file for project 'frontend' in '{}'",
                    projects_dir(&monorelease).display()
                )
            );
            Ok(())
        }
    }

    my_test! {
        fn test_get_project_from_config_file_rejects_invalid_yaml() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_config(&monorelease, "frontend", "yaml", "name: [unterminated\n");

            let err = monorelease
                .get_project_from_config_file("frontend".to_string())
                .unwrap_err();

            assert!(err.to_string().contains("failed to parse project config"));
            Ok(())
        }
    }

    my_test! {
        fn test_get_project_from_config_file_rejects_missing_fields() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_config(
                &monorelease,
                "frontend",
                "yaml",
                "dependencies: {}\nreleases: {}\n",
            );

            let err = monorelease
                .get_project_from_config_file("frontend".to_string())
                .unwrap_err();

            assert!(err.to_string().contains("failed to parse project config"));
            Ok(())
        }
    }

    my_test! {
        fn test_get_project_from_config_file_rejects_empty_file() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_config(&monorelease, "frontend", "yaml", "");

            let err = monorelease
                .get_project_from_config_file("frontend".to_string())
                .unwrap_err();

            assert!(err.to_string().contains("failed to parse project config"));
            Ok(())
        }
    }

    my_test! {
        fn test_get_project_from_config_file_ignores_unknown_fields() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_config(
                &monorelease,
                "frontend",
                "yaml",
                "name: frontend\ndependencies: {}\nreleases: {}\nextra: 1\n",
            );

            let project = monorelease.get_project_from_config_file("frontend".to_string())?;

            assert_eq!(project.config.name, "frontend");
            Ok(())
        }
    }

    // --- VersionBump::from_magnitude ---

    #[test]
    fn test_from_magnitude_accepts_valid_values() {
        assert_eq!(
            VersionBump::from_magnitude("patch").unwrap(),
            VersionBump::Patch
        );
        assert_eq!(
            VersionBump::from_magnitude("minor").unwrap(),
            VersionBump::Minor
        );
        assert_eq!(
            VersionBump::from_magnitude("major").unwrap(),
            VersionBump::Major
        );
    }

    #[test]
    fn test_from_magnitude_rejects_unknown_values() {
        for invalid in ["", "MAJOR", "Patch", "minor ", "majorly"] {
            let err = VersionBump::from_magnitude(invalid).unwrap_err();
            assert!(
                err.to_string().contains("unknown version bump"),
                "unexpected error for {invalid:?}: {err}"
            );
        }
    }

    #[test]
    fn test_version_bump_ordering() {
        assert!(VersionBump::Patch < VersionBump::Minor);
        assert!(VersionBump::Minor < VersionBump::Major);
    }

    // --- increment ---

    #[test]
    fn test_increment_adds_one_and_detects_overflow() {
        assert_eq!(increment(0, "patch").unwrap(), 1);
        assert_eq!(increment(41, "patch").unwrap(), 42);

        let err = increment(u64::MAX, "major").unwrap_err();
        assert!(
            err.to_string()
                .contains("major version component overflowed")
        );
    }

    // --- VersionBump::apply ---

    #[test]
    fn test_apply_happy_path() {
        assert_eq!(VersionBump::Patch.apply("1.2.3").unwrap(), "1.2.4");
        assert_eq!(VersionBump::Minor.apply("1.2.3").unwrap(), "1.3.0");
        assert_eq!(VersionBump::Major.apply("1.2.3").unwrap(), "2.0.0");
        assert_eq!(VersionBump::Patch.apply("0.0.0").unwrap(), "0.0.1");
        assert_eq!(VersionBump::Minor.apply("0.0.0").unwrap(), "0.1.0");
        assert_eq!(VersionBump::Major.apply("0.0.0").unwrap(), "1.0.0");
    }

    #[test]
    fn test_apply_rejects_malformed_versions() {
        let err = VersionBump::Minor.apply("1").unwrap_err();
        assert!(err.to_string().contains("missing a minor component"));

        let err = VersionBump::Patch.apply("1.2").unwrap_err();
        assert!(err.to_string().contains("missing a patch component"));

        let err = VersionBump::Patch.apply("1.2.3.4").unwrap_err();
        assert!(err.to_string().contains("expected 'major.minor.patch'"));

        for version in ["a.2.3", "1.x.3", "1.2.x", "1.2.3-alpha"] {
            let err = VersionBump::Patch.apply(version).unwrap_err();
            assert!(
                err.to_string().contains("invalid"),
                "unexpected error for {version:?}: {err}"
            );
        }
    }

    #[test]
    fn test_apply_detects_component_overflow() {
        let err = VersionBump::Major
            .apply("18446744073709551615.0.0")
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("major version component overflowed")
        );

        let err = VersionBump::Minor
            .apply("1.18446744073709551615.0")
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("minor version component overflowed")
        );

        let err = VersionBump::Patch
            .apply("1.2.18446744073709551615")
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("patch version component overflowed")
        );
    }

    // --- parse_change_frontmatter ---

    #[test]
    fn test_parse_change_frontmatter_valid() {
        let changes = parse_change_frontmatter("---\nfrontend: minor\n---\nbody").unwrap();
        assert_eq!(changes.get("frontend").map(String::as_str), Some("minor"));

        let changes =
            parse_change_frontmatter("---\nfrontend: minor\nbackend: patch\n---\nbody\n---\nmore")
                .unwrap();
        assert_eq!(changes.get("frontend").map(String::as_str), Some("minor"));
        assert_eq!(changes.get("backend").map(String::as_str), Some("patch"));
        assert_eq!(changes.len(), 2);
    }

    #[test]
    fn test_parse_change_frontmatter_rejects_missing_delimiters() {
        let err = parse_change_frontmatter("frontend: minor\n---").unwrap_err();
        assert!(err.to_string().contains("missing its YAML frontmatter"));

        let err = parse_change_frontmatter("---\nfrontend: minor\n").unwrap_err();
        assert!(err.to_string().contains("not terminated with '---'"));
    }

    #[test]
    fn test_parse_change_frontmatter_rejects_invalid_yaml() {
        let err = parse_change_frontmatter("---\nfrontend: [unterminated\n---\nbody").unwrap_err();
        assert!(err.to_string().contains("failed to parse YAML frontmatter"));

        let err = parse_change_frontmatter("---\nfrontend: [minor]\n---\nbody").unwrap_err();
        assert!(err.to_string().contains("failed to parse YAML frontmatter"));
    }

    // --- Project::current_release_wave ---

    #[test]
    fn test_current_release_wave() {
        assert_eq!(sample_project(&[]).current_release_wave(), 0);
        assert_eq!(
            sample_project(&[("1.0.0", 2), ("1.1.0", 7), ("1.2.0", 3)]).current_release_wave(),
            7
        );
    }

    // --- Project::current_version ---

    #[test]
    fn test_current_version_uses_latest_wave() {
        assert_eq!(sample_project(&[]).current_version(), None);
        assert_eq!(
            sample_project(&[("10.0.0", 1), ("2.0.0", 2)]).current_version(),
            Some("2.0.0")
        );
    }

    // --- Project::save_config ---

    my_test! {
        fn test_save_config_round_trips_and_overwrites() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            fs::create_dir_all(projects_dir(&monorelease))?;
            let config_path = projects_dir(&monorelease).join("sample.yaml");

            let mut project = sample_project(&[("1.0.0", 1)]);
            project.config_path = config_path;
            project.save_config()?;

            project.config.releases.insert("2.0.0".to_string(), 2);
            project.save_config()?;

            let reloaded = monorelease.get_project_from_config_file("sample".to_string())?;
            assert_eq!(reloaded.config, project.config);
            Ok(())
        }
    }

    my_test! {
        fn test_save_config_fails_when_path_unwritable() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            let mut project = sample_project(&[]);
            project.config_path = monorelease
                .releases_dir
                .join("missing_dir")
                .join("sample.yaml");

            let err = project.save_config().unwrap_err();

            assert!(err.to_string().contains("failed to write project config"));
            Ok(())
        }
    }

    // --- next_project_versions ---

    my_test! {
        fn test_next_project_versions_applies_all_bumps() -> Result<()> {
            for (magnitude, expected) in
                [("patch", "1.2.4"), ("minor", "1.3.0"), ("major", "2.0.0")]
            {
                let monorelease = default_monorelease_env()?;
                write_project(&monorelease, "frontend", &[("1.2.3", 1)]);
                write_change_file(&monorelease, 2, "change", &[("frontend", magnitude)]);

                let versions = monorelease.next_project_versions(1, 2)?;

                assert_eq!(
                    versions.get("frontend").map(String::as_str),
                    Some(expected),
                    "unexpected version for {magnitude}"
                );
            }

            let monorelease = default_monorelease_env()?;
            write_project(&monorelease, "frontend", &[]);
            write_change_file(&monorelease, 1, "change", &[("frontend", "minor")]);
            let versions = monorelease.next_project_versions(0, 1)?;
            assert_eq!(versions.get("frontend").map(String::as_str), Some("0.1.0"));
            Ok(())
        }
    }

    my_test! {
        fn test_next_project_versions_highest_bump_wins_across_projects() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_project(&monorelease, "frontend", &[("1.0.0", 1)]);
            write_project(&monorelease, "backend", &[("2.0.0", 1)]);
            write_change_file(&monorelease, 2, "a", &[("frontend", "patch")]);
            write_change_file(&monorelease, 2, "b", &[("frontend", "major")]);
            write_change_file(&monorelease, 3, "c", &[("frontend", "minor")]);
            write_change_file(&monorelease, 3, "d", &[("backend", "patch")]);

            let versions = monorelease.next_project_versions(1, 3)?;

            assert_eq!(versions.get("frontend").map(String::as_str), Some("2.0.0"));
            assert_eq!(versions.get("backend").map(String::as_str), Some("2.0.1"));
            Ok(())
        }
    }

    my_test! {
        fn test_next_project_versions_bails_on_non_markdown_entry() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_wave_entry(&monorelease, 1, "notes.txt", "hello");

            let err = monorelease.next_project_versions(0, 1).unwrap_err();

            assert!(err.to_string().contains("unexpected non-Markdown file"));
            Ok(())
        }
    }

    my_test! {
        fn test_next_project_versions_bails_on_missing_wave_dir() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_change_file(&monorelease, 2, "change", &[("frontend", "minor")]);

            let err = monorelease.next_project_versions(0, 2).unwrap_err();

            let message = err.to_string();
            assert!(message.contains("failed to read wave directory"));
            assert!(message.contains(&wave_dir(&monorelease, 1).display().to_string()));
            Ok(())
        }
    }

    my_test! {
        fn test_next_project_versions_bails_on_bad_change_file() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_project(&monorelease, "frontend", &[("1.0.0", 1)]);
            write_change_file(&monorelease, 2, "bad", &[("frontend", "huge")]);

            let err = monorelease.next_project_versions(1, 2).unwrap_err();
            assert!(err.to_string().contains("invalid version bump in change file"));

            let monorelease = default_monorelease_env()?;
            write_project(&monorelease, "frontend", &[("1.0.0", 1)]);
            write_wave_entry(&monorelease, 2, "bad.md", "not frontmatter");

            let err = monorelease.next_project_versions(1, 2).unwrap_err();
            assert!(err.to_string().contains("failed to parse change file"));
            Ok(())
        }
    }

    my_test! {
        fn test_next_project_versions_bails_on_bad_project_config() -> Result<()> {
            let monorelease = default_monorelease_env()?;
            write_change_file(&monorelease, 1, "change", &[("ghost", "patch")]);

            let err = monorelease.next_project_versions(0, 1).unwrap_err();
            assert!(
                err.to_string()
                    .contains("could not find a config file for project 'ghost'")
            );

            let monorelease = default_monorelease_env()?;
            write_project(&monorelease, "frontend", &[]);
            write_config(
                &monorelease,
                "frontend",
                "yml",
                &project_yaml("frontend", &[]),
            );
            write_change_file(&monorelease, 1, "change", &[("frontend", "minor")]);

            let err = monorelease.next_project_versions(0, 1).unwrap_err();
            assert!(err.to_string().contains("found both"));
            Ok(())
        }
    }

    // --- bump_project ---

    my_test! {
        fn test_bump_project_first_release() -> Result<()> {
            let monorelease = setup_env()?;
            write_project(&monorelease, "frontend", &[]);
            write_change_file(&monorelease, 1, "change", &[("frontend", "minor")]);

            monorelease.bump_project("frontend".to_string())?;

            let project = monorelease.get_project_from_config_file("frontend".to_string())?;
            assert_eq!(project.config.releases.get("0.1.0"), Some(&1));
            Ok(())
        }
    }

    my_test! {
        fn test_bump_project_subsequent_release() -> Result<()> {
            let monorelease = setup_env()?;
            write_project(&monorelease, "frontend", &[("1.2.3", 1)]);
            write_change_file(&monorelease, 2, "change", &[("frontend", "patch")]);

            monorelease.bump_project("frontend".to_string())?;

            let project = monorelease.get_project_from_config_file("frontend".to_string())?;
            assert_eq!(project.config.releases.get("1.2.3"), Some(&1));
            assert_eq!(project.config.releases.get("1.2.4"), Some(&2));
            Ok(())
        }
    }

    my_test! {
        fn test_bump_project_rejects_pending_changes() -> Result<()> {
            let monorelease = setup_env()?;
            write_project(&monorelease, "frontend", &[]);
            let next_dir = monorelease.releases_dir.join(NEXT_DIR_PATH);
            fs::create_dir_all(&next_dir)?;
            fs::write(next_dir.join("pending.md"), "pending")?;

            let err = monorelease
                .bump_project("frontend".to_string())
                .unwrap_err();

            assert!(
                err.to_string()
                    .contains("bumping project when changes are pending")
            );
            Ok(())
        }
    }

    my_test! {
        fn test_bump_project_rejects_no_new_changes_or_project_ahead() -> Result<()> {
            let monorelease = setup_env()?;
            write_project(&monorelease, "frontend", &[]);
            let err = monorelease
                .bump_project("frontend".to_string())
                .unwrap_err();
            assert!(
                err.to_string()
                    .contains("no new change files since current project release")
            );

            let monorelease = setup_env()?;
            write_project(&monorelease, "frontend", &[("1.0.0", 5)]);
            let err = monorelease
                .bump_project("frontend".to_string())
                .unwrap_err();
            assert!(
                err.to_string()
                    .contains("current project release is later than the currently last wave")
            );
            Ok(())
        }
    }

    my_test! {
        fn test_bump_project_rejects_unknown_target_or_missing_config() -> Result<()> {
            let monorelease = setup_env()?;
            write_project(&monorelease, "frontend", &[]);
            write_project(&monorelease, "backend", &[]);
            write_change_file(&monorelease, 1, "change", &[("backend", "minor")]);
            let err = monorelease
                .bump_project("frontend".to_string())
                .unwrap_err();
            assert!(err.to_string().contains("no change files targeting the project"));

            let monorelease = setup_env()?;
            write_project(&monorelease, "frontend", &[]);
            write_change_file(
                &monorelease,
                1,
                "change",
                &[("frontend", "minor"), ("ghost", "patch")],
            );
            let err = monorelease
                .bump_project("frontend".to_string())
                .unwrap_err();
            assert!(
                err.to_string()
                    .contains("could not find a config file for project 'ghost'")
            );
            Ok(())
        }
    }

    my_test! {
        fn test_bump_project_rejects_version_collision() -> Result<()> {
            let monorelease = setup_env()?;
            write_project(&monorelease, "frontend", &[("0.1.0", 1), ("0.2.0", 0)]);
            write_change_file(&monorelease, 2, "change", &[("frontend", "minor")]);

            let err = monorelease
                .bump_project("frontend".to_string())
                .unwrap_err();

            assert!(err.to_string().contains("version already exists for project"));
            Ok(())
        }
    }
}
