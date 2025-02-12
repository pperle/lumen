use clap::{command, Parser, Subcommand, ValueEnum};
use error::LumenError;
use git_entity::{git_commit::GitCommit, git_diff::GitDiff, GitEntity};
use std::process;

mod ai_prompt;
mod command;
mod error;
mod git_entity;
mod provider;

#[derive(Parser)]
#[command(name = "lumen")]
#[command(about = "AI-powered CLI tool for git commit summaries", long_about = None)]
struct Cli {
    #[arg(
        value_enum,
        short = 'p',
        long = "provider",
        env("LUMEN_AI_PROVIDER"),
        default_value = "phind"
    )]
    provider: ProviderType,

    #[arg(short = 'k', long = "api-key", env = "LUMEN_API_KEY")]
    api_key: Option<String>,

    #[arg(short = 'm', long = "model", env = "LUMEN_AI_MODEL")]
    model: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum, Debug)]
enum ProviderType {
    Openai,
    Phind,
    Groq,
    Claude,
    Ollama,
}

#[derive(Subcommand)]
enum Commands {
    /// Explain the changes in a commit, or the current diff
    Explain {
        /// The commit hash to use
        #[arg(group = "target")]
        sha: Option<String>,

        /// Explain current diff
        #[arg(long, group = "target")]
        diff: bool,

        /// Use staged diff
        #[arg(long)]
        staged: bool,

        /// Ask a question instead of summary
        #[arg(short, long)]
        query: Option<String>,
    },
    /// List all commits in an interactive fuzzy-finder, and summarize the changes
    List,
    /// Generate a commit message for the staged changes
    Draft {
        /// Add context to communicate intent
        #[arg(short, long)]
        context: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("\x1b[91m\rerror:\x1b[0m {e}");
        process::exit(1);
    }
}

async fn run() -> Result<(), LumenError> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();
    let provider = provider::LumenProvider::new(client, cli.provider, cli.api_key, cli.model)?;
    let command = command::LumenCommand::new(provider);

    match cli.command {
        Commands::Explain {
            sha,
            diff,
            staged,
            query,
        } => {
            let git_entity = if diff {
                GitEntity::Diff(GitDiff::new(staged)?)
            } else if let Some(sha) = sha {
                GitEntity::Commit(GitCommit::new(sha)?)
            } else {
                return Err(LumenError::InvalidArguments(
                    "`explain` expects SHA-1 or --diff to be present".into(),
                ));
            };

            command
                .execute(command::CommandType::Explain { git_entity, query })
                .await?;
        }
        Commands::List => command.execute(command::CommandType::List).await?,
        Commands::Draft { context } => {
            command
                .execute(command::CommandType::Draft(context))
                .await?
        }
    }

    Ok(())
}
