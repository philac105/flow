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

    // Where a flow of your own goes. `flow` only ever reads these (ADR-0008),
    // so an absent one is named rather than omitted: the answer to "where do I
    // create it" has to be the exact path on screen.
    match crate::config::user_presets_dir() {
        Some(dir) => println!("  {}{}", dir.display(), absence_note(&dir)),
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

    // The nearest presets directory always, because that is the one you would
    // create; the ancestors only when they exist, because every directory up to
    // the ceiling of the walk is one you could theoretically create.
    let mut project_dirs = crate::preset_path::project_dirs(root).into_iter();
    if let Some(nearest) = project_dirs.next() {
        println!("  {}{}", nearest.display(), absence_note(&nearest));
    }
    for inherited in project_dirs.filter(|dir| dir.is_dir()) {
        println!("  {}   (inherited from an ancestor)", inherited.display());
    }

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

/// Wide enough for the longest layer name, which is a closed set of three.
const LAYER_WIDTH: usize = 9;

/// List every flow you could init with, from all three layers of the Preset
/// Path, saying where each one came from.
pub fn presets(root: &Path) -> Result<()> {
    let (user, _) = crate::config::UserConfig::load()?;
    let chosen = !user.preset.is_empty();
    let default = if chosen {
        user.preset.as_str()
    } else {
        crate::presets::DEFAULT
    };

    let found = crate::preset_path::discover(root);
    let presets = &found.presets;
    println!("Flows you can init with. Where two share a name the project's wins, then yours,");
    println!(
        "then what ships. `flow init --preset <name>`, or pass a path to a .toml of your own.\n"
    );

    // Wide enough for the longest name, so a preset someone wrote does not
    // wrap the column just by having a longer name than ours.
    let width = presets
        .iter()
        .map(|p| p.name.len())
        .max()
        .unwrap_or(0)
        .max(10)
        + 2;
    for preset in presets {
        let mark = if preset.name == default { "*" } else { " " };
        println!(
            "  {mark} {:<width$}{:<LAYER_WIDTH$}{}",
            preset.name,
            preset.layer.label(),
            preset.description
        );
        // A shadowed preset stays on screen, with the description of what was
        // overridden: silent shadowing is how someone loses an afternoon to a
        // flow they do not recognise. Both layers are named in full, so two
        // ancestors carrying the same name are told apart.
        for beaten in &preset.shadowed {
            println!(
                "      {} shadows {}{}",
                preset.layer,
                beaten.layer,
                if beaten.description.is_empty() {
                    String::new()
                } else {
                    format!(": {}", beaten.description)
                }
            );
        }
    }

    // Only when there is something to say: a directory with nothing wrong in
    // it should look like a directory with nothing wrong in it.
    if !found.skipped.is_empty() {
        println!("\nSkipped:");
        for file in &found.skipped {
            println!("    {} {}", file.path.display(), file.reason);
        }
    }

    // A default that resolves to nothing leaves every row unmarked, and this
    // listing is where someone comes to work out why `flow init` refused. The
    // footer alone would explain a `*` that is not on screen.
    if !presets.iter().any(|preset| preset.name == default) {
        if chosen {
            println!(
                "\nYour user config sets `preset = \"{default}\"`, and nothing above is named \
                 that — a bare `flow init` has nothing to write."
            );
        } else {
            println!(
                "\nNothing above is named `{default}`, so a bare `flow init` has nothing to write."
            );
        }
    }

    println!("\n* is what a bare `flow init` writes. Change it with `preset = \"<name>\"` in your user config.");
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

/// The note a directory carries when it is not there yet.
fn absence_note(dir: &Path) -> &'static str {
    if dir.is_dir() {
        ""
    } else {
        "   (does not exist — drop a .toml in here to add a flow)"
    }
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
