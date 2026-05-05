use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "nxc", about = "Nexus Code — AI coding agent", version)]
pub struct Cli {
    /// One-shot prompt (skip interactive session)
    pub prompt: Option<String>,

    /// Override the model for this run
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Auto-execute all tool calls without asking (fastest)
    #[arg(long)]
    pub yolo: bool,

    /// Ask before every tool call (safest)
    #[arg(long)]
    pub safe: bool,

    /// Resume the most recent session in cwd
    #[arg(long)]
    pub resume: bool,

    #[command(subcommand)]
    pub command: Option<Sub>,
}

#[derive(Subcommand, Debug)]
pub enum Sub {
    /// Run the setup wizard
    Init,
    /// List available OpenRouter models
    Models,
    /// Open config file in $EDITOR
    Config,
    /// List saved sessions
    Sessions,
}
