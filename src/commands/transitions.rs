use crate::commands::lifecycle::print_next;
use crate::flow::Flow;
use crate::run::{self, now, Run, StageStatus};
use anyhow::{anyhow, Result};
use std::path::Path;

pub fn next(root: &Path, slug: Option<&str>, agent: Option<&str>) -> Result<()> {
    let flow = Flow::load(root)?;
    let run = run::resolve(root, slug)?;

    // Drift first: a session that died mid-stage should say so before anyone
    // acts on a position that may be a lie.
    let drift = run.drift(&flow, root);
    if !drift.is_empty() {
        println!("drift — resolve this before continuing:");
        for d in &drift {
            println!("  ! {}: {}", d.stage, d.message);
        }
        println!();
    }

    print_next(&flow, &run, agent);
    Ok(())
}

pub fn done(
    root: &Path,
    slug: Option<&str>,
    message: Option<&str>,
    artifact: Option<&str>,
) -> Result<()> {
    let flow = Flow::load(root)?;
    let mut run = run::resolve(root, slug)?;
    let index = current_or_err(&run)?;

    let stamp = now();
    {
        let record = &mut run.meta.stages[index];
        record.status = StageStatus::Done;
        record.completed = Some(stamp.clone());
        if let Some(artifact) = artifact {
            record.artifact = Some(artifact.to_string());
        }
    }
    let finished_name = run.meta.stages[index].name.clone();

    let headline = match advance(&mut run, index, &stamp) {
        Some(next_name) => format!("`{finished_name}` done → `{next_name}`"),
        None => format!("`{finished_name}` done — every stage settled"),
    };
    run.record(&headline, message);
    run.save()?;

    println!("{headline}\n");
    print_next(&flow, &run, None);
    Ok(())
}

pub fn skip(root: &Path, slug: Option<&str>, message: Option<&str>) -> Result<()> {
    let flow = Flow::load(root)?;
    let mut run = run::resolve(root, slug)?;
    let index = current_or_err(&run)?;
    let name = run.meta.stages[index].name.clone();

    let stage = flow
        .stages
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| anyhow!("`{name}` is not a stage of flow `{}`", flow.name))?;
    if !stage.optional {
        return Err(anyhow!(
            "`{name}` is not optional — mark it optional in .flow/flow.toml, or use `flow done`"
        ));
    }

    let stamp = now();
    {
        let record = &mut run.meta.stages[index];
        record.status = StageStatus::Skipped;
        record.completed = Some(stamp.clone());
    }

    let headline = match advance(&mut run, index, &stamp) {
        Some(next_name) => format!("`{name}` skipped → `{next_name}`"),
        None => format!("`{name}` skipped — every stage settled"),
    };
    run.record(&headline, message);
    run.save()?;

    println!("{headline}\n");
    print_next(&flow, &run, None);
    Ok(())
}

pub fn back(
    root: &Path,
    slug: Option<&str>,
    stage: Option<&str>,
    message: Option<&str>,
) -> Result<()> {
    let flow = Flow::load(root)?;
    let mut run = run::resolve(root, slug)?;

    // Without a current stage the run has run off the end, so "back" means back
    // into the last stage there is.
    let anchor = run.current_index().unwrap_or(run.meta.stages.len());
    let target = match stage {
        Some(name) => run
            .meta
            .stages
            .iter()
            .position(|s| s.name == name)
            .ok_or_else(|| anyhow!("`{name}` is not a stage of this run"))?,
        None => anchor
            .checked_sub(1)
            .ok_or_else(|| anyhow!("`{}` is already at its first stage", run.meta.slug))?,
    };
    if target >= anchor {
        return Err(anyhow!(
            "`{}` is not behind the current stage — `flow back` only moves backwards",
            run.meta.stages[target].name
        ));
    }

    let name = run.meta.stages[target].name.clone();
    let stamp = now();
    // Everything from the target onwards is unsettled again. The log keeps the
    // record that it once wasn't.
    for record in run.meta.stages.iter_mut().skip(target) {
        record.status = StageStatus::Pending;
        record.completed = None;
    }
    {
        let record = &mut run.meta.stages[target];
        record.status = StageStatus::InProgress;
        record.started = Some(stamp);
    }

    let headline = format!("sent back to `{name}`");
    run.record(&headline, message);
    run.save()?;

    println!("{headline}\n");
    print_next(&flow, &run, None);
    Ok(())
}

fn current_or_err(run: &Run) -> Result<usize> {
    run.current_index().ok_or_else(|| {
        anyhow!(
            "`{}` has no stage in flight — every stage is settled. Use `flow finish` or `flow back`.",
            run.meta.slug
        )
    })
}

/// Open the next unsettled stage after `index`, returning its name.
fn advance(run: &mut Run, index: usize, stamp: &str) -> Option<String> {
    let next = run
        .meta
        .stages
        .iter()
        .skip(index + 1)
        .position(|s| matches!(s.status, StageStatus::Pending | StageStatus::InProgress))
        .map(|offset| index + 1 + offset)?;
    let record = &mut run.meta.stages[next];
    record.status = StageStatus::InProgress;
    record.started.get_or_insert_with(|| stamp.to_string());
    Some(record.name.clone())
}
