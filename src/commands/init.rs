use crate::config::user_config_path;
use crate::flow::{flow_path, runs_dir};
use anyhow::{anyhow, Result};
use std::path::Path;

const ADAPTER_SKILL: &str = include_str!("../../assets/adapter-skill.md");

const BLOCK_START: &str = "<!-- flow:start -->";
const BLOCK_END: &str = "<!-- flow:end -->";

const AGENTS_BLOCK: &str = r#"## Flow

This repo tracks work with `flow`. Run `flow status` to see every run and the
stage it is on, and `flow next` for the current run's stage and the command to
run for it. `flow` never runs commands itself — it prints them and records that
they happened.

When a stage is complete, record it with `flow done -m "<handoff>"`. The message
replaces the run's `## Where we are` block and is appended to its `## Log`, so
write it for someone who has never seen the work.

State lives in `.flow/` and is the source of truth, not the conversation. Commit
it alongside the work."#;

/// Write a preset into the repo, plus the agent adapter. Additive and
/// idempotent throughout — see ADR-0004.
pub fn run(root: &Path, preset: Option<&str>) -> Result<()> {
    // Which flow you reach for by default is a preference, so it lives with
    // your other preferences (ADR-0007).
    let (user, _) = crate::config::UserConfig::load()?;
    let (chosen, asked_by) = match preset {
        Some(name) => (name.to_string(), Asked::Outright),
        None if !user.preset.is_empty() => (user.preset.clone(), Asked::UserConfig),
        // A named flow, deliberately, rather than whichever file sorts first.
        None => (crate::presets::DEFAULT.to_string(), Asked::Outright),
    };
    // Before anything is written: a name that resolves to nothing must leave
    // the repo exactly as it found it.
    let (contents, origin) = resolve_preset(root, &chosen, asked_by)?;

    std::fs::create_dir_all(runs_dir(root))?;
    // Which run you are on is yours, like a checked-out branch. A gitignore
    // inside .flow keeps it local without touching the repo's own.
    std::fs::write(
        crate::flow::flow_dir(root).join(".gitignore"),
        "# Which run you are working on — local to your checkout.\n/current\n",
    )?;

    let path = flow_path(root);
    if path.exists() {
        println!("  kept {} (already yours)", rel(root, &path));
    } else {
        std::fs::write(&path, &contents)?;
        println!("wrote {}   ({origin})", rel(root, &path));
    }

    let skill = root.join(".claude/skills/flow/SKILL.md");
    std::fs::create_dir_all(skill.parent().unwrap())?;
    std::fs::write(&skill, ADAPTER_SKILL)?;
    println!("wrote {}", rel(root, &skill));

    let agents = root.join("AGENTS.md");
    write_agents_block(&agents)?;
    println!("wrote {} (flow block)", rel(root, &agents));

    println!(
        "\nEdit {} to make the flow yours, then:\n  flow start   (it asks what you're building, and what it is)",
        rel(root, &flow_path(root))
    );

    // Which agent you drive is yours, not the repo's, and lives elsewhere.
    if !user_config_path().is_some_and(|p| p.is_file()) {
        println!(
            "\nTo hand stages to an agent with `flow go`, set one up once per machine:\n               flow config --init"
        );
    }
    Ok(())
}

/// Where the name came from — the two cases that need different words when it
/// resolves to nothing. A name you typed and the pinned default are the same
/// case: the message names the name, and that is all either needs.
#[derive(Clone, Copy)]
enum Asked {
    Outright,
    UserConfig,
}

/// A preset's name, resolved through the Preset Path, or a path to a flow you
/// wrote yourself. Returns the flow and where it was taken from.
fn resolve_preset(root: &Path, chosen: &str, asked_by: Asked) -> Result<(String, String)> {
    // The same walk the listing uses, so precedence is one rule everywhere
    // rather than one rule per command.
    let found = crate::preset_path::discover(root).presets;
    if let Some(preset) = found.iter().find(|preset| preset.name == chosen) {
        let origin = format!("{}, from {}", preset.name, preset.layer);
        return Ok((preset.contents.clone(), origin));
    }

    // A path is not a discovered preset: it is read verbatim, and the rule that
    // a preset is named by its file does not apply to it.
    let path = Path::new(chosen);
    if path.is_file() {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("could not read {}: {e}", path.display()))?;
        return Ok((
            contents,
            format!("{}, read as you wrote it", path.display()),
        ));
    }

    let available = found
        .iter()
        .map(|preset| preset.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(match asked_by {
        Asked::UserConfig => anyhow!(
            "your user config sets `preset = \"{chosen}\"`, but no preset or file is called \
             that — presets on your path: {available}. See `flow presets`."
        ),
        Asked::Outright => anyhow!(
            "no preset or file called `{chosen}` — presets on your path: {available}. \
             See `flow presets`."
        ),
    })
}

/// Replace the delimited flow block in place, or append one. Everything outside
/// the markers is preserved byte for byte.
fn write_agents_block(path: &Path) -> Result<()> {
    let block = format!("{BLOCK_START}\n{AGENTS_BLOCK}\n{BLOCK_END}");
    let existing = std::fs::read_to_string(path).unwrap_or_default();

    let updated = match (existing.find(BLOCK_START), existing.find(BLOCK_END)) {
        (Some(start), Some(end)) if end > start => {
            let mut out = String::with_capacity(existing.len());
            out.push_str(&existing[..start]);
            out.push_str(&block);
            out.push_str(&existing[end + BLOCK_END.len()..]);
            out
        }
        _ if existing.trim().is_empty() => format!("{block}\n"),
        _ => format!("{}\n\n{block}\n", existing.trim_end()),
    };
    std::fs::write(path, updated)?;
    Ok(())
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
