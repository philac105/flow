use crate::flow::{flow_dir, Flow};
use crate::run::{self, now, Run, StageStatus};
use anyhow::Result;
use std::path::{Path, PathBuf};

const BOARD_TEMPLATE: &str = include_str!("../../assets/board.html");

pub fn status(root: &Path, all: bool) -> Result<()> {
    let flow = Flow::load(root)?;
    let runs = visible(root, all)?;

    if runs.is_empty() {
        println!(
            "no {}runs yet — start one with `flow start \"<title>\"`",
            if all { "" } else { "active " }
        );
        return Ok(());
    }

    let rows: Vec<[String; 4]> = runs
        .iter()
        .map(|r| {
            let (done, total) = r.progress();
            let stage = if r.is_finished() {
                "finished".to_string()
            } else {
                r.current_stage_name().unwrap_or("—").to_string()
            };
            [
                r.meta.slug.clone(),
                if r.meta.kind.is_empty() {
                    "—".into()
                } else {
                    r.meta.kind.clone()
                },
                stage,
                format!("{done}/{total}"),
            ]
        })
        .collect();

    let headers = ["RUN", "KIND", "STAGE", "DONE"];
    let widths: Vec<usize> = (0..4)
        .map(|i| {
            rows.iter()
                .map(|r| r[i].chars().count())
                .chain(std::iter::once(headers[i].len()))
                .max()
                .unwrap_or(0)
        })
        .collect();

    print_row(&headers.map(String::from), &widths);
    for row in &rows {
        print_row(row, &widths);
    }

    let drifting: Vec<(String, String)> = runs
        .iter()
        .flat_map(|r| {
            r.drift(&flow, root)
                .into_iter()
                .map(move |d| (r.meta.slug.clone(), format!("{}: {}", d.stage, d.message)))
        })
        .collect();
    if !drifting.is_empty() {
        println!("\ndrift:");
        for (slug, message) in drifting {
            println!("  ! {slug} — {message}");
        }
    }
    Ok(())
}

fn print_row(cells: &[String; 4], widths: &[usize]) {
    let line: Vec<String> = cells
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{:<width$}", c, width = widths[i]))
        .collect();
    println!("{}", line.join("  ").trim_end());
}

pub fn board(root: &Path, output: Option<PathBuf>, all: bool) -> Result<()> {
    let flow = Flow::load(root)?;
    let runs = visible(root, all)?;

    let body = if runs.is_empty() {
        "<p class=\"empty\">No runs yet. Start one with <code>flow start \"&lt;title&gt;\"</code>.</p>"
            .to_string()
    } else {
        let cards: Vec<String> = runs.iter().map(|r| card(&flow, r, root)).collect();
        format!("<div class=\"grid\">\n{}\n</div>", cards.join("\n"))
    };

    let active = runs.iter().filter(|r| !r.is_finished()).count();
    let subtitle = format!(
        "{} run{} · flow <code>{}</code> · {active} active",
        runs.len(),
        if runs.len() == 1 { "" } else { "s" },
        escape(&flow.name)
    );

    let html = BOARD_TEMPLATE
        .replace("__TITLE__", &escape(&title_for(root)))
        .replace("__SUBTITLE__", &subtitle)
        .replace("__BODY__", &body)
        .replace("__GENERATED__", &now());

    let path = output.unwrap_or_else(|| flow_dir(root).join("board.html"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, html)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn card(flow: &Flow, run: &Run, root: &Path) -> String {
    let current = run.current_index();
    let stages: Vec<String> = run
        .meta
        .stages
        .iter()
        .enumerate()
        .map(|(i, record)| {
            let class = match record.status {
                StageStatus::Done => "done",
                StageStatus::Skipped => "skipped",
                _ if Some(i) == current => "current",
                _ => "",
            };
            format!(
                "      <li class=\"{class}\"><span class=\"dot\"></span><span class=\"name\">{}</span></li>",
                escape(&record.name)
            )
        })
        .collect();

    let drift = run.drift(flow, root);
    let drift_html = if drift.is_empty() {
        String::new()
    } else {
        let items: Vec<String> = drift
            .iter()
            .map(|d| format!("{}: {}", escape(&d.stage), escape(&d.message)))
            .collect();
        format!("    <div class=\"drift\">{}</div>\n", items.join("<br>"))
    };

    let handoff = run.handoff.trim();
    let handoff_html = if handoff.is_empty() {
        String::new()
    } else {
        format!("    <div class=\"handoff\">{}</div>\n", escape(handoff))
    };

    let kind = if run.meta.kind.is_empty() {
        run.meta.flow.clone()
    } else {
        run.meta.kind.clone()
    };

    format!(
        "  <article class=\"card\">\n    <div class=\"kind\">{}</div>\n    \
         <div class=\"title\">{}</div>\n    <div class=\"slug\">{}</div>\n    \
         <ol class=\"stages\">\n{}\n    </ol>\n{}{}  </article>",
        escape(&kind),
        escape(&run.meta.title),
        escape(&run.meta.slug),
        stages.join("\n"),
        drift_html,
        handoff_html,
    )
}

fn visible(root: &Path, all: bool) -> Result<Vec<Run>> {
    let runs = run::load_all(root)?;
    Ok(if all {
        runs
    } else {
        runs.into_iter().filter(|r| !r.is_finished()).collect()
    })
}

fn title_for(root: &Path) -> String {
    root.canonicalize()
        .ok()
        .as_deref()
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "flow".into())
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
