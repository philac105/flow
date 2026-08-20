use crate::config::{user_config_path, Settings};
use crate::flow::{flow_path, Flow};
use anyhow::{anyhow, Result};
use std::path::Path;

const STARTER: &str = include_str!("../../assets/user-config.toml");

/// Show where settings live and which file each one came from. This is the
/// answer to "where do I set this up" — so it names paths, always.
pub fn show(root: &Path) -> Result<()> {
    // A malformed flow must say so rather than reading as an absent one.
    let flow = if flow_path(root).is_file() {
        Some(Flow::load(root)?)
    } else {
        None
    };
    let fallback = empty_flow();
    let settings = Settings::resolve(flow.as_ref().unwrap_or(&fallback), None)?;

    println!("Yours — the agent you drive, and how it starts on this machine:");
    match &settings.user_path {
        Some(path) => println!(
            "  {}{}",
            path.display(),
            if settings.user_exists {
                ""
            } else {
                "   (does not exist — `flow config --init` writes it)"
            }
        ),
        None => println!("  unavailable (no HOME or XDG_CONFIG_HOME)"),
    }

    println!("\nThe project's — which stages exist, committed and shared:");
    let path = flow_path(root);
    println!(
        "  {}{}",
        path.display(),
        if path.is_file() {
            ""
        } else {
            "   (no flow here — `flow init`)"
        }
    );

    println!(
        "\nDefault agent: {}",
        if settings.agent.is_empty() {
            "none".to_string()
        } else {
            format!("{}   (from {})", settings.agent, settings.agent_source)
        }
    );

    if settings.agents.is_empty() {
        println!("\nNo agents configured. `flow config --init`, then `flow go` works.");
    } else {
        println!("\nAgents:");
        for (name, (launcher, source)) in &settings.agents {
            let mark = if *name == settings.agent { "*" } else { " " };
            println!(
                "  {mark} {name}  {}   (from {source})",
                launcher.command.join(" ")
            );
        }
    }
    Ok(())
}

/// Write the starter user config, never clobbering one that exists.
pub fn init() -> Result<()> {
    let path = user_config_path()
        .ok_or_else(|| anyhow!("no HOME or XDG_CONFIG_HOME to write a config into"))?;
    if path.exists() {
        println!("{} already exists — left alone.", path.display());
        return Ok(());
    }
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, STARTER)?;
    println!(
        "wrote {}\n\nEdit it to pick your agent, then `flow go`.",
        path.display()
    );
    Ok(())
}

/// `flow config` should work outside a flow repo, where there is no flow to read.
fn empty_flow() -> Flow {
    Flow {
        name: String::new(),
        description: String::new(),
        agent: String::new(),
        agents: Default::default(),
        stages: Vec::new(),
    }
}
