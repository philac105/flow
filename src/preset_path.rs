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

/// A preset that won its name, and whatever it beat.
pub struct Preset {
    pub name: String,
    pub description: String,
    /// Read here, written out by `init` — which starts resolving through this
    /// module in the ticket after this one.
    #[allow(dead_code)]
    pub contents: String,
    pub layer: Layer,
    /// The layers that carry this name too but were farther away, nearest
    /// first. Empty for almost every preset.
    pub shadowed: Vec<Layer>,
}

/// Every preset reachable from `start`, by name. One entry per name: the
/// nearest owner of it, carrying the layers it shadowed.
pub fn discover(start: &Path) -> Vec<Preset> {
    let mut by_name: BTreeMap<String, Preset> = BTreeMap::new();

    for (layer, name, description, contents) in candidates(start) {
        match by_name.get_mut(&name) {
            // Already claimed by a nearer owner: this one is shadowed, and says
            // so rather than vanishing.
            Some(winner) => winner.shadowed.push(layer),
            None => {
                by_name.insert(
                    name.clone(),
                    Preset {
                        name,
                        description,
                        contents,
                        layer,
                        shadowed: Vec::new(),
                    },
                );
            }
        }
    }

    by_name.into_values().collect()
}

/// The presets directories the project layer reads, nearest ancestor first.
///
/// Deliberately not `find_root`: that search stops at the first ancestor with a
/// `.flow`, so a package that has already run `init` would never see the repo
/// root's menu — gutting the only case the project layer exists for. This is a
/// second traversal of the same ancestry, and `find_root` keeps its behaviour.
pub fn project_dirs(start: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut cur = Some(start);
    while let Some(dir) = cur {
        dirs.push(flow_dir(dir).join("presets"));
        cur = dir.parent();
    }
    dirs
}

/// Every preset file on the Path, nearest owner first — so the first entry for
/// a name is the one that wins.
fn candidates(start: &Path) -> Vec<(Layer, String, String, String)> {
    let mut found = Vec::new();

    for dir in project_dirs(start) {
        let layer = Layer::Project(dir.clone());
        read_dir(&dir, &layer, &mut found);
    }
    if let Some(dir) = user_presets_dir() {
        read_dir(&dir, &Layer::User, &mut found);
    }
    for preset in crate::presets::SHIPPED {
        found.push((
            Layer::Shipped,
            preset.name.to_string(),
            preset.description.to_string(),
            preset.contents.to_string(),
        ));
    }

    found
}

/// Every readable preset in one directory, in filename order. A directory that
/// is absent or unreadable is not an error — most of them will not exist.
fn read_dir(dir: &Path, layer: &Layer, found: &mut Vec<(Layer, String, String, String)>) {
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
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        // A file we cannot make sense of is skipped, never fatal: one bad
        // preset must not stop the repo you are actually initialising.
        if let Ok(flow) = parse(stem, &text) {
            found.push((layer.clone(), stem.to_string(), flow.description, text));
        }
    }
}

/// A preset file, or why it is not one.
fn parse(stem: &str, text: &str) -> Result<Flow, String> {
    let flow: Flow = toml::from_str(text).map_err(|e| e.to_string())?;
    if flow.stages.is_empty() {
        return Err("declares no stages".to_string());
    }
    crate::preset_name::check(stem, &flow.name)?;
    Ok(flow)
}
