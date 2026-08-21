use crate::flow::{Flow, Launcher};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

/// Settings that belong to the person, not the project: which agent they drive
/// and how it starts on their machine. Kept out of the repo so that committing
/// a flow shares the process without shipping anyone's tooling.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub agent: String,
    /// Which built-in flow a bare `flow init` writes.
    #[serde(default)]
    pub preset: String,
    #[serde(default)]
    pub agents: BTreeMap<String, Launcher>,
}

/// Where a setting came from, so `flow config` can explain itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Flag,
    Repo,
    User,
    None,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Source::Flag => "--agent flag",
            Source::Repo => ".flow/flow.toml",
            Source::User => "user config",
            Source::None => "unset",
        })
    }
}

/// `$XDG_CONFIG_HOME/flow/config.toml`, falling back to `~/.config`.
pub fn user_config_path() -> Option<PathBuf> {
    Some(user_dir()?.join("config.toml"))
}

/// `$XDG_CONFIG_HOME/flow/presets/` — the user layer of the Preset Path,
/// resolved the same way the config file beside it is. `flow` only ever reads
/// it (ADR-0008).
pub fn user_presets_dir() -> Option<PathBuf> {
    Some(user_dir()?.join("presets"))
}

/// `$XDG_CONFIG_HOME/flow`, falling back to `~/.config/flow`.
fn user_dir() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(base.join("flow"))
}

impl UserConfig {
    pub fn load() -> Result<(UserConfig, Option<PathBuf>)> {
        let Some(path) = user_config_path() else {
            return Ok((UserConfig::default(), None));
        };
        if !path.is_file() {
            return Ok((UserConfig::default(), Some(path)));
        }
        let text = std::fs::read_to_string(&path)?;
        let config: UserConfig = toml::from_str(&text)
            .map_err(|e| anyhow!("could not parse {}: {e}", path.display()))?;
        Ok((config, Some(path)))
    }
}

/// The launchers and default agent, after layering the repo's flow over the
/// user's config. A repo may override by name when it genuinely needs to; the
/// preset does not, so the usual case is that agents live only with the user.
pub struct Settings {
    pub agents: BTreeMap<String, (Launcher, Source)>,
    pub agent: String,
    pub agent_source: Source,
    pub user_path: Option<PathBuf>,
    pub user_exists: bool,
}

impl Settings {
    pub fn resolve(flow: &Flow, flag: Option<&str>) -> Result<Settings> {
        let (user, user_path) = UserConfig::load()?;
        let user_exists = user_path.as_ref().is_some_and(|p| p.is_file());

        let mut agents: BTreeMap<String, (Launcher, Source)> = user
            .agents
            .into_iter()
            .map(|(k, v)| (k, (v, Source::User)))
            .collect();
        for (name, launcher) in &flow.agents {
            agents.insert(name.clone(), (launcher.clone(), Source::Repo));
        }

        // A named agent wins over the repo's default, which wins over the
        // user's, which wins over the only one that exists.
        let (agent, agent_source) = if let Some(flag) = flag {
            (flag.to_string(), Source::Flag)
        } else if !flow.agent.is_empty() {
            (flow.agent.clone(), Source::Repo)
        } else if !user.agent.is_empty() {
            (user.agent.clone(), Source::User)
        } else if agents.len() == 1 {
            let only = agents.keys().next().unwrap().clone();
            let source = agents[&only].1.clone();
            (only, source)
        } else {
            (String::new(), Source::None)
        };

        Ok(Settings {
            agents,
            agent,
            agent_source,
            user_path,
            user_exists,
        })
    }

    pub fn launcher(&self) -> Result<(&str, &Launcher)> {
        if self.agent.is_empty() {
            return Err(anyhow!(
                "no agent configured — run `flow config --init` and set one up"
            ));
        }
        match self.agents.get_key_value(self.agent.as_str()) {
            Some((name, (launcher, _))) if !launcher.command.is_empty() => {
                Ok((name.as_str(), launcher))
            }
            Some((name, _)) => Err(anyhow!("agent `{name}` declares an empty command")),
            None => {
                let known: Vec<&str> = self.agents.keys().map(String::as_str).collect();
                Err(anyhow!(
                    "no agent called `{}` — configured agents: {}. See `flow config`.",
                    self.agent,
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
