use crate::config::Settings;
use crate::flow::{Flow, Launcher};
use crate::run::{self, Run};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;

/// Hand the current stage to an agent.
///
/// The prompt is assembled here; the launching is whatever `.flow/flow.toml`
/// declared. `flow go` is for a human at a terminal — an agent that is already
/// running should use `flow next`, which is why the guard exists.
pub fn go(root: &Path, slug: Option<&str>, agent: Option<&str>, print_only: bool) -> Result<()> {
    let flow = Flow::load(root)?;
    let run = run::resolve(root, slug)?;

    let Some(index) = run.current_index() else {
        return Err(anyhow!(
            "`{}` has no stage in flight — nothing to hand over",
            run.meta.slug
        ));
    };
    if run.is_finished() {
        return Err(anyhow!(
            "`{}` is finished — bring it back with `flow reopen` first",
            run.meta.slug
        ));
    }

    let settings = Settings::resolve(&flow, agent)?;
    let (name, launcher) = settings.launcher()?;
    let stage = &flow.stages[index];
    let prompt = build_prompt(&flow, &run, index, Some(name));

    if print_only {
        println!("{prompt}");
        return Ok(());
    }

    if let Some(marker) = nesting_marker(launcher) {
        println!(
            "`{marker}` is set — an agent is already running here, so `flow go` \
             would nest a session inside a session.\n\nHand it this instead:\n\n{prompt}"
        );
        return Ok(());
    }

    let argv: Vec<String> = launcher
        .command
        .iter()
        .map(|arg| {
            arg.replace("{prompt}", &prompt)
                .replace("{slug}", &run.meta.slug)
                .replace("{stage}", &stage.name)
        })
        .collect();

    println!("→ {name}: {}\n", argv[0]);
    let status = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(root)
        .status()
        .map_err(|e| anyhow!("could not start `{}`: {e}", argv[0]))?;

    if !status.success() {
        println!("\n`{}` exited {}", argv[0], describe(status));
    }

    report_after(root, &flow, &run.meta.slug)
}

/// What the agent is told. The stage's command comes first so it reads as an
/// instruction, with everything a cold agent needs underneath it.
fn build_prompt(flow: &Flow, run: &Run, index: usize, agent: Option<&str>) -> String {
    let stage = &flow.stages[index];
    let mut out = String::new();

    let command = stage.command_for(agent);
    if !command.is_empty() {
        out.push_str(command);
        out.push_str("\n\n");
    }

    let kind = if run.meta.kind.is_empty() {
        String::new()
    } else {
        format!(", a {}", run.meta.kind)
    };
    out.push_str(&format!(
        "You are working on `{}` ({}){kind}.\nStage {} of {}: **{}** — {}\n",
        run.meta.slug,
        run.meta.title,
        index + 1,
        flow.stages.len(),
        stage.name,
        stage.description,
    ));

    let handoff = run.handoff.trim();
    if !handoff.is_empty() {
        out.push_str(&format!("\n## Where we are\n\n{handoff}\n"));
    }

    out.push_str("\n## Recording\n\nWhen this stage is genuinely complete:\n\n    flow done ");
    out.push_str(&run.meta.slug);
    out.push_str(" -m \"<where this leaves the work, for someone who has never seen it>\"");
    if let Some(artifact) = stage.artifact_for(&run.meta.slug) {
        if !crate::flow::is_tracker_artifact(&artifact) {
            out.push_str(&format!(" --artifact {artifact}"));
        }
    }
    out.push_str(
        "\n\nDo not record it until it is done. The file outlives this session, and a \
         lie in it costs a later session real time.\n",
    );
    out
}

/// The first guard variable that is set, if any.
fn nesting_marker(launcher: &Launcher) -> Option<&str> {
    launcher
        .guard_env
        .iter()
        .find(|var| std::env::var_os(var).is_some())
        .map(String::as_str)
}

/// Say what changed while the agent had it. Never records anything — an agent
/// exiting cleanly means the session ended, not that the work is done.
fn report_after(root: &Path, flow: &Flow, slug: &str) -> Result<()> {
    let run = Run::load(&Run::path_for(root, slug))?;
    match run.current_stage_name() {
        Some(stage) => {
            println!("\n`{slug}` still reads `{stage}`.");
            for d in run.drift(flow, root) {
                println!("  ! {}: {}", d.stage, d.message);
            }
            println!("Record it with `flow done` when it is done, or `flow go` again to carry on.");
        }
        None => println!("\n`{slug}` has no stage left — close it with `flow finish`."),
    }
    Ok(())
}

fn describe(status: std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("with status {code}"),
        None => "on a signal".to_string(),
    }
}
