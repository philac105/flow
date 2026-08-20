use crate::flow::Flow;
use crate::run::{self, Run, RunStatus, StageStatus};
use anyhow::{anyhow, Result};
use std::path::Path;

pub fn start(root: &Path, title: &str, kind: &str) -> Result<()> {
    let flow = Flow::load(root)?;
    let slug = run::slugify(title);
    if slug.is_empty() {
        return Err(anyhow!(
            "`{title}` has no characters a slug can be made from"
        ));
    }

    let path = Run::path_for(root, &slug);
    if path.exists() {
        return Err(anyhow!(
            "a run called `{slug}` already exists at {}",
            path.display()
        ));
    }

    let mut run = Run::new(slug.clone(), title.to_string(), kind.to_string(), &flow);
    run.path = path;
    let first = &flow.stages[0];
    run.record(
        &format!("Started. First stage is `{}`.", first.name),
        Some(&format!(
            "{}\n\nRun `{}` next.",
            first.description, first.command
        )),
    );
    run.save()?;
    // Starting something is the clearest possible statement of what you are
    // working on, so it becomes current.
    run::set_current(root, &slug)?;

    println!("started `{slug}` (now current)\n");
    print_next(&flow, &run, None);
    Ok(())
}

pub fn show(root: &Path, slug: Option<&str>) -> Result<()> {
    let flow = Flow::load(root)?;
    let run = run::resolve(root, slug)?;

    println!("{} ({})", run.meta.title, run.meta.slug);
    let kind = if run.meta.kind.is_empty() {
        "—".to_string()
    } else {
        run.meta.kind.clone()
    };
    let (done, total) = run.progress();
    println!(
        "{kind} · flow `{}` · {done}/{total} stages · {}",
        run.meta.flow,
        if run.is_finished() {
            "finished"
        } else {
            "active"
        }
    );
    println!("updated {}\n", run.meta.updated);

    let current = run.current_index();
    for (i, record) in run.meta.stages.iter().enumerate() {
        let marker = match record.status {
            StageStatus::Done => "[x]",
            StageStatus::Skipped => "[-]",
            _ if Some(i) == current => "[>]",
            _ => "[ ]",
        };
        print!("  {marker} {}", record.name);
        if let Some(artifact) = &record.artifact {
            print!("  → {artifact}");
        }
        println!();
    }

    let drift = run.drift(&flow, root);
    if !drift.is_empty() {
        println!("\ndrift:");
        for d in &drift {
            println!("  ! {}: {}", d.stage, d.message);
        }
    }

    println!("\n## Where we are\n\n{}", run.handoff.trim());
    if !run.log.trim().is_empty() {
        println!("\n## Log\n\n{}", run.log.trim());
    }
    Ok(())
}

pub fn finish(root: &Path, slug: Option<&str>, message: Option<&str>) -> Result<()> {
    let mut run = run::resolve(root, slug)?;
    if run.is_finished() {
        return Err(anyhow!("`{}` is already finished", run.meta.slug));
    }
    run.meta.status = RunStatus::Finished;
    run.record("Run finished.", message);
    run.save()?;
    println!("finished `{}`", run.meta.slug);
    Ok(())
}

pub fn switch(root: &Path, slug: &str) -> Result<()> {
    let flow = Flow::load(root)?;
    let run = run::resolve(root, Some(slug))?;
    run::set_current(root, &run.meta.slug)?;
    println!("switched to `{}`\n", run.meta.slug);
    print_next(&flow, &run, None);
    Ok(())
}

pub fn reopen(root: &Path, slug: Option<&str>, message: Option<&str>) -> Result<()> {
    let flow = Flow::load(root)?;
    let mut run = run::resolve(root, slug)?;
    if !run.is_finished() {
        return Err(anyhow!("`{}` is already active", run.meta.slug));
    }
    run.meta.status = RunStatus::Active;
    run.record("Reopened.", message);
    run.save()?;

    println!("reopened `{}`\n", run.meta.slug);
    print_next(&flow, &run, None);
    Ok(())
}

/// Shared by `start` and `next` so both describe a stage the same way.
pub fn print_next(flow: &Flow, run: &Run, agent: Option<&str>) {
    let Some(index) = run.current_index() else {
        println!(
            "`{}` has no stages left. Close it with `flow finish`.",
            run.meta.slug
        );
        return;
    };
    let stage = &flow.stages[index];
    let total = flow.stages.len();

    println!("{} — stage {} of {total}", run.meta.title, index + 1);
    println!("\n  {}\n  {}", stage.name, stage.description);

    let command = stage.command_for(agent);
    if !command.is_empty() {
        println!("\n  run: {command}");
    }
    if let Some(artifact) = stage.artifact_for(&run.meta.slug) {
        println!("  produces: {artifact}");
    }

    if !run.handoff.trim().is_empty() {
        println!("\n## Where we are\n\n{}", run.handoff.trim());
    }
}
