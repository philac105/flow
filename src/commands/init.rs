use crate::flow::{flow_path, runs_dir};
use anyhow::{anyhow, Result};
use std::path::Path;

const MAIN_FLOW: &str = include_str!("../../assets/main-flow.toml");
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
pub fn run(root: &Path, preset: &str) -> Result<()> {
    let contents = match preset {
        "main-flow" => MAIN_FLOW,
        other => {
            return Err(anyhow!(
                "unknown preset `{other}` — the built-in preset is `main-flow`"
            ))
        }
    };

    std::fs::create_dir_all(runs_dir(root))?;

    let path = flow_path(root);
    if path.exists() {
        println!("  kept {} (already yours)", rel(root, &path));
    } else {
        std::fs::write(&path, contents)?;
        println!("wrote {}", rel(root, &path));
    }

    let skill = root.join(".claude/skills/flow/SKILL.md");
    std::fs::create_dir_all(skill.parent().unwrap())?;
    std::fs::write(&skill, ADAPTER_SKILL)?;
    println!("wrote {}", rel(root, &skill));

    let agents = root.join("AGENTS.md");
    write_agents_block(&agents)?;
    println!("wrote {} (flow block)", rel(root, &agents));

    println!(
        "\nEdit {} to make the flow yours, then:\n  flow start \"<what you're building>\"",
        rel(root, &flow_path(root))
    );
    Ok(())
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

