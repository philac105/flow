use crate::flow::{is_tracker_artifact, runs_dir, Flow};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

pub const HANDOFF_HEADING: &str = "## Where we are";
pub const LOG_HEADING: &str = "## Log";
const FRONTMATTER_FENCE: &str = "+++";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Pending,
    InProgress,
    Done,
    Skipped,
}

impl fmt::Display for StageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StageStatus::Pending => "pending",
            StageStatus::InProgress => "in progress",
            StageStatus::Done => "done",
            StageStatus::Skipped => "skipped",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Active,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRecord {
    pub name: String,
    pub status: StageStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<String>,
}

/// The frontmatter of a run file. Field order matters: `toml` requires scalars
/// before tables, so `stages` stays last.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMeta {
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub kind: String,
    pub flow: String,
    pub status: RunStatus,
    pub created: String,
    pub updated: String,
    #[serde(default, rename = "stage")]
    pub stages: Vec<StageRecord>,
}

/// One traversal of the flow by one piece of work.
#[derive(Debug, Clone)]
pub struct Run {
    pub meta: RunMeta,
    /// The always-current block. Rewritten on every transition, never appended to.
    pub handoff: String,
    /// Append-only history. Never rewritten.
    pub log: String,
    pub path: PathBuf,
}

/// A stage whose recorded status disagrees with its artifact on disk.
pub struct Drift {
    pub stage: String,
    pub message: String,
}

impl Run {
    pub fn path_for(root: &Path, slug: &str) -> PathBuf {
        runs_dir(root).join(format!("{slug}.md"))
    }

    pub fn new(slug: String, title: String, kind: String, flow: &Flow) -> Run {
        let now = now();
        let stages: Vec<StageRecord> = flow
            .stages
            .iter()
            .enumerate()
            .map(|(i, s)| StageRecord {
                name: s.name.clone(),
                status: if i == 0 {
                    StageStatus::InProgress
                } else {
                    StageStatus::Pending
                },
                artifact: None,
                started: if i == 0 { Some(now.clone()) } else { None },
                completed: None,
            })
            .collect();
        Run {
            meta: RunMeta {
                slug,
                title,
                kind,
                flow: flow.name.clone(),
                status: RunStatus::Active,
                created: now.clone(),
                updated: now,
                stages,
            },
            handoff: String::new(),
            log: String::new(),
            path: PathBuf::new(),
        }
    }

    /// The index of the stage this run is on: the first in-progress stage, else
    /// the first pending one. `None` once every stage is done or skipped.
    pub fn current_index(&self) -> Option<usize> {
        self.meta
            .stages
            .iter()
            .position(|s| s.status == StageStatus::InProgress)
            .or_else(|| {
                self.meta
                    .stages
                    .iter()
                    .position(|s| s.status == StageStatus::Pending)
            })
    }

    pub fn current_stage_name(&self) -> Option<&str> {
        self.current_index().map(|i| self.meta.stages[i].name.as_str())
    }

    /// Stages settled one way or another, over the total. Used for `3/5`.
    pub fn progress(&self) -> (usize, usize) {
        let done = self
            .meta
            .stages
            .iter()
            .filter(|s| matches!(s.status, StageStatus::Done | StageStatus::Skipped))
            .count();
        (done, self.meta.stages.len())
    }

    pub fn is_finished(&self) -> bool {
        self.meta.status == RunStatus::Finished
    }

    /// Compare each stage's recorded status against whether its declared
    /// artifact exists. Reported, never resolved — see ADR-0002.
    pub fn drift(&self, flow: &Flow, root: &Path) -> Vec<Drift> {
        let mut out = Vec::new();
        for record in &self.meta.stages {
            let Some(stage) = flow.stages.iter().find(|s| s.name == record.name) else {
                continue;
            };
            // A recorded artifact is what actually happened; the stage's
            // declaration is only the expectation.
            let artifact = record
                .artifact
                .clone()
                .or_else(|| stage.artifact_for(&self.meta.slug));
            let Some(artifact) = artifact else { continue };
            if is_tracker_artifact(&artifact) {
                continue;
            }
            let exists = root.join(&artifact).exists();
            match (record.status, exists) {
                (StageStatus::Done, false) => out.push(Drift {
                    stage: record.name.clone(),
                    message: format!("marked done but {artifact} is missing"),
                }),
                (StageStatus::Pending | StageStatus::InProgress, true) => out.push(Drift {
                    stage: record.name.clone(),
                    message: format!(
                        "{artifact} exists but the stage is {} — did a session die here?",
                        record.status
                    ),
                }),
                _ => {}
            }
        }
        out
    }

    pub fn load(path: &Path) -> Result<Run> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("could not read {}", path.display()))?;
        let mut run = Run::parse(&text)
            .with_context(|| format!("could not parse {}", path.display()))?;
        run.path = path.to_path_buf();
        Ok(run)
    }

    pub fn parse(text: &str) -> Result<Run> {
        let rest = text
            .strip_prefix(FRONTMATTER_FENCE)
            .ok_or_else(|| anyhow!("missing opening `{FRONTMATTER_FENCE}` frontmatter fence"))?;
        let rest = rest.trim_start_matches(['\r', '\n']);
        let end = rest
            .find(&format!("\n{FRONTMATTER_FENCE}"))
            .ok_or_else(|| anyhow!("missing closing `{FRONTMATTER_FENCE}` frontmatter fence"))?;
        let frontmatter = &rest[..end];
        let body = rest[end + 1 + FRONTMATTER_FENCE.len()..].trim_start_matches(['\r', '\n']);
        let meta: RunMeta = toml::from_str(frontmatter)?;
        let (handoff, log) = split_body(body);
        Ok(Run {
            meta,
            handoff,
            log,
            path: PathBuf::new(),
        })
    }

    pub fn render(&self) -> Result<String> {
        let frontmatter = toml::to_string_pretty(&self.meta)?;
        let handoff = self.handoff.trim();
        let log = self.log.trim_end();
        Ok(format!(
            "{FRONTMATTER_FENCE}\n{frontmatter}{FRONTMATTER_FENCE}\n\n\
             {HANDOFF_HEADING}\n\n{handoff}\n\n{LOG_HEADING}\n\n{log}\n",
        ))
    }

    pub fn save(&mut self) -> Result<()> {
        self.meta.updated = now();
        let rendered = self.render()?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, rendered)
            .with_context(|| format!("could not write {}", self.path.display()))
    }

    /// Replace the handoff wholesale; append one entry to the log. The two are
    /// deliberately different operations — the handoff says where we are, the
    /// log says how we got here.
    pub fn record(&mut self, headline: &str, note: Option<&str>) {
        let note = note.unwrap_or("").trim();
        self.handoff = if note.is_empty() {
            headline.to_string()
        } else {
            format!("{headline}\n\n{note}")
        };
        let stamp = now();
        let mut entry = format!("### {}Z — {headline}\n", to_seconds(&stamp));
        if !note.is_empty() {
            entry.push('\n');
            entry.push_str(note);
            entry.push('\n');
        }
        if self.log.trim().is_empty() {
            self.log = entry;
        } else {
            self.log = format!("{}\n\n{}", self.log.trim_end(), entry);
        }
    }
}

/// Split a run body into its handoff block and its log. Anything before the
/// handoff heading is discarded; anything after the log heading is kept verbatim.
fn split_body(body: &str) -> (String, String) {
    let (before_log, log) = match body.find(LOG_HEADING) {
        Some(i) => (&body[..i], body[i + LOG_HEADING.len()..].trim().to_string()),
        None => (body, String::new()),
    };
    let handoff = match before_log.find(HANDOFF_HEADING) {
        Some(i) => before_log[i + HANDOFF_HEADING.len()..].trim().to_string(),
        None => before_log.trim().to_string(),
    };
    (handoff, log)
}

/// Millisecond precision, because runs updated within the same second must
/// still sort in the order they happened.
pub fn now() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

/// The same instant trimmed to seconds, for log headings a human reads.
fn to_seconds(stamp: &str) -> &str {
    match stamp.find('.') {
        Some(i) => &stamp[..i],
        None => stamp.trim_end_matches('Z'),
    }
}

/// Lowercase, non-alphanumerics collapsed to single hyphens, trimmed.
pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut pending_sep = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_sep && !out.is_empty() {
                out.push('-');
            }
            pending_sep = false;
            out.push(ch.to_ascii_lowercase());
        } else {
            pending_sep = true;
        }
    }
    out
}

/// Every run in the repo, most recently updated first.
pub fn load_all(root: &Path) -> Result<Vec<Run>> {
    let dir = runs_dir(root);
    let mut runs = Vec::new();
    if !dir.is_dir() {
        return Ok(runs);
    }
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            runs.push(Run::load(&path)?);
        }
    }
    runs.sort_by(|a, b| b.meta.updated.cmp(&a.meta.updated));
    Ok(runs)
}

/// Resolve a run by slug, or fall back to the only active run when the repo has
/// exactly one — so `flow next` needs no argument in the common case.
pub fn resolve(root: &Path, slug: Option<&str>) -> Result<Run> {
    if let Some(slug) = slug {
        let path = Run::path_for(root, slug);
        if !path.exists() {
            return Err(anyhow!("no run called `{slug}` — try `flow status`"));
        }
        return Run::load(&path);
    }
    let mut active: Vec<Run> = load_all(root)?.into_iter().filter(|r| !r.is_finished()).collect();
    match active.len() {
        0 => Err(anyhow!("no active runs — start one with `flow start \"<title>\"`")),
        1 => Ok(active.remove(0)),
        _ => {
            let names: Vec<&str> = active.iter().map(|r| r.meta.slug.as_str()).collect();
            Err(anyhow!(
                "several active runs — name one of: {}",
                names.join(", ")
            ))
        }
    }
}
