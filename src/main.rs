mod commands;
mod config;
mod flow;
mod prompt;
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
        /// A built-in flow, or a path to one of your own. It becomes yours.
        #[arg(long)]
        preset: Option<String>,
    },
    /// List the flows that ship in the binary
    Presets,
    /// Begin a new run through the flow
    ///
    /// Anything left out is asked for, when you are at a terminal. Elsewhere —
    /// an agent, a script, a pipe — what you pass is all it gets.
    Start {
        title: Option<String>,
        /// feature, bug, task, project — free text, for your eyes only
        #[arg(long)]
        kind: Option<String>,
        /// The brief: what this work is, in your words. It becomes the run's
        /// first handoff, and is all the first stage has to go on.
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Choose the run that bare commands act on
    Switch { slug: String },
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
    /// Show where settings live, and which file each one came from
    Config {
        /// Write the starter user config if it does not exist yet
        #[arg(long)]
        init: bool,
    },
    /// Hand the current stage to an agent, with the prompt already assembled
    Go {
        slug: Option<String>,
        /// Which configured agent to launch
        #[arg(long)]
        agent: Option<String>,
        /// Show the prompt instead of launching anything
        #[arg(long)]
        print: bool,
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
    /// Bring a finished run back onto the board
    Reopen {
        slug: Option<String>,
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
    let cwd = match cli.root.clone() {
        Some(root) => root,
        None => std::env::current_dir()?,
    };

    // `init` writes where it is told; everything else searches upward for a
    // `.flow`, so the commands work from anywhere inside the repo.
    let root = match cli.command {
        Command::Init { .. } => cwd,
        Command::Config { .. } | Command::Presets => flow::find_root(&cwd),
        _ => flow::find_root(&cwd),
    };

    match cli.command {
        Command::Init { preset } => commands::init::run(&root, preset.as_deref()),
        Command::Presets => commands::config::presets(),
        Command::Config { init } => {
            if init {
                commands::config::init()
            } else {
                commands::config::show(&root)
            }
        }
        Command::Start {
            title,
            kind,
            message,
        } => {
            commands::lifecycle::start(&root, title.as_deref(), kind.as_deref(), message.as_deref())
        }
        Command::Show { slug } => commands::lifecycle::show(&root, slug.as_deref()),
        Command::Finish { slug, message } => {
            commands::lifecycle::finish(&root, slug.as_deref(), message.as_deref())
        }
        Command::Reopen { slug, message } => {
            commands::lifecycle::reopen(&root, slug.as_deref(), message.as_deref())
        }
        Command::Switch { slug } => commands::lifecycle::switch(&root, &slug),
        Command::Status { all } => commands::view::status(&root, all),
        Command::Board { output, all } => commands::view::board(&root, output, all),
        Command::Go { slug, agent, print } => {
            commands::agent::go(&root, slug.as_deref(), agent.as_deref(), print)
        }
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
