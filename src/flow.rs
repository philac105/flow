use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A flow definition: an ordered, linear sequence of stages.
///
/// Loaded from `.flow/flow.toml` in the repo. Nothing in this crate may
/// special-case a stage by name — see ADR-0003.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "stage")]
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub repeatable: bool,
    /// Per-agent overrides for `command`. Absent for almost every stage.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agents: BTreeMap<String, String>,
}

impl Stage {
    /// The command to show, honouring a per-agent override when one exists.
    pub fn command_for(&self, agent: Option<&str>) -> &str {
        agent
            .and_then(|a| self.agents.get(a))
            .map(String::as_str)
            .unwrap_or(&self.command)
    }

    /// The artifact this stage should leave behind, with `{slug}` resolved.
    /// `None` when the stage declares nothing.
    pub fn artifact_for(&self, slug: &str) -> Option<String> {
        let raw = self.artifact.as_deref()?;
        if raw.trim().is_empty() {
            return None;
        }
        Some(raw.replace("{slug}", slug))
    }
}

/// An artifact that lives on a tracker is recorded but never checked on disk.
pub fn is_tracker_artifact(artifact: &str) -> bool {
    artifact.starts_with("tracker:")
}

impl Flow {
    pub fn load(root: &Path) -> Result<Flow> {
        let path = flow_path(root);
        let text = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "no flow found at {} — run `flow init` first",
                path.display()
            )
        })?;
        let flow: Flow =
            toml::from_str(&text).with_context(|| format!("could not parse {}", path.display()))?;
        if flow.stages.is_empty() {
            return Err(anyhow!("{} declares no stages", path.display()));
        }
        Ok(flow)
    }
}

pub fn flow_dir(root: &Path) -> PathBuf {
    root.join(".flow")
}

pub fn flow_path(root: &Path) -> PathBuf {
    flow_dir(root).join("flow.toml")
}

pub fn runs_dir(root: &Path) -> PathBuf {
    flow_dir(root).join("runs")
}

/// Walk up from `start` looking for a `.flow` directory, so commands work from
/// anywhere inside the repo. Falls back to `start` when none is found, so that
/// `flow init` has somewhere to write.
pub fn find_root(start: &Path) -> PathBuf {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        if flow_dir(dir).is_dir() {
            return dir.to_path_buf();
        }
        cur = dir.parent();
    }
    start.to_path_buf()
}
