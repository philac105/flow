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
    /// Which launcher `flow go` uses when none is named on the command line.
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub agents: BTreeMap<String, Launcher>,
    #[serde(default, rename = "stage")]
    pub stages: Vec<Stage>,
}

/// How to start an agent. Declared in `.flow/flow.toml`, never compiled in —
/// the binary substitutes into an argv it was handed and knows nothing about
/// which agent is on the other end. See ADR-0006.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Launcher {
    /// argv to spawn. `{prompt}`, `{slug}` and `{stage}` are substituted.
    pub command: Vec<String>,
    /// Environment variables whose presence means an agent is already running
    /// here. `flow go` refuses rather than nesting a session inside a session.
    #[serde(default)]
    pub guard_env: Vec<String>,
}

impl Flow {
    /// Resolve a launcher by name, falling back to the flow's default and then
    /// to the only one declared.
    pub fn launcher(&self, name: Option<&str>) -> Result<(&str, &Launcher)> {
        let chosen = name
            .map(str::to_string)
            .or_else(|| (!self.agent.is_empty()).then(|| self.agent.clone()))
            .or_else(|| {
                (self.agents.len() == 1).then(|| self.agents.keys().next().unwrap().clone())
            });

        let Some(chosen) = chosen else {
            return Err(anyhow!(
                "no agent configured — add an [agents.<name>] table to .flow/flow.toml"
            ));
        };
        match self.agents.get_key_value(chosen.as_str()) {
            Some((name, launcher)) if !launcher.command.is_empty() => Ok((name.as_str(), launcher)),
            Some((name, _)) => Err(anyhow!("agent `{name}` declares an empty command")),
            None => {
                let known: Vec<&str> = self.agents.keys().map(String::as_str).collect();
                Err(anyhow!(
                    "no agent called `{chosen}` — .flow/flow.toml declares: {}",
                    if known.is_empty() {
                        "none".into()
                    } else {
                        known.join(", ")
                    }
                ))
            }
        }
    }
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
