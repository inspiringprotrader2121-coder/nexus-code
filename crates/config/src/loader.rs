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

pub fn load_from_path(global_path: &str, project_path: Option<&str>) -> Result<Config> {
    let mut cfg = Config::default();

    if !global_path.is_empty() {
        if let Ok(text) = fs::read_to_string(global_path) {
            let parsed: Config = toml::from_str(&text)
                .with_context(|| format!("parsing {global_path}"))?;
            merge_into(&mut cfg, parsed);
        }
    }

    if let Some(p) = project_path {
        if let Ok(text) = fs::read_to_string(p) {
            let parsed: Config = toml::from_str(&text)
                .with_context(|| format!("parsing {p}"))?;
            merge_into(&mut cfg, parsed);
        }
    }

    Ok(cfg)
}

fn merge_into(base: &mut Config, overlay: Config) {
    // provider
    if !overlay.provider.api_key.is_empty() {
        base.provider.api_key = overlay.provider.api_key;
    }
    if overlay.provider.model != Config::default().provider.model {
        base.provider.model = overlay.provider.model;
    }
    if overlay.provider.base_url != Config::default().provider.base_url {
        base.provider.base_url = overlay.provider.base_url;
    }
    // agent
    if overlay.agent.approval_mode != ApprovalMode::Auto {
        base.agent.approval_mode = overlay.agent.approval_mode;
    }
    // only override scalar agent fields if they differ from defaults
    if overlay.agent.max_turns != 50 {
        base.agent.max_turns = overlay.agent.max_turns;
    }
    if overlay.agent.context_limit != 128_000 {
        base.agent.context_limit = overlay.agent.context_limit;
    }
    // tools: merge approval maps
    base.tools.approval.extend(overlay.tools.approval);
    // mcp: append servers
    base.mcp.servers.extend(overlay.mcp.servers);
    // search
    if overlay.search.serpapi_key.is_some() {
        base.search.serpapi_key = overlay.search.serpapi_key;
    }
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
        std::env::set_var("NXC_API_KEY", "sk-from-env");
        let cfg = apply_env_overrides(Config::default());
        assert_eq!(cfg.provider.api_key, "sk-from-env");
        std::env::remove_var("NXC_API_KEY");
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
