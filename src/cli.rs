use std::env;

use anyhow::{Context, Result};
use monorelease::{MonoRelease, changes::ProjectChange, config::Config, logging::Log};

#[derive(clap::Parser, Debug, Clone)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Command {
    /// Prints the version of MonoRelease.
    Version,
    /// Initialise MonoRelease in the current directory.
    Init,
    /// Add a change.
    /// Is interactive by default unless a description is provided.
    Add {
        #[arg(value_parser = parse_change_project_input)]
        projects: Vec<ProjectChange>,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Wave commands.
    Waves,
    /// Project commands.
    Projects {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Generate changelogs.
    Changelogs {
        #[command(subcommand)]
        command: ChangelogCommand,
    },
}

const CHANGE_PROJECT_INPUT_DELIMITER: &str = ":";
fn parse_change_project_input(s: &str) -> Result<ProjectChange> {
    let (project_name, version_bump) = s
        .split_once(CHANGE_PROJECT_INPUT_DELIMITER)
        .with_context(|| format!("does not contain '{CHANGE_PROJECT_INPUT_DELIMITER}'"))?;

    Ok(ProjectChange {
        project_name: project_name.to_string(),
        version_bump: version_bump.to_string(),
    })
}

#[derive(clap::Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum ProjectCommand {
    Bump {
        project: String,

        /// Performs a dry-run.
        #[arg(long)]
        dry: bool,
    },
    /// Revert the last bump of a project.
    Revert { project: String },
}

pub type Wave = u32;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ChangelogCommand {
    /// Generate the changelog for a single exact version of a project.
    Exact {
        /// The project ID.
        project: String,
        /// The project version to generate the changelog for.
        version: String,
    },
    /// Generate the changelog across a range of versions of a project.
    Range {
        project: String,
        /// The starting point of the changelog (inclusive).
        /// Can either be a version of the project or a wave.
        ///
        /// Examples:
        /// - version:1.2.3
        /// - wave:42
        from: String,
        /// The end point of the changelog (inclusive).
        /// Can either be a version of the project or a wave.
        ///
        /// Examples:
        /// - version:1.2.3
        /// - wave:42
        to: String,
    },
    /// Generate the full changelog for a project.
    Full { project: String },
}

pub struct Cli;

impl Cli {
    #[allow(clippy::todo, unused)]
    pub fn run(args: Args, log: &Log) -> Result<()> {
        match args.command {
            Command::Version => {
                println!("{}", MonoRelease::version());
                Ok(())
            }
            Command::Init => {
                let dir = env::current_dir()?;
                Config::ensure_file_exists_in_dir(&dir);

                let config = Config::load_from_dir(&dir)?;
                log.apply_config(&config.log);
                let monorelease = MonoRelease::new(config);

                monorelease.init(&env::current_dir()?)
            }
            Command::Add {
                name,
                message,
                projects,
            } => {
                // TODO: Implement interactive prompts

                const INTERACTIVE_PROMPT_ERROR: &str =
                    "interactive prompts have not been implemented yet";
                let name = name.context(INTERACTIVE_PROMPT_ERROR)?;
                let message = message.context(INTERACTIVE_PROMPT_ERROR)?;

                let monorelease = monorelease_from_config(log)?;
                monorelease.add(&name, &message, &projects)
            }
            Command::Waves => todo!(),
            Command::Projects { command } => match command {
                ProjectCommand::Bump { project, dry } => todo!(),
                ProjectCommand::Revert { project } => todo!(),
            },
            Command::Changelogs { command } => match command {
                ChangelogCommand::Exact { project, version } => todo!(),
                ChangelogCommand::Range { project, from, to } => todo!(),
                ChangelogCommand::Full { project } => todo!(),
            },
        }
    }
}

fn monorelease_from_config(log: &Log) -> Result<MonoRelease> {
    let dir = env::current_dir()?;
    let config = Config::load_from_dir(&dir)?;
    log.apply_config(&config.log)?;
    Ok(MonoRelease::new(config))
}
