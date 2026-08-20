mod commands;
mod flow;
mod run;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Track work from idea to shipped code, in files that outlive the session.
///
/// `flow` never runs your commands. It tells you which one is next and records
/// that it happened, so a session that ends — or runs out of tokens — costs you
/// nothing.
#[derive(Parser)]
#[command(name = "flow", version, about, long_about = None)]
struct Cli {
    /// Repo to operate on. Defaults to the nearest ancestor holding a `.flow`.
    #[arg(long, global = true, value_name = "PATH")]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write a flow and the agent adapter into this repo
    Init {
        /// Which built-in flow to write out. It becomes yours — edit it freely.
        #[arg(long, default_value = "main-flow")]
        preset: String,
    },
    /// Begin a new run through the flow
    Start {
        title: String,
        /// feature, bug, task, project — free text, for your eyes only
        #[arg(long, default_value = "")]
        kind: String,
    },
    /// The board: every run and where it stands
    Status {
        /// Include finished runs
        #[arg(long)]
        all: bool,
    },
    /// One run's position, handoff and full log
    Show { slug: Option<String> },
    /// What to do now, and the command to do it with
    Next {
        slug: Option<String>,
        /// Show a per-agent command override, when the stage declares one
        #[arg(long)]
        agent: Option<String>,
    },
    /// Mark the current stage complete and move to the next
    Done {
        slug: Option<String>,
        /// The handoff: where this leaves the work. Written for a cold reader.
        #[arg(short, long)]
        message: Option<String>,
        /// What this stage actually produced
        #[arg(long)]
        artifact: Option<String>,
    },
    /// Pass over the current stage, when the flow marks it optional
    Skip {
        slug: Option<String>,
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Send a run back to an earlier stage — review kicking work back
    Back {
        slug: Option<String>,
        /// Stage to return to. Defaults to the one before the current.
        #[arg(long)]
        stage: Option<String>,
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Mark a run finished so it drops off the board
    Finish {
        slug: Option<String>,
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Write a standalone HTML board you can open without a session running
    Board {
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
        #[arg(long)]
        all: bool,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let cwd = cli
        .root
        .clone()
        .unwrap_or(std::env::current_dir()?);

    // `init` writes where it is told; everything else searches upward for a
    // `.flow`, so the commands work from anywhere inside the repo.
    let root = match cli.command {
        Command::Init { .. } => cwd,
        _ => flow::find_root(&cwd),
    };

    match cli.command {
        Command::Init { preset } => commands::init::run(&root, &preset),
        Command::Start { title, kind } => commands::lifecycle::start(&root, &title, &kind),
        Command::Show { slug } => commands::lifecycle::show(&root, slug.as_deref()),
        Command::Finish { slug, message } => {
            commands::lifecycle::finish(&root, slug.as_deref(), message.as_deref())
        }
        Command::Status { all } => commands::view::status(&root, all),
        Command::Board { output, all } => commands::view::board(&root, output, all),
        Command::Next { slug, agent } => {
            commands::transitions::next(&root, slug.as_deref(), agent.as_deref())
        }
        Command::Done {
            slug,
            message,
            artifact,
        } => commands::transitions::done(
            &root,
            slug.as_deref(),
            message.as_deref(),
            artifact.as_deref(),
        ),
        Command::Skip { slug, message } => {
            commands::transitions::skip(&root, slug.as_deref(), message.as_deref())
        }
        Command::Back {
            slug,
            stage,
            message,
        } => commands::transitions::back(
            &root,
            slug.as_deref(),
            stage.as_deref(),
            message.as_deref(),
        ),
    }
}
