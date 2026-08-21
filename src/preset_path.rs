//! The Preset Path: the ordered list of places presets are read from, nearest
//! owner first — the project, then the user, then what ships in the binary.
//!
//! A nearer preset shadows a farther one of the same name, whole file for whole
//! file: no merging and no `extends` (ADR-0008). The shadowed entry survives in
//! what this module returns, because silent shadowing is how someone loses an
//! afternoon to a flow they do not recognise.
//!
//! `flow` only ever reads these directories. Nothing here writes.

use crate::config::user_presets_dir;
use crate::flow::{flow_dir, Flow};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// Which layer of the Preset Path a preset was found on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layer {
    /// `.flow/presets/` in the starting directory or one of its ancestors,
    /// which is named because inheritance from a parent must never be invisible.
    Project(PathBuf),
    /// `$XDG_CONFIG_HOME/flow/presets/`.
    User,
    /// Embedded in the binary at build time from `presets/`.
    Shipped,
}

impl Layer {
    /// The layer's name on its own, for a column that has to line up.
    pub fn label(&self) -> &'static str {
        match self {
            Layer::Project(_) => "project",
            Layer::User => "user",
            Layer::Shipped => "shipped",
        }
    }
}

impl fmt::Display for Layer {
    /// Named in full, so that two project layers are told apart by the ancestor
    /// they came from.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Layer::Project(dir) => write!(f, "project ({})", dir.display()),
            other => f.write_str(other.label()),
        }
    }
}

/// A file in a presets directory that is not a preset, and why. Never fatal:
/// a half-finished flow someone is drafting must not break the repo they are
/// actually initialising.
pub struct Skipped {
    pub path: PathBuf,
    /// A predicate, so that `<path> <reason>` reads as a sentence.
    pub reason: String,
}

/// Everything the Preset Path turned up: what you can init with, and what was
/// declined.
pub struct Discovered {
    pub presets: Vec<Preset>,
    pub skipped: Vec<Skipped>,
}

/// A preset that lost its name to a nearer one. It keeps its description,
/// because knowing *that* you overrode something is only half of knowing what
/// you overrode.
pub struct Shadowed {
    pub layer: Layer,
    pub description: String,
}

/// A preset that won its name, and whatever it beat.
pub struct Preset {
    pub name: String,
    pub description: String,
    /// The flow itself, which `init` writes out verbatim.
    pub contents: String,
    pub layer: Layer,
    /// The presets that carry this name too but were farther away, nearest
    /// first. Empty for almost every preset.
    pub shadowed: Vec<Shadowed>,
}

/// Every preset reachable from `start`, by name — one entry per name, the
/// nearest owner of it, carrying the layers it shadowed — plus the files that
/// were declined and why.
pub fn discover(start: &Path) -> Discovered {
    let mut by_name: BTreeMap<String, Preset> = BTreeMap::new();
    let (found, skipped) = candidates(start);

    for preset in found {
        match by_name.get_mut(&preset.name) {
            // Already claimed by a nearer owner: this one is shadowed, and says
            // so rather than vanishing.
            Some(winner) => winner.shadowed.push(Shadowed {
                layer: preset.layer,
                description: preset.description,
            }),
            None => {
                by_name.insert(preset.name.clone(), preset);
            }
        }
    }

    Discovered {
        presets: by_name.into_values().collect(),
        skipped,
    }
}

/// The presets directories the project layer reads, nearest ancestor first.
///
/// Deliberately not `find_root`: that search stops at the first ancestor with a
/// `.flow`, so a package that has already run `init` would never see the repo
/// root's menu — gutting the only case the project layer exists for. This is a
/// second traversal of the same ancestry, and `find_root` keeps its behaviour.
///
/// The walk stops at the `ceiling` below rather than at `/`.
///
/// `start` is expected absolute: the parent of a relative path is the empty
/// path, which reads as the process's working directory. `main` resolves it
/// once, for this walk and for `find_root` alike.
pub fn project_dirs(start: &Path) -> Vec<PathBuf> {
    let ceiling = ceiling(start);
    let mut dirs = Vec::new();
    let mut cur = Some(start);
    while let Some(dir) = cur {
        dirs.push(flow_dir(dir).join("presets"));
        if dir == ceiling {
            break;
        }
        cur = dir.parent();
    }
    dirs
}

/// How far up the project layer reaches: your repository, or your home
/// directory, whichever is farther — and the starting directory itself when
/// neither is on the ancestry.
///
/// Above those are directories nobody in particular owns — `/tmp`, `/`, a
/// shared mount — and a preset is not inert data: it carries the launcher argv
/// `flow go` spawns. Anyone able to write `/tmp/.flow/presets/main-flow.toml`
/// would otherwise change what a bare `flow init` writes for every repo beneath
/// it, beating what ships and looking exactly like the repo's own.
fn ceiling(start: &Path) -> PathBuf {
    // The outermost repository, not the nearest: a monorepo standardising the
    // flows its packages reach for is the case the project layer exists for,
    // and its root is above any submodule's.
    let mut repo = None;
    let mut cur = Some(start);
    while let Some(dir) = cur {
        // Only whether a repository is here — never anything inside it.
        if dir.join(".git").exists() {
            repo = Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| start.starts_with(home));

    match (repo, home) {
        // Both are ancestors of `start`, so one contains the other. Take the
        // farther, so that a `~/work/.flow/presets` still reaches the repos
        // under it.
        (Some(repo), Some(home)) if repo.starts_with(&home) => home,
        (Some(repo), _) => repo,
        (None, Some(home)) => home,
        (None, None) => start.to_path_buf(),
    }
}

/// Every preset file on the Path, nearest owner first — so the first entry for
/// a name is the one that wins — and every file declined along the way.
fn candidates(start: &Path) -> (Vec<Preset>, Vec<Skipped>) {
    let mut found = Vec::new();
    let mut skipped = Vec::new();

    for dir in project_dirs(start) {
        let layer = Layer::Project(dir.clone());
        read_dir(&dir, &layer, &mut found, &mut skipped);
    }
    if let Some(dir) = user_presets_dir() {
        read_dir(&dir, &Layer::User, &mut found, &mut skipped);
    }
    // Nothing shipped can be skipped: the build script already refused to
    // publish a preset that fails these checks.
    for preset in crate::presets::SHIPPED {
        found.push(Preset {
            name: preset.name.to_string(),
            description: preset.description.to_string(),
            contents: preset.contents.to_string(),
            layer: Layer::Shipped,
            shadowed: Vec::new(),
        });
    }

    (found, skipped)
}

/// Every preset in one directory, in filename order. A directory that is absent
/// or unreadable is not an error — most of them will not exist.
fn read_dir(dir: &Path, layer: &Layer, found: &mut Vec<Preset>, skipped: &mut Vec<Skipped>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        // A README or an editor swapfile in a presets directory is not a
        // problem, and must not be reported as one.
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();

    for path in paths {
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => {
                skipped.push(Skipped {
                    path,
                    reason: format!("could not be read: {e}"),
                });
                continue;
            }
        };
        match parse(stem, &text) {
            Ok(flow) => found.push(Preset {
                name: stem.to_string(),
                description: flow.description,
                contents: text,
                layer: layer.clone(),
                shadowed: Vec::new(),
            }),
            Err(reason) => skipped.push(Skipped { path, reason }),
        }
    }
}

/// A preset file, or why it is not one — written as a predicate, for whoever
/// has to fix the file.
fn parse(stem: &str, text: &str) -> Result<Flow, String> {
    // Told apart deliberately: TOML that will not lex is a different mistake
    // from TOML that lexes into something that is not a flow.
    let table = text
        .parse::<toml::Table>()
        .map_err(|e| format!("is not valid TOML: {}", one_line(&e.to_string())))?;
    // The runtime half of the rule the build script enforces fatally: we
    // control what ships, but a user's directory is theirs. Read off the table
    // the way the build script reads it, and before the flow is deserialised —
    // a missing `name` is a mistake with a name of its own, not a serde
    // complaint about a missing field.
    let declared = table.get("name").and_then(|value| value.as_str());
    crate::preset_name::check(stem, declared.unwrap_or_default())?;
    let flow: Flow = toml::from_str(text)
        .map_err(|e| format!("does not read as a flow: {}", one_line(&e.to_string())))?;
    if flow.stages.is_empty() {
        return Err("declares no stages".to_string());
    }
    Ok(flow)
}

/// `toml` reports an error as a location, an echo of the offending line, and a
/// caret under it. On one line the echo and the caret art are noise, so this
/// keeps the sentences and drops the drawing.
fn one_line(message: &str) -> String {
    message
        .lines()
        // The echo of the source line, which the reader has in front of them.
        .filter(|line| !line.trim_start().starts_with(|c: char| c.is_ascii_digit()))
        .map(|line| {
            line.trim()
                .trim_start_matches('|')
                .trim()
                .trim_start_matches('^')
                .trim()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}
