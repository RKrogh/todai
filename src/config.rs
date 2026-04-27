use anyhow::{anyhow, Context as _, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_RELATIVE_PATH: &str = ".todai/config.toml";
pub const STIGNORE_RELATIVE_PATH: &str = ".stignore";
pub const TODAI_HOME_ENV: &str = "TODAI_HOME";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub contexts: Contexts,
    pub notifications: Notifications,
    pub ai: Ai,
    pub logging: Logging,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub default_context: String,
    pub editor: Option<String>,
    pub archive_done: bool,
    pub archive_after_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Contexts {
    pub allowed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Notifications {
    pub method: String,
    pub homeassistant_url: String,
    pub homeassistant_token_env: String,
    pub device_name: String,
    pub default_notify_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Ai {
    pub provider: String,
    pub model: String,
    pub api_key_env: String,
    pub morning_briefing: String,
    pub evening_check: String,
    pub hourly_scan: bool,
    pub hourly_scan_range: String,
    pub exclude_contexts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Logging {
    pub path: String,
    pub retain_runs: u32,
    pub retain_days: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: General::default(),
            contexts: Contexts::default(),
            notifications: Notifications::default(),
            ai: Ai::default(),
            logging: Logging::default(),
        }
    }
}

impl Default for General {
    fn default() -> Self {
        Self {
            default_context: "private:inbox".into(),
            editor: None,
            archive_done: true,
            archive_after_days: 7,
        }
    }
}

impl Default for Contexts {
    fn default() -> Self {
        Self { allowed: vec![] }
    }
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            method: "homeassistant".into(),
            homeassistant_url: "http://homeassistant.local:8123".into(),
            homeassistant_token_env: "HA_TOKEN".into(),
            device_name: "robert_phone".into(),
            default_notify_mode: "smart".into(),
        }
    }
}

impl Default for Ai {
    fn default() -> Self {
        Self {
            provider: "anthropic".into(),
            model: "claude-sonnet-4-6".into(),
            api_key_env: "ANTHROPIC_API_KEY".into(),
            morning_briefing: "07:00".into(),
            evening_check: "18:00".into(),
            hourly_scan: true,
            hourly_scan_range: "08:00-17:00".into(),
            exclude_contexts: vec![],
        }
    }
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            path: ".todai/logs/agent.log".into(),
            retain_runs: 100,
            retain_days: 14,
        }
    }
}

pub fn resolve_root(cli_override: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = cli_override {
        return Ok(p.to_path_buf());
    }
    if let Ok(env) = std::env::var(TODAI_HOME_ENV) {
        if !env.is_empty() {
            return Ok(PathBuf::from(env));
        }
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))?;
    Ok(home.join(".todai"))
}

pub fn load(root: &Path) -> Result<Config> {
    let path = root.join(CONFIG_RELATIVE_PATH);
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    let cfg: Config = toml::from_str(&raw)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(cfg)
}

pub fn write_default(root: &Path) -> Result<PathBuf> {
    let path = root.join(CONFIG_RELATIVE_PATH);
    if path.exists() {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, default_config_toml())?;
    Ok(path)
}

pub fn write_stignore(root: &Path) -> Result<PathBuf> {
    let path = root.join(STIGNORE_RELATIVE_PATH);
    if path.exists() {
        return Ok(path);
    }
    fs::write(&path, default_stignore())?;
    Ok(path)
}

fn default_config_toml() -> &'static str {
    r#"# todai configuration. Edit and re-run any todai command to apply.

[general]
default_context = "private:inbox"
# editor = "nvim"          # uncomment to override $EDITOR
archive_done = true
archive_after_days = 7

[contexts]
# Optional whitelist for autocomplete + validation. Empty = anything goes.
allowed = []

[notifications]
method = "homeassistant"          # homeassistant | ntfy | email | none
homeassistant_url = "http://homeassistant.local:8123"
homeassistant_token_env = "HA_TOKEN"
device_name = "robert_phone"
default_notify_mode = "smart"     # smart | dumb (per-reminder override wins)

[ai]
provider = "anthropic"            # anthropic | openai | ollama (future)
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY" # env var the agent reads; never store the key here
morning_briefing = "07:00"
evening_check = "18:00"
hourly_scan = true
hourly_scan_range = "08:00-17:00"
# Contexts here are NEVER sent to the AI. Reminders fall back to dumb mode.
exclude_contexts = []

[logging]
path = ".todai/logs/agent.log"
retain_runs = 100
retain_days = 14
"#
}

fn default_stignore() -> &'static str {
    r#"// Syncthing ignore patterns for the todai store.
// See https://docs.syncthing.net/users/ignoring.html

.archive/**
.todai/logs/**
*.sync-conflict-*
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip() {
        let cfg = Config::default();
        let serialized = toml::to_string(&cfg).unwrap();
        let parsed: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(parsed.general.default_context, cfg.general.default_context);
        assert_eq!(parsed.notifications.default_notify_mode, "smart");
    }

    #[test]
    fn missing_config_yields_defaults() {
        let dir = std::env::temp_dir().join(format!("todai-cfg-{}", nanoid::nanoid!(6)));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = load(&dir).unwrap();
        assert_eq!(cfg.ai.model, "claude-sonnet-4-6");
        std::fs::remove_dir_all(&dir).ok();
    }
}
