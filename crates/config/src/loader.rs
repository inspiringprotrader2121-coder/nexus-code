use crate::model::{ApprovalMode, Config};
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::{fs, path::PathBuf};

pub fn global_config_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "nexuscode", "nxc")
        .map(|d| d.config_dir().join("config.toml"))
}

pub fn project_config_path() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .map(|d| d.join(".nxc").join("config.toml"))
}

pub fn load() -> Result<Config> {
    let global  = global_config_path();
    let project = project_config_path();
    let cfg = load_from_path(
        global.as_deref().and_then(|p| p.to_str()).unwrap_or(""),
        project.as_deref().and_then(|p| p.to_str()),
    )?;
    Ok(apply_env_overrides(cfg))
}

/// Intermediate struct for TOML deserialization — all fields optional so we
/// can distinguish "field was set" from "field was absent".
#[derive(serde::Deserialize, Default)]
struct PartialConfig {
    provider: Option<PartialProvider>,
    agent:    Option<PartialAgent>,
    tools:    Option<PartialTools>,
    mcp:      Option<PartialMcp>,
    search:   Option<PartialSearch>,
}

#[derive(serde::Deserialize, Default)]
struct PartialProvider {
    api_key:  Option<String>,
    model:    Option<String>,
    base_url: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct PartialAgent {
    approval_mode: Option<crate::model::ApprovalMode>,
    max_turns:     Option<u32>,
    context_limit: Option<u32>,
}

#[derive(serde::Deserialize, Default)]
struct PartialTools {
    approval: Option<std::collections::HashMap<String, crate::model::ApprovalMode>>,
}

#[derive(serde::Deserialize, Default)]
struct PartialMcp {
    servers: Option<Vec<crate::model::McpServer>>,
}

#[derive(serde::Deserialize, Default)]
struct PartialSearch {
    serpapi_key: Option<String>,
}

fn apply_partial(base: &mut Config, overlay: PartialConfig) {
    if let Some(p) = overlay.provider {
        if let Some(v) = p.api_key  { base.provider.api_key  = v; }
        if let Some(v) = p.model    { base.provider.model    = v; }
        if let Some(v) = p.base_url { base.provider.base_url = v; }
    }
    if let Some(a) = overlay.agent {
        if let Some(v) = a.approval_mode { base.agent.approval_mode = v; }
        if let Some(v) = a.max_turns     { base.agent.max_turns     = v; }
        if let Some(v) = a.context_limit { base.agent.context_limit = v; }
    }
    if let Some(t) = overlay.tools {
        if let Some(map) = t.approval { base.tools.approval.extend(map); }
    }
    if let Some(m) = overlay.mcp {
        if let Some(servers) = m.servers { base.mcp.servers.extend(servers); }
    }
    if let Some(s) = overlay.search {
        if let Some(v) = s.serpapi_key { base.search.serpapi_key = Some(v); }
    }
}

pub fn load_from_path(global_path: &str, project_path: Option<&str>) -> Result<Config> {
    let mut cfg = Config::default();

    if !global_path.is_empty() {
        if let Ok(text) = fs::read_to_string(global_path) {
            let parsed: PartialConfig = toml::from_str(&text)
                .with_context(|| format!("parsing {global_path}"))?;
            apply_partial(&mut cfg, parsed);
        }
    }

    if let Some(p) = project_path {
        if let Ok(text) = fs::read_to_string(p) {
            let parsed: PartialConfig = toml::from_str(&text)
                .with_context(|| format!("parsing {p}"))?;
            apply_partial(&mut cfg, parsed);
        }
    }

    Ok(cfg)
}

pub fn apply_env_overrides(mut cfg: Config) -> Config {
    if let Ok(v) = std::env::var("NXC_API_KEY")  { cfg.provider.api_key  = v; }
    if let Ok(v) = std::env::var("NXC_MODEL")    { cfg.provider.model    = v; }
    if let Ok(v) = std::env::var("NXC_BASE_URL") { cfg.provider.base_url = v; }
    if let Ok(v) = std::env::var("NXC_APPROVAL") {
        cfg.agent.approval_mode = match v.as_str() {
            "ask"  => ApprovalMode::Ask,
            "yolo" => ApprovalMode::Yolo,
            _      => ApprovalMode::Auto,
        };
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn loads_global_toml() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");
        fs::write(&cfg_path, r#"
[provider]
api_key = "sk-test"
model   = "openai/gpt-4o"
"#).unwrap();
        let cfg = load_from_path(cfg_path.to_str().unwrap(), None).unwrap();
        assert_eq!(cfg.provider.api_key, "sk-test");
        assert_eq!(cfg.provider.model, "openai/gpt-4o");
    }

    #[test]
    fn env_var_overrides_toml() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");
        fs::write(&cfg_path, "[provider]\napi_key = \"sk-from-toml\"\n").unwrap();

        temp_env::with_var("NXC_API_KEY", Some("sk-from-env"), || {
            let base = load_from_path(cfg_path.to_str().unwrap(), None).unwrap();
            let cfg  = apply_env_overrides(base);
            assert_eq!(cfg.provider.api_key, "sk-from-env");
        });
    }

    #[test]
    fn project_toml_overrides_global() {
        let global_dir  = tempdir().unwrap();
        let project_dir = tempdir().unwrap();
        fs::write(global_dir.path().join("config.toml"),
            "[provider]\nmodel = \"openai/gpt-4o\"\n").unwrap();
        fs::write(project_dir.path().join("config.toml"),
            "[provider]\nmodel = \"anthropic/claude-haiku-4-5-20251001\"\n").unwrap();
        let cfg = load_from_path(
            global_dir.path().join("config.toml").to_str().unwrap(),
            Some(project_dir.path().join("config.toml").to_str().unwrap()),
        ).unwrap();
        assert_eq!(cfg.provider.model, "anthropic/claude-haiku-4-5-20251001");
    }
}
