use crate::flow::Flow;
use crate::prompt;
use crate::run::{self, Run, RunStatus, StageStatus};
use anyhow::{anyhow, Result};
use std::path::Path;

/// Begin a run. Every argument is optional at the command line and filled in
/// by asking, so `flow start` on its own is a complete way in. Nothing is asked
/// unless a person is there to answer — see [`crate::prompt`].
pub fn start(
    root: &Path,
    title: Option<&str>,
    kind: Option<&str>,
    brief: Option<&str>,
) -> Result<()> {
    let flow = Flow::load(root)?;
    let asking = prompt::interactive();

    let title = match title {
        Some(title) => title.to_string(),
        None if asking => prompt::line("title")?,
        None => return Err(anyhow!("no title — `flow start \"<title>\"`")),
    };
    if title.trim().is_empty() {
        return Err(anyhow!("no title — `flow start \"<title>\"`"));
    }

    let slug = run::slugify(&title);
    if slug.is_empty() {
        return Err(anyhow!(
            "`{title}` has no characters a slug can be made from"
        ));
    }

    // Everything that can refuse the run refuses it before the questions, so a
    // brief someone has just typed is never thrown away.
    let path = Run::path_for(root, &slug);
    if path.exists() {
        return Err(anyhow!(
            "a run called `{slug}` already exists at {}",
            path.display()
        ));
    }

    let kind = match kind {
        Some(kind) => kind.to_string(),
        None if asking => prompt::line("kind (feature, bug, task — enter to skip)")?,
        None => String::new(),
    };
    let brief = match brief {
        Some(brief) => brief.to_string(),
        None if asking => prompt::paragraph(&format!(
            "brief — what this work is, for whoever picks up `{}` cold (blank line to finish)",
            flow.stages[0].name
        ))?,
        None => String::new(),
    };

    let mut run = Run::new(slug.clone(), title.clone(), kind, &flow);
    run.path = path;
    let first = &flow.stages[0];
    // The brief is the whole reason the first stage has anything to work with.
    // Without it a cold agent gets a slug and nothing else, so say so plainly
    // rather than opening with boilerplate that reads like context.
    let opening = match brief.trim() {
        "" => "No brief was given — the title is all there is. Find out what this work \
               actually is, and who wants it, before doing the stage."
            .to_string(),
        brief => brief.to_string(),
    };
    run.record(
        &format!("Started. First stage is `{}`.", first.name),
        Some(&format!(
            "{opening}\n\n{}\n\nRun `{}` next.",
            first.description, first.command
        )),
    );
    run.save()?;
    // Starting something is the clearest possible statement of what you are
    // working on, so it becomes current.
    run::set_current(root, &slug)?;

    if asking {
        println!();
    }
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
    // A finished run cannot be current: `resolve` steps over one, so the
    // pointer would say `beta` while the next `flow done` wrote into `alpha`.
    if run.is_finished() {
        return Err(anyhow!(
            "`{}` is finished — bring it back with `flow reopen {}` instead",
            run.meta.slug,
            run.meta.slug
        ));
    }
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
    // Same reasoning as `start`: picking a run back up says which one you are
    // on. Without this the pointer stays on whatever else is active, and the
    // next bare `flow done` records against that instead.
    run::set_current(root, &run.meta.slug)?;

    println!("reopened `{}` (now current)\n", run.meta.slug);
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
