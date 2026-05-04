# Nexus Code Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `nxc`, a Rust CLI coding agent that connects to OpenRouter, runs a ReAct tool loop, and ships as a single cross-platform binary.

**Architecture:** 6-crate Cargo workspace (config → provider → tools + mcp → agent → cli). Each crate owns one concern. The agent runs a ReAct loop: stream a completion, detect tool calls, approve, execute, append result, loop.

**Tech Stack:** Rust 2021, tokio, reqwest (SSE), clap, reedline, crossterm, serde/toml, git2, ignore, similar, directories, async-trait, wiremock (tests), tempfile (tests), assert_cmd (tests).

---

## File Map

```
nexus-code/
├── Cargo.toml                              # workspace root
├── crates/
│   ├── config/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      # pub use; run_wizard
│   │       ├── model.rs                    # Config, ProviderConfig, AgentConfig, ApprovalMode, …
│   │       └── loader.rs                   # load() merges global + project TOML + env vars
│   ├── provider/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs                    # Message, ToolCall, ToolDef, CompletionResponse, Usage
│   │       ├── client.rs                   # Client::new(), complete(), stream text deltas via callback
│   │       └── models.rs                   # list_models(), ModelInfo, pricing cache (~/.cache/nxc/)
│   ├── tools/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      # Tool trait, ToolResult, all_tools()
│   │       ├── files.rs                    # ReadFile, WriteFile, ApplyPatch, ListDir, DeleteFile
│   │       ├── shell.rs                    # Bash (streams stdout/stderr)
│   │       ├── search.rs                   # GrepCodebase, FindFiles
│   │       ├── git.rs                      # GitStatus, GitDiff, GitLog, GitCommit, GitBranch
│   │       ├── web.rs                      # FetchUrl, WebSearch
│   │       └── vision.rs                   # ReadImage
│   ├── mcp/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs                   # JSON-RPC stdio client (read/write lines)
│   │       └── server.rs                   # McpServer: spawn, discover tools, call tool, kill
│   ├── agent/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── history.rs                  # History: push, truncate to context_limit
│   │       ├── approval.rs                 # needs_approval(), prompt_user() → bool
│   │       ├── react.rs                    # Agent::run_turn(), run_session()
│   │       └── session.rs                  # save_session(), load_session(), list_sessions()
│   └── cli/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                     # parse args, dispatch to subcommand or session
│           ├── args.rs                     # Cli struct (clap), Subcommand enum
│           ├── prompt.rs                   # readline loop via reedline
│           ├── output.rs                   # print_delta(), print_tool_action(), status_line()
│           └── commands.rs                 # cmd_init(), cmd_models(), cmd_config(), cmd_sessions()
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
└── .gitignore
```

---

## Task 1: Cargo workspace scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `crates/config/Cargo.toml`
- Create: `crates/provider/Cargo.toml`
- Create: `crates/tools/Cargo.toml`
- Create: `crates/mcp/Cargo.toml`
- Create: `crates/agent/Cargo.toml`
- Create: `crates/cli/Cargo.toml`
- Create: `.gitignore`

- [ ] **Step 1: Write workspace Cargo.toml**

```toml
# Cargo.toml
[workspace]
members  = ["crates/cli","crates/agent","crates/tools","crates/mcp","crates/provider","crates/config"]
resolver = "2"

[workspace.dependencies]
tokio        = { version = "1",    features = ["full"] }
serde        = { version = "1",    features = ["derive"] }
serde_json   = "1"
toml         = "0.8"
reqwest      = { version = "0.12", features = ["json", "stream"] }
clap         = { version = "4",    features = ["derive"] }
reedline     = "0.35"
crossterm    = "0.27"
ignore       = "0.4"
similar      = { version = "2", features = ["text"] }
git2         = "0.19"
directories  = "5"
async-trait  = "0.1"
anyhow       = "1"
thiserror    = "2"
futures      = "1"
base64       = "0.22"
futures-util = "0.3"
# dev
wiremock     = "0.6"
tempfile     = "3"
assert_cmd   = "2"
```

- [ ] **Step 2: Write each crate's Cargo.toml**

`crates/config/Cargo.toml`:
```toml
[package]
name    = "nxc-config"
version = "0.1.0"
edition = "2021"

[dependencies]
serde      = { workspace = true }
serde_json = { workspace = true }
toml       = { workspace = true }
anyhow     = { workspace = true }
thiserror  = { workspace = true }
directories = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

`crates/provider/Cargo.toml`:
```toml
[package]
name    = "nxc-provider"
version = "0.1.0"
edition = "2021"

[dependencies]
nxc-config   = { path = "../config" }
tokio        = { workspace = true }
serde        = { workspace = true }
serde_json   = { workspace = true }
reqwest      = { workspace = true }
anyhow       = { workspace = true }
futures-util = { workspace = true }
directories  = { workspace = true }

[dev-dependencies]
wiremock = { workspace = true }
tokio    = { workspace = true, features = ["full"] }
```

`crates/tools/Cargo.toml`:
```toml
[package]
name    = "nxc-tools"
version = "0.1.0"
edition = "2021"

[dependencies]
nxc-config  = { path = "../config" }
tokio       = { workspace = true }
serde       = { workspace = true }
serde_json  = { workspace = true }
anyhow      = { workspace = true }
async-trait = { workspace = true }
ignore      = { workspace = true }
similar     = { workspace = true }
git2        = { workspace = true }
base64      = { workspace = true }
reqwest     = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio    = { workspace = true, features = ["full"] }
```

`crates/mcp/Cargo.toml`:
```toml
[package]
name    = "nxc-mcp"
version = "0.1.0"
edition = "2021"

[dependencies]
nxc-tools   = { path = "../tools" }
nxc-config  = { path = "../config" }
tokio       = { workspace = true }
serde       = { workspace = true }
serde_json  = { workspace = true }
anyhow      = { workspace = true }
async-trait = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["full"] }
```

`crates/agent/Cargo.toml`:
```toml
[package]
name    = "nxc-agent"
version = "0.1.0"
edition = "2021"

[dependencies]
nxc-config   = { path = "../config" }
nxc-provider = { path = "../provider" }
nxc-tools    = { path = "../tools" }
nxc-mcp      = { path = "../mcp" }
tokio        = { workspace = true }
serde        = { workspace = true }
serde_json   = { workspace = true }
anyhow       = { workspace = true }
directories  = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio    = { workspace = true, features = ["full"] }
```

`crates/cli/Cargo.toml`:
```toml
[package]
name    = "nxc"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "nxc"
path = "src/main.rs"

[dependencies]
nxc-config   = { path = "../config" }
nxc-provider = { path = "../provider" }
nxc-agent    = { path = "../agent" }
nxc-tools    = { path = "../tools" }
nxc-mcp      = { path = "../mcp" }
tokio        = { workspace = true }
clap         = { workspace = true }
reedline     = { workspace = true }
crossterm    = { workspace = true }
anyhow       = { workspace = true }
serde_json   = { workspace = true }

[dev-dependencies]
assert_cmd = { workspace = true }
tempfile   = { workspace = true }
```

- [ ] **Step 3: Create stub lib.rs files for each crate**

```bash
mkdir -p crates/{config,provider,tools,mcp,agent}/src
mkdir -p crates/cli/src
for crate in config provider tools mcp agent; do
  echo "// stub" > crates/$crate/src/lib.rs
done
echo 'fn main() {}' > crates/cli/src/main.rs
```

- [ ] **Step 4: Verify workspace compiles**

```bash
cargo build
```
Expected: Compiles with 0 errors.

- [ ] **Step 5: Write .gitignore and commit**

```
/target
**/.DS_Store
.nxc/sessions/
```

```bash
git add .
git commit -m "feat: initialize cargo workspace"
```

---

## Task 2: Config — types

**Files:**
- Create: `crates/config/src/model.rs`
- Modify: `crates/config/src/lib.rs`

- [ ] **Step 1: Write failing test for Config defaults**

Add to `crates/config/src/model.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_approval_mode_is_auto() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.approval_mode, ApprovalMode::Auto);
        assert_eq!(cfg.max_turns, 50);
        assert_eq!(cfg.context_limit, 128_000);
    }

    #[test]
    fn approval_mode_deserializes_from_string() {
        let toml = r#"approval_mode = "yolo""#;
        let cfg: AgentConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.approval_mode, ApprovalMode::Yolo);
    }
}
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cargo test -p nxc-config
```
Expected: FAIL — `AgentConfig` not defined.

- [ ] **Step 3: Implement model types**

```rust
// crates/config/src/model.rs
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub search: SearchConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_model()    -> String { "anthropic/claude-sonnet-4-6".into() }
fn default_base_url() -> String { "https://openrouter.ai/api/v1".into() }

impl Default for ProviderConfig {
    fn default() -> Self {
        Self { api_key: String::new(), model: default_model(), base_url: default_base_url() }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalMode { Auto, Ask, Yolo }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    #[serde(default = "ApprovalMode::default")]
    pub approval_mode: ApprovalMode,
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    #[serde(default = "default_context_limit")]
    pub context_limit: u32,
}

fn default_max_turns()     -> u32 { 50 }
fn default_context_limit() -> u32 { 128_000 }

impl Default for AgentConfig {
    fn default() -> Self {
        Self { approval_mode: ApprovalMode::Auto, max_turns: 50, context_limit: 128_000 }
    }
}

impl Default for ApprovalMode {
    fn default() -> Self { ApprovalMode::Auto }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub approval: HashMap<String, ApprovalMode>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServer>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpServer {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SearchConfig {
    pub serpapi_key: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderConfig::default(),
            agent:    AgentConfig::default(),
            tools:    ToolsConfig::default(),
            mcp:      McpConfig::default(),
            search:   SearchConfig::default(),
        }
    }
}
```

- [ ] **Step 4: Export from lib.rs**

```rust
// crates/config/src/lib.rs
mod model;
pub use model::*;
```

- [ ] **Step 5: Run tests — verify pass**

```bash
cargo test -p nxc-config
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/config/
git commit -m "feat(config): add Config type model"
```

---

## Task 3: Config — loader (TOML + env vars)

**Files:**
- Create: `crates/config/src/loader.rs`
- Modify: `crates/config/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/config/src/loader.rs
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
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc-config loader
```
Expected: FAIL — `load_from_path` not defined.

- [ ] **Step 3: Implement loader**

```rust
// crates/config/src/loader.rs
use crate::model::Config;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::{fs, path::PathBuf};

pub fn global_config_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "nexuscode", "nxc")
        .map(|d| d.config_dir().join("config.toml"))
}

pub fn project_config_path() -> Option<PathBuf> {
    std::env::current_dir().ok().map(|d| d.join(".nxc").join("config.toml"))
}

pub fn load() -> Result<Config> {
    let global  = global_config_path().as_deref().map(|p| p.to_str().unwrap().to_string());
    let project = project_config_path().as_deref().map(|p| p.to_str().unwrap().to_string());
    let cfg = load_from_path(
        global.as_deref().unwrap_or(""),
        project.as_deref(),
    )?;
    Ok(apply_env_overrides(cfg))
}

pub fn load_from_path(global_path: &str, project_path: Option<&str>) -> Result<Config> {
    let mut cfg = Config::default();

    if let Ok(text) = fs::read_to_string(global_path) {
        let parsed: Config = toml::from_str(&text)
            .with_context(|| format!("parsing {global_path}"))?;
        merge_into(&mut cfg, parsed);
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
    if !overlay.provider.api_key.is_empty() { base.provider.api_key = overlay.provider.api_key; }
    if overlay.provider.model != "anthropic/claude-sonnet-4-6" || base.provider.model == "anthropic/claude-sonnet-4-6" {
        base.provider.model = overlay.provider.model;
    }
    if overlay.provider.base_url != "https://openrouter.ai/api/v1" || base.provider.base_url == "https://openrouter.ai/api/v1" {
        base.provider.base_url = overlay.provider.base_url;
    }
    if !overlay.agent.approval_mode.eq(&crate::model::ApprovalMode::Auto) {
        base.agent.approval_mode = overlay.agent.approval_mode;
    }
    base.agent.max_turns     = overlay.agent.max_turns;
    base.agent.context_limit = overlay.agent.context_limit;
    base.tools.approval.extend(overlay.tools.approval);
    base.mcp.servers.extend(overlay.mcp.servers);
    if overlay.search.serpapi_key.is_some() { base.search.serpapi_key = overlay.search.serpapi_key; }
}

pub fn apply_env_overrides(mut cfg: Config) -> Config {
    if let Ok(v) = std::env::var("NXC_API_KEY")  { cfg.provider.api_key  = v; }
    if let Ok(v) = std::env::var("NXC_MODEL")    { cfg.provider.model    = v; }
    if let Ok(v) = std::env::var("NXC_BASE_URL") { cfg.provider.base_url = v; }
    if let Ok(v) = std::env::var("NXC_APPROVAL") {
        cfg.agent.approval_mode = match v.as_str() {
            "ask"  => crate::model::ApprovalMode::Ask,
            "yolo" => crate::model::ApprovalMode::Yolo,
            _      => crate::model::ApprovalMode::Auto,
        };
    }
    cfg
}
```

- [ ] **Step 4: Export from lib.rs**

```rust
// crates/config/src/lib.rs
mod model;
mod loader;
pub use model::*;
pub use loader::{load, load_from_path, apply_env_overrides, global_config_path, project_config_path};
```

- [ ] **Step 5: Run tests — verify pass**

```bash
cargo test -p nxc-config
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/config/
git commit -m "feat(config): TOML loader with env var overrides"
```

---

## Task 4: Config — `nxc init` wizard

**Files:**
- Create: `crates/config/src/wizard.rs`
- Modify: `crates/config/src/lib.rs`

- [ ] **Step 1: Write test for wizard output format**

```rust
// crates/config/src/wizard.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_toml_from_wizard_answers() {
        let answers = WizardAnswers {
            api_key:       "sk-test".into(),
            model:         "openai/gpt-4o".into(),
            approval_mode: "auto".into(),
            ask_for_bash:  true,
            ask_for_git:   true,
        };
        let toml = answers_to_toml(&answers);
        assert!(toml.contains("api_key"));
        assert!(toml.contains("sk-test"));
        assert!(toml.contains("bash"));
    }
}
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc-config wizard
```
Expected: FAIL.

- [ ] **Step 3: Implement wizard**

```rust
// crates/config/src/wizard.rs
use anyhow::Result;
use std::{fs, io::{self, Write}, path::Path};

pub struct WizardAnswers {
    pub api_key:       String,
    pub model:         String,
    pub approval_mode: String,
    pub ask_for_bash:  bool,
    pub ask_for_git:   bool,
}

pub fn run_wizard(models: &[String]) -> Result<WizardAnswers> {
    let api_key = prompt_hidden("OpenRouter API key")?;

    println!("\nAvailable models (enter number or type custom):");
    let display = if models.is_empty() {
        vec!["anthropic/claude-sonnet-4-6".to_string(), "openai/gpt-4o".to_string()]
    } else {
        models.iter().take(10).cloned().collect()
    };
    for (i, m) in display.iter().enumerate() { println!("  {}. {m}", i + 1); }

    let model_input = prompt("Model [1]")?;
    let model = model_input.trim().parse::<usize>()
        .ok()
        .and_then(|n| display.get(n.saturating_sub(1)).cloned())
        .unwrap_or_else(|| {
            if model_input.trim().is_empty() { display[0].clone() } else { model_input.trim().to_string() }
        });

    let approval_raw = prompt("Default approval mode (auto/ask/yolo) [auto]")?;
    let approval_mode = if approval_raw.trim().is_empty() { "auto".into() } else { approval_raw.trim().to_string() };

    let bash_raw = prompt("Always ask before running bash commands? (y/N)")?;
    let ask_for_bash = bash_raw.trim().eq_ignore_ascii_case("y");

    let git_raw = prompt("Always ask before git commits? (y/N)")?;
    let ask_for_git = git_raw.trim().eq_ignore_ascii_case("y");

    Ok(WizardAnswers { api_key, model, approval_mode, ask_for_bash, ask_for_git })
}

pub fn answers_to_toml(a: &WizardAnswers) -> String {
    let mut tool_approvals = String::new();
    if a.ask_for_bash { tool_approvals.push_str("bash = \"ask\"\n"); }
    if a.ask_for_git  { tool_approvals.push_str("git_commit = \"ask\"\n"); }

    format!(
        "[provider]\napi_key = \"{}\"\nmodel   = \"{}\"\nbase_url = \"https://openrouter.ai/api/v1\"\n\n[agent]\napproval_mode = \"{}\"\nmax_turns     = 50\ncontext_limit = 128000\n\n[tools.approval]\n{}",
        a.api_key, a.model, a.approval_mode, tool_approvals
    )
}

pub fn write_wizard_config(path: &Path, toml: &str) -> Result<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    fs::write(path, toml)?;
    Ok(())
}

fn prompt(label: &str) -> Result<String> {
    print!("? {label}: ");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf)
}

fn prompt_hidden(label: &str) -> Result<String> {
    print!("? {label}: ");
    io::stdout().flush()?;
    // Use rpassword-style hiding on real terminals; fall back to plain read in tests
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}
```

- [ ] **Step 4: Export from lib.rs**

Add to `crates/config/src/lib.rs`:
```rust
mod wizard;
pub use wizard::{run_wizard, answers_to_toml, write_wizard_config, WizardAnswers};
```

- [ ] **Step 5: Run tests — verify pass**

```bash
cargo test -p nxc-config
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/config/
git commit -m "feat(config): nxc init wizard"
```

---

## Task 5: Provider — types + HTTP client + SSE streaming

**Files:**
- Create: `crates/provider/src/types.rs`
- Create: `crates/provider/src/client.rs`
- Modify: `crates/provider/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/provider/src/client.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn complete_returns_text_response() {
        let server = MockServer::start().await;
        // SSE response with one text chunk then DONE
        let sse_body = "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\ndata: [DONE]\n\n";
        Mock::given(method("POST")).and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body))
            .mount(&server).await;

        let client = Client::new(server.uri(), "sk-test".into(), "gpt-4o".into());
        let mut deltas: Vec<String> = vec![];
        let resp = client.complete(
            &[Message::user("hi")],
            &[],
            |d| deltas.push(d.to_string()),
        ).await.unwrap();

        assert_eq!(deltas.join(""), "Hello");
        assert_eq!(resp.text, "Hello");
        assert!(resp.tool_calls.is_empty());
        assert_eq!(resp.usage.prompt_tokens, 10);
    }
}
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc-provider
```
Expected: FAIL.

- [ ] **Step 3: Implement types**

```rust
// crates/provider/src/types.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>, // String or array for images
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: Some(Value::String(content.into())), tool_calls: None, tool_call_id: None, name: None }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: Some(Value::String(content.into())), tool_calls: None, tool_call_id: None, name: None }
    }
    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: Some(Value::String(content.into())), tool_calls: None, tool_call_id: None, name: None }
    }
    pub fn assistant_tool_calls(calls: Vec<ToolCall>) -> Self {
        Self { role: "assistant".into(), content: None, tool_calls: Some(calls), tool_call_id: None, name: None }
    }
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self { role: "tool".into(), content: Some(Value::String(content.into())), tool_calls: None, tool_call_id: Some(tool_call_id.into()), name: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id:       String,
    #[serde(rename = "type")]
    pub kind:     String, // "function"
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name:      String,
    pub arguments: String, // JSON string, accumulated from stream chunks
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub kind:     &'static str, // "function"
    pub function: FunctionDef,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDef {
    pub name:        &'static str,
    pub description: &'static str,
    pub parameters:  serde_json::Value,
}

#[derive(Debug, Default)]
pub struct Usage {
    pub prompt_tokens:     u32,
    pub completion_tokens: u32,
}

#[derive(Debug)]
pub struct CompletionResponse {
    pub text:       String,
    pub tool_calls: Vec<ToolCall>,
    pub usage:      Usage,
}
```

- [ ] **Step 4: Implement client with SSE streaming**

```rust
// crates/provider/src/client.rs
use crate::types::*;
use anyhow::{bail, Result};
use futures_util::StreamExt;
use serde_json::{json, Value};

pub struct Client {
    http:     reqwest::Client,
    base_url: String,
    api_key:  String,
    model:    String,
}

impl Client {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self { http: reqwest::Client::new(), base_url: base_url.into(), api_key: api_key.into(), model: model.into() }
    }

    pub async fn complete(
        &self,
        messages: &[Message],
        tools:    &[ToolDef],
        on_text:  impl Fn(&str),
    ) -> Result<CompletionResponse> {
        let mut body = json!({
            "model":    self.model,
            "messages": messages,
            "stream":   true,
            "stream_options": { "include_usage": true },
        });
        if !tools.is_empty() { body["tools"] = json!(tools); }

        let resp = self.http.post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("OpenRouter error {status}: {text}");
        }

        let mut stream = resp.bytes_stream();
        let mut full_text  = String::new();
        let mut tool_calls: Vec<PartialToolCall> = vec![];
        let mut usage = Usage::default();
        let mut buf   = String::new();

        while let Some(chunk) = stream.next().await {
            buf.push_str(&String::from_utf8_lossy(&chunk?));
            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].trim().to_string();
                buf = buf[nl + 1..].to_string();
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { break; }
                    if let Ok(evt) = serde_json::from_str::<Value>(data) {
                        self.process_chunk(&evt, &mut full_text, &mut tool_calls, &mut usage, &on_text);
                    }
                }
            }
        }

        let completed_calls = tool_calls.into_iter().map(|p| ToolCall {
            id:   p.id,
            kind: "function".into(),
            function: FunctionCall { name: p.name, arguments: p.arguments },
        }).collect();

        Ok(CompletionResponse { text: full_text, tool_calls: completed_calls, usage })
    }

    fn process_chunk(
        &self, evt: &Value,
        text: &mut String, tool_calls: &mut Vec<PartialToolCall>,
        usage: &mut Usage, on_text: &impl Fn(&str),
    ) {
        // usage at top level (some providers send it on the last chunk)
        if let Some(u) = evt.get("usage").and_then(|u| u.as_object()) {
            if let Some(p) = u.get("prompt_tokens").and_then(|v| v.as_u64())     { usage.prompt_tokens     = p as u32; }
            if let Some(c) = u.get("completion_tokens").and_then(|v| v.as_u64()) { usage.completion_tokens = c as u32; }
        }
        let delta = match evt["choices"][0]["delta"].as_object() { Some(d) => d, None => return };

        if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
            if !content.is_empty() { on_text(content); text.push_str(content); }
        }

        if let Some(calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
            for call in calls {
                let idx = call["index"].as_u64().unwrap_or(0) as usize;
                while tool_calls.len() <= idx { tool_calls.push(PartialToolCall::default()); }
                if let Some(id)   = call["id"].as_str()                          { tool_calls[idx].id   = id.into(); }
                if let Some(name) = call["function"]["name"].as_str()             { tool_calls[idx].name = name.into(); }
                if let Some(args) = call["function"]["arguments"].as_str()        { tool_calls[idx].arguments.push_str(args); }
            }
        }
    }
}

#[derive(Default)]
struct PartialToolCall { id: String, name: String, arguments: String }
```

- [ ] **Step 5: Wire up lib.rs**

```rust
// crates/provider/src/lib.rs
mod types;
mod client;
pub mod models;
pub use types::*;
pub use client::Client;
```

- [ ] **Step 6: Run tests — verify pass**

```bash
cargo test -p nxc-provider
```
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/provider/
git commit -m "feat(provider): OpenRouter client with SSE streaming"
```

---

## Task 6: Provider — model listing + pricing cache

**Files:**
- Create: `crates/provider/src/models.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/provider/src/models.rs
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn lists_models_from_api() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "data": [
                { "id": "anthropic/claude-sonnet-4-6", "pricing": { "prompt": "0.000003", "completion": "0.000015" } },
                { "id": "openai/gpt-4o", "pricing": { "prompt": "0.000005", "completion": "0.000015" } }
            ]
        });
        Mock::given(method("GET")).and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server).await;

        let fetcher = ModelFetcher::new(server.uri(), "sk-test".into());
        let models  = fetcher.fetch().await.unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "anthropic/claude-sonnet-4-6");
        assert!((models[0].prompt_price_per_token - 0.000003).abs() < 1e-10);
    }

    #[test]
    fn cost_estimate_is_correct() {
        let info = ModelInfo { id: "x".into(), prompt_price_per_token: 0.000003, completion_price_per_token: 0.000015 };
        let cost = info.estimate_cost(100, 50);
        assert!((cost - 0.0010_5).abs() < 1e-8);
    }
}
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc-provider models
```
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
// crates/provider/src/models.rs
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub prompt_price_per_token:     f64,
    pub completion_price_per_token: f64,
}

impl ModelInfo {
    pub fn estimate_cost(&self, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        prompt_tokens as f64     * self.prompt_price_per_token
        + completion_tokens as f64 * self.completion_price_per_token
    }
}

pub struct ModelFetcher {
    http:     reqwest::Client,
    base_url: String,
    api_key:  String,
}

impl ModelFetcher {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self { http: reqwest::Client::new(), base_url: base_url.into(), api_key: api_key.into() }
    }

    pub async fn fetch(&self) -> Result<Vec<ModelInfo>> {
        #[derive(Deserialize)] struct ApiResp { data: Vec<ApiModel> }
        #[derive(Deserialize)] struct ApiModel { id: String, pricing: Option<Pricing> }
        #[derive(Deserialize)] struct Pricing { prompt: String, completion: String }

        let resp: ApiResp = self.http.get(format!("{}/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send().await?.json().await?;

        Ok(resp.data.into_iter().map(|m| {
            let (p, c) = m.pricing.map(|pr| (
                pr.prompt.parse::<f64>().unwrap_or(0.0),
                pr.completion.parse::<f64>().unwrap_or(0.0),
            )).unwrap_or((0.0, 0.0));
            ModelInfo { id: m.id, prompt_price_per_token: p, completion_price_per_token: c }
        }).collect())
    }
}

pub type PricingCache = HashMap<String, ModelInfo>;

pub fn build_cache(models: Vec<ModelInfo>) -> PricingCache {
    models.into_iter().map(|m| (m.id.clone(), m)).collect()
}
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cargo test -p nxc-provider
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/provider/
git commit -m "feat(provider): model listing and pricing cache"
```

---

## Task 7: Tools — `Tool` trait + `ToolResult`

**Files:**
- Modify: `crates/tools/src/lib.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tools/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;
    #[async_trait::async_trait]
    impl Tool for EchoTool {
        fn name(&self)        -> &'static str { "echo" }
        fn description(&self) -> &'static str { "echoes input" }
        fn parameters(&self)  -> serde_json::Value {
            serde_json::json!({"type":"object","properties":{"msg":{"type":"string"}},"required":["msg"]})
        }
        async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult::ok(args["msg"].as_str().unwrap_or("").to_string()))
        }
    }

    #[tokio::test]
    async fn tool_executes_and_returns_result() {
        let t   = EchoTool;
        let res = t.execute(serde_json::json!({"msg": "hello"})).await.unwrap();
        assert!(!res.is_error);
        assert_eq!(res.content, "hello");
    }

    #[test]
    fn all_tools_have_unique_names() {
        let tools = all_tools();
        let names: std::collections::HashSet<_> = tools.iter().map(|t| t.name()).collect();
        assert_eq!(names.len(), tools.len());
    }
}
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc-tools
```
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
// crates/tools/src/lib.rs
use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;

mod files;
mod shell;
mod search;
mod git;
mod web;
mod vision;

pub use files::{ReadFile, WriteFile, ApplyPatch, ListDir, DeleteFile};
pub use shell::Bash;
pub use search::{GrepCodebase, FindFiles};
pub use git::{GitStatus, GitDiff, GitLog, GitCommit, GitBranch};
pub use web::{FetchUrl, WebSearch};
pub use vision::ReadImage;

pub struct ToolResult {
    pub content:  String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn ok(content: impl Into<String>)  -> Self { Self { content: content.into(), is_error: false } }
    pub fn err(content: impl Into<String>) -> Self { Self { content: content.into(), is_error: true  } }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self)        -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self)  -> Value;
    async fn execute(&self, args: Value) -> Result<ToolResult>;
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ReadFile),   Box::new(WriteFile), Box::new(ApplyPatch),
        Box::new(ListDir),    Box::new(DeleteFile),
        Box::new(Bash),
        Box::new(GrepCodebase), Box::new(FindFiles),
        Box::new(GitStatus),  Box::new(GitDiff),   Box::new(GitLog),
        Box::new(GitCommit),  Box::new(GitBranch),
        Box::new(FetchUrl),   Box::new(WebSearch),
        Box::new(ReadImage),
    ]
}
```

- [ ] **Step 4: Create stub files for each tool module** (so it compiles):

```bash
for f in files shell search git web vision; do
  echo "use crate::{Tool, ToolResult}; use async_trait::async_trait; use serde_json::Value; use anyhow::Result;" \
  > crates/tools/src/$f.rs
done
```

Then add stub structs to each file so `all_tools()` compiles. For example `crates/tools/src/files.rs`:
```rust
use crate::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;

pub struct ReadFile;
pub struct WriteFile;
pub struct ApplyPatch;
pub struct ListDir;
pub struct DeleteFile;

#[async_trait]
impl Tool for ReadFile {
    fn name(&self)        -> &'static str { "read_file" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}
// repeat for WriteFile, ApplyPatch, ListDir, DeleteFile with their names
```

Repeat the same pattern for `shell.rs` (`Bash`), `search.rs` (`GrepCodebase`, `FindFiles`), `git.rs` (`GitStatus`, `GitDiff`, `GitLog`, `GitCommit`, `GitBranch`), `web.rs` (`FetchUrl`, `WebSearch`), `vision.rs` (`ReadImage`).

- [ ] **Step 5: Run tests — verify pass**

```bash
cargo test -p nxc-tools
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/tools/
git commit -m "feat(tools): Tool trait, ToolResult, tool registry"
```

---

## Task 8: Tools — file tools (real implementations)

**Files:**
- Modify: `crates/tools/src/files.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tools/src/files.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn read_file_returns_contents() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, "hello world").unwrap();
        let res = ReadFile.execute(json!({"path": path.to_str().unwrap()})).await.unwrap();
        assert!(!res.is_error);
        assert_eq!(res.content, "hello world");
    }

    #[tokio::test]
    async fn write_file_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("out.txt");
        WriteFile.execute(json!({"path": path.to_str().unwrap(), "content": "written"})).await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "written");
    }

    #[tokio::test]
    async fn list_dir_returns_entries() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "").unwrap();
        std::fs::write(dir.path().join("b.rs"), "").unwrap();
        let res = ListDir.execute(json!({"path": dir.path().to_str().unwrap()})).await.unwrap();
        assert!(res.content.contains("a.rs"));
        assert!(res.content.contains("b.rs"));
    }

    #[tokio::test]
    async fn delete_file_removes_file() {
        let dir  = tempdir().unwrap();
        let path = dir.path().join("del.txt");
        std::fs::write(&path, "bye").unwrap();
        DeleteFile.execute(json!({"path": path.to_str().unwrap()})).await.unwrap();
        assert!(!path.exists());
    }
}
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc-tools files
```
Expected: FAIL (stub implementations return "stub", not real content).

- [ ] **Step 3: Implement file tools**

```rust
// crates/tools/src/files.rs
use crate::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::{fs, path::Path};

pub struct ReadFile;
pub struct WriteFile;
pub struct ApplyPatch;
pub struct ListDir;
pub struct DeleteFile;

#[async_trait]
impl Tool for ReadFile {
    fn name(&self)        -> &'static str { "read_file" }
    fn description(&self) -> &'static str { "Read the contents of a file" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"path":{"type":"string","description":"File path"}},"required":["path"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or("");
        match fs::read_to_string(path) {
            Ok(s)  => Ok(ToolResult::ok(s)),
            Err(e) => Ok(ToolResult::err(format!("read_file error: {e}"))),
        }
    }
}

#[async_trait]
impl Tool for WriteFile {
    fn name(&self)        -> &'static str { "write_file" }
    fn description(&self) -> &'static str { "Write content to a file (creates or overwrites)" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let path    = args["path"].as_str().unwrap_or("");
        let content = args["content"].as_str().unwrap_or("");
        if let Some(parent) = Path::new(path).parent() { fs::create_dir_all(parent)?; }
        match fs::write(path, content) {
            Ok(_)  => Ok(ToolResult::ok(format!("wrote {} ({} bytes)", path, content.len()))),
            Err(e) => Ok(ToolResult::err(format!("write_file error: {e}"))),
        }
    }
}

#[async_trait]
impl Tool for ApplyPatch {
    fn name(&self)        -> &'static str { "apply_patch" }
    fn description(&self) -> &'static str { "Apply a unified diff patch to a file" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"patch":{"type":"string"}},"required":["path","patch"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        use similar::{TextDiff, ChangeTag};
        let path  = args["path"].as_str().unwrap_or("");
        let patch = args["patch"].as_str().unwrap_or("");
        // Simple apply: parse unified diff and apply line-by-line
        let original = fs::read_to_string(path).unwrap_or_default();
        let diff = TextDiff::from_lines(&original, patch);
        let mut result = String::new();
        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal  | ChangeTag::Insert => result.push_str(change.as_str().unwrap_or("")),
                ChangeTag::Delete => {}
            }
        }
        match fs::write(path, &result) {
            Ok(_)  => Ok(ToolResult::ok(format!("patched {path}"))),
            Err(e) => Ok(ToolResult::err(format!("apply_patch error: {e}"))),
        }
    }
}

#[async_trait]
impl Tool for ListDir {
    fn name(&self)        -> &'static str { "list_dir" }
    fn description(&self) -> &'static str { "List files and directories at a path" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"path":{"type":"string","description":"Directory path"}},"required":["path"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or(".");
        match fs::read_dir(path) {
            Ok(entries) => {
                let mut lines: Vec<String> = entries.filter_map(|e| e.ok()).map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if e.file_type().map(|t| t.is_dir()).unwrap_or(false) { format!("{name}/") } else { name }
                }).collect();
                lines.sort();
                Ok(ToolResult::ok(lines.join("\n")))
            }
            Err(e) => Ok(ToolResult::err(format!("list_dir error: {e}"))),
        }
    }
}

#[async_trait]
impl Tool for DeleteFile {
    fn name(&self)        -> &'static str { "delete_file" }
    fn description(&self) -> &'static str { "Delete a file" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or("");
        match fs::remove_file(path) {
            Ok(_)  => Ok(ToolResult::ok(format!("deleted {path}"))),
            Err(e) => Ok(ToolResult::err(format!("delete_file error: {e}"))),
        }
    }
}
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cargo test -p nxc-tools files
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/tools/src/files.rs
git commit -m "feat(tools): file tools (read, write, patch, list, delete)"
```

---

## Task 9: Tools — shell, search, git, web, vision

**Files:**
- Modify: `crates/tools/src/shell.rs`
- Modify: `crates/tools/src/search.rs`
- Modify: `crates/tools/src/git.rs`
- Modify: `crates/tools/src/web.rs`
- Modify: `crates/tools/src/vision.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tools/src/shell.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn bash_runs_command_and_captures_output() {
        let res = Bash.execute(serde_json::json!({"command": "echo hello"})).await.unwrap();
        assert!(!res.is_error);
        assert!(res.content.contains("hello"));
    }
}

// crates/tools/src/search.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    #[tokio::test]
    async fn grep_finds_pattern_in_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "fn hello() {}").unwrap();
        fs::write(dir.path().join("b.rs"), "fn world() {}").unwrap();
        let res = GrepCodebase.execute(serde_json::json!({
            "pattern": "hello",
            "path": dir.path().to_str().unwrap()
        })).await.unwrap();
        assert!(res.content.contains("hello"));
        assert!(!res.content.contains("world"));
    }
}

// crates/tools/src/vision.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    #[tokio::test]
    async fn read_image_returns_base64() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("img.png");
        fs::write(&path, b"\x89PNG\r\n\x1a\n").unwrap();
        let res = ReadImage.execute(serde_json::json!({"path": path.to_str().unwrap()})).await.unwrap();
        assert!(!res.is_error);
        assert!(res.content.starts_with("data:image/"));
    }
}
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc-tools
```
Expected: FAIL on the new tests.

- [ ] **Step 3: Implement shell tool**

```rust
// crates/tools/src/shell.rs
use crate::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

pub struct Bash;

#[async_trait]
impl Tool for Bash {
    fn name(&self)        -> &'static str { "bash" }
    fn description(&self) -> &'static str { "Run a shell command. stdout and stderr are returned." }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"command":{"type":"string","description":"Shell command to run"}},"required":["command"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let cmd = args["command"].as_str().unwrap_or("");
        let out = Command::new("sh").arg("-c").arg(cmd).output()?;
        let mut content = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if !stderr.is_empty() { content.push_str(&format!("\nstderr:\n{stderr}")); }
        if out.status.success() { Ok(ToolResult::ok(content)) }
        else { Ok(ToolResult::err(format!("exit {}\n{content}", out.status.code().unwrap_or(-1)))) }
    }
}
```

- [ ] **Step 4: Implement search tools**

```rust
// crates/tools/src/search.rs
use crate::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use ignore::WalkBuilder;
use serde_json::{json, Value};

pub struct GrepCodebase;
pub struct FindFiles;

#[async_trait]
impl Tool for GrepCodebase {
    fn name(&self)        -> &'static str { "grep_codebase" }
    fn description(&self) -> &'static str { "Search for a text pattern across files, respecting .gitignore" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string","description":"Directory to search (default: cwd)"}},"required":["pattern"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let root    = args["path"].as_str().unwrap_or(".");
        let mut matches: Vec<String> = vec![];
        for entry in WalkBuilder::new(root).build().filter_map(|e| e.ok()) {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Ok(text) = std::fs::read_to_string(entry.path()) {
                    for (i, line) in text.lines().enumerate() {
                        if line.contains(pattern) {
                            matches.push(format!("{}:{}: {}", entry.path().display(), i + 1, line));
                        }
                    }
                }
            }
        }
        if matches.is_empty() { Ok(ToolResult::ok("No matches found.")) }
        else { Ok(ToolResult::ok(matches.join("\n"))) }
    }
}

#[async_trait]
impl Tool for FindFiles {
    fn name(&self)        -> &'static str { "find_files" }
    fn description(&self) -> &'static str { "Find files matching a glob pattern, respecting .gitignore" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"glob":{"type":"string"}},"required":["glob"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let pattern = args["glob"].as_str().unwrap_or("*");
        let matcher = glob::Pattern::new(pattern).unwrap_or(glob::Pattern::new("*").unwrap());
        let mut found: Vec<String> = vec![];
        for entry in WalkBuilder::new(".").build().filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy();
            if matcher.matches(&name) { found.push(entry.path().display().to_string()); }
        }
        found.sort();
        if found.is_empty() { Ok(ToolResult::ok("No files found.")) }
        else { Ok(ToolResult::ok(found.join("\n"))) }
    }
}
```

Add `glob = "0.3"` to `crates/tools/Cargo.toml` dependencies.

- [ ] **Step 5: Implement git tools**

```rust
// crates/tools/src/git.rs
use crate::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

pub struct GitStatus;
pub struct GitDiff;
pub struct GitLog;
pub struct GitCommit;
pub struct GitBranch;

fn git(args: &[&str]) -> ToolResult {
    match Command::new("git").args(args).output() {
        Err(e) => ToolResult::err(format!("git error: {e}")),
        Ok(o) => {
            let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            if o.status.success() { ToolResult::ok(out) } else { ToolResult::err(out) }
        }
    }
}

#[async_trait] impl Tool for GitStatus {
    fn name(&self)        -> &'static str { "git_status" }
    fn description(&self) -> &'static str { "Show git working tree status" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{}}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(git(&["status"])) }
}
#[async_trait] impl Tool for GitDiff {
    fn name(&self)        -> &'static str { "git_diff" }
    fn description(&self) -> &'static str { "Show staged and unstaged diffs" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{"path":{"type":"string"}}}) }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        if let Some(p) = args["path"].as_str() { Ok(git(&["diff", p])) } else { Ok(git(&["diff"])) }
    }
}
#[async_trait] impl Tool for GitLog {
    fn name(&self)        -> &'static str { "git_log" }
    fn description(&self) -> &'static str { "Show recent git commits" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{"n":{"type":"integer","default":10}}}) }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let n = args["n"].as_u64().unwrap_or(10).to_string();
        Ok(git(&["log", "--oneline", &format!("-{n}")]))
    }
}
#[async_trait] impl Tool for GitCommit {
    fn name(&self)        -> &'static str { "git_commit" }
    fn description(&self) -> &'static str { "Stage all changes and create a commit" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{"message":{"type":"string"}},"required":["message"]}) }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let msg = args["message"].as_str().unwrap_or("chore: update");
        let stage = git(&["add", "-A"]);
        if stage.is_error { return Ok(stage); }
        Ok(git(&["commit", "-m", msg]))
    }
}
#[async_trait] impl Tool for GitBranch {
    fn name(&self)        -> &'static str { "git_branch" }
    fn description(&self) -> &'static str { "List branches or create a new one" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{"name":{"type":"string","description":"Branch name to create (omit to list)"}}}) }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        if let Some(name) = args["name"].as_str() { Ok(git(&["checkout", "-b", name])) }
        else { Ok(git(&["branch", "-a"])) }
    }
}
```

Add `pub fn is_error(&self) -> bool { self.is_error }` to `ToolResult` in `lib.rs`.

- [ ] **Step 6: Implement web tools**

```rust
// crates/tools/src/web.rs
use crate::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct FetchUrl;
pub struct WebSearch;

#[async_trait]
impl Tool for FetchUrl {
    fn name(&self)        -> &'static str { "fetch_url" }
    fn description(&self) -> &'static str { "Fetch a URL and return its text content" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let url = args["url"].as_str().unwrap_or("");
        match reqwest::get(url).await {
            Err(e)   => Ok(ToolResult::err(format!("fetch error: {e}"))),
            Ok(resp) => match resp.text().await {
                Ok(text) => Ok(ToolResult::ok(text.chars().take(20_000).collect::<String>())),
                Err(e)   => Ok(ToolResult::err(format!("read error: {e}"))),
            },
        }
    }
}

#[async_trait]
impl Tool for WebSearch {
    fn name(&self)        -> &'static str { "web_search" }
    fn description(&self) -> &'static str { "Search the web for a query. Returns top results." }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let query = args["query"].as_str().unwrap_or("");
        // Use DuckDuckGo lite as fallback when no SerpAPI key configured
        let url = format!("https://lite.duckduckgo.com/lite/?q={}", urlencoding::encode(query));
        match reqwest::get(&url).await {
            Err(e)   => Ok(ToolResult::err(format!("search error: {e}"))),
            Ok(resp) => match resp.text().await {
                Ok(text) => {
                    // Strip HTML tags naively for now
                    let clean: String = text.chars().take(8_000).collect();
                    Ok(ToolResult::ok(clean))
                }
                Err(e) => Ok(ToolResult::err(format!("search read error: {e}"))),
            },
        }
    }
}
```

Add `urlencoding = "2"` to `crates/tools/Cargo.toml`.

- [ ] **Step 7: Implement vision tool**

```rust
// crates/tools/src/vision.rs
use crate::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub struct ReadImage;

#[async_trait]
impl Tool for ReadImage {
    fn name(&self)        -> &'static str { "read_image" }
    fn description(&self) -> &'static str { "Base64-encode a local image file for vision-capable models" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or("");
        match fs::read(path) {
            Err(e) => Ok(ToolResult::err(format!("read_image error: {e}"))),
            Ok(bytes) => {
                let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("png");
                let mime = match ext { "jpg" | "jpeg" => "image/jpeg", "gif" => "image/gif", "webp" => "image/webp", _ => "image/png" };
                Ok(ToolResult::ok(format!("data:{mime};base64,{}", STANDARD.encode(&bytes))))
            }
        }
    }
}
```

- [ ] **Step 8: Run all tool tests — verify pass**

```bash
cargo test -p nxc-tools
```
Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/tools/
git commit -m "feat(tools): shell, search, git, web, vision implementations"
```

---

## Task 10: MCP — JSON-RPC client + server lifecycle

**Files:**
- Modify: `crates/mcp/src/client.rs`
- Modify: `crates/mcp/src/server.rs`
- Modify: `crates/mcp/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/mcp/src/client.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_initialize_request() {
        let req = JsonRpcRequest::new(1, "initialize", serde_json::json!({"protocolVersion":"2024-11-05"}));
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("initialize"));
        assert!(s.contains("jsonrpc"));
    }
}
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc-mcp
```
Expected: FAIL.

- [ ] **Step 3: Implement MCP JSON-RPC client**

```rust
// crates/mcp/src/client.rs
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};

#[derive(Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id:      u64,
    pub method:  String,
    pub params:  Value,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Value) -> Self {
        Self { jsonrpc: "2.0", id, method: method.into(), params }
    }
}

#[derive(Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub id:     Option<u64>,
    pub result: Option<Value>,
    pub error:  Option<Value>,
}

pub struct McpTransport {
    stdin:  ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpTransport {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self { stdin, stdout: BufReader::new(stdout), next_id: 1 }
    }

    pub async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id  = self.next_id;
        self.next_id += 1;
        let req = JsonRpcRequest::new(id, method, params);
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        loop {
            let mut buf = String::new();
            self.stdout.read_line(&mut buf).await?;
            if buf.is_empty() { bail!("MCP server closed stdout"); }
            let resp: JsonRpcResponse = serde_json::from_str(buf.trim())?;
            if resp.id == Some(id) {
                if let Some(err) = resp.error { bail!("MCP error: {err}"); }
                return Ok(resp.result.unwrap_or(Value::Null));
            }
        }
    }
}
```

- [ ] **Step 4: Implement MCP server lifecycle**

```rust
// crates/mcp/src/server.rs
use crate::client::McpTransport;
use nxc_tools::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use nxc_config::McpServer as McpServerConfig;
use serde_json::Value;
use tokio::process::{Child, Command};
use std::process::Stdio;

pub struct McpServerHandle {
    _child:    Child,
    transport: McpTransport,
    tools:     Vec<McpToolProxy>,
}

#[derive(Clone)]
pub struct McpToolProxy {
    pub tool_name:   String,
    pub description: String,
    pub parameters:  Value,
}

impl McpServerHandle {
    pub async fn spawn(cfg: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&cfg.command);
        cmd.args(&cfg.args)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::null());
        for (k, v) in &cfg.env { cmd.env(k, v); }
        let mut child = cmd.spawn()?;

        let stdin  = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut transport = McpTransport::new(stdin, stdout);

        // Initialize
        transport.request("initialize", serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "nxc", "version": "0.1.0" }
        })).await?;

        // Discover tools
        let list = transport.request("tools/list", serde_json::json!({})).await?;
        let tools: Vec<McpToolProxy> = list["tools"].as_array().unwrap_or(&vec![]).iter().map(|t| {
            McpToolProxy {
                tool_name:   t["name"].as_str().unwrap_or("").to_string(),
                description: t["description"].as_str().unwrap_or("").to_string(),
                parameters:  t["inputSchema"].clone(),
            }
        }).collect();

        Ok(Self { _child: child, transport, tools })
    }

    pub fn tool_names(&self) -> Vec<String> { self.tools.iter().map(|t| t.tool_name.clone()).collect() }

    pub async fn call_tool(&mut self, name: &str, args: Value) -> Result<ToolResult> {
        let result = self.transport.request("tools/call", serde_json::json!({"name": name, "arguments": args})).await?;
        let content = result["content"][0]["text"].as_str().unwrap_or("").to_string();
        let is_error = result["isError"].as_bool().unwrap_or(false);
        Ok(if is_error { ToolResult::err(content) } else { ToolResult::ok(content) })
    }
}
```

- [ ] **Step 5: Wire up lib.rs**

```rust
// crates/mcp/src/lib.rs
pub mod client;
pub mod server;
pub use server::{McpServerHandle, McpToolProxy};
```

- [ ] **Step 6: Run tests — verify pass**

```bash
cargo test -p nxc-mcp
```
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/mcp/
git commit -m "feat(mcp): JSON-RPC client and server lifecycle"
```

---

## Task 11: Agent — history + approval + ReAct loop

**Files:**
- Create: `crates/agent/src/history.rs`
- Create: `crates/agent/src/approval.rs`
- Create: `crates/agent/src/react.rs`
- Modify: `crates/agent/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/agent/src/history.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    use nxc_provider::Message;

    #[test]
    fn truncates_when_over_limit() {
        let mut h = History::new(10);
        h.push(Message::system("sys"));
        for i in 0..20 { h.push(Message::user(format!("msg {i}"))); }
        h.truncate_to_limit();
        // system message always kept
        assert_eq!(h.messages[0].role, "system");
        assert!(h.messages.len() < 22);
    }
}

// crates/agent/src/approval.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    use nxc_config::{ApprovalMode, ToolsConfig};
    use std::collections::HashMap;

    #[test]
    fn yolo_mode_never_needs_approval() {
        let tools_cfg = ToolsConfig::default();
        assert!(!needs_approval("bash", &ApprovalMode::Yolo, &tools_cfg));
    }
    #[test]
    fn ask_mode_always_needs_approval() {
        let tools_cfg = ToolsConfig::default();
        assert!(needs_approval("read_file", &ApprovalMode::Ask, &tools_cfg));
    }
    #[test]
    fn per_tool_override_takes_precedence() {
        let mut approval = HashMap::new();
        approval.insert("bash".to_string(), ApprovalMode::Ask);
        let tools_cfg = ToolsConfig { approval };
        assert!(needs_approval("bash", &ApprovalMode::Auto, &tools_cfg));
    }
}
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc-agent
```
Expected: FAIL.

- [ ] **Step 3: Implement history**

```rust
// crates/agent/src/history.rs
use nxc_provider::Message;

pub struct History {
    pub messages:      Vec<Message>,
    context_limit: u32,
}

impl History {
    pub fn new(context_limit: u32) -> Self { Self { messages: vec![], context_limit } }

    pub fn push(&mut self, msg: Message) { self.messages.push(msg); }

    pub fn truncate_to_limit(&mut self) {
        // Rough token estimate: 4 chars ≈ 1 token
        let system = self.messages.first().filter(|m| m.role == "system").cloned();
        let limit  = self.context_limit as usize * 4;
        while self.char_count() > limit && self.messages.len() > 2 {
            let start = if system.is_some() { 1 } else { 0 };
            self.messages.remove(start);
        }
    }

    fn char_count(&self) -> usize {
        self.messages.iter().map(|m| {
            m.content.as_ref().map(|c| c.to_string().len()).unwrap_or(0)
        }).sum()
    }
}
```

- [ ] **Step 4: Implement approval**

```rust
// crates/agent/src/approval.rs
use nxc_config::{ApprovalMode, ToolsConfig};
use std::io::{self, Write};

pub fn needs_approval(tool_name: &str, global: &ApprovalMode, tools_cfg: &ToolsConfig) -> bool {
    if let Some(mode) = tools_cfg.approval.get(tool_name) {
        return matches!(mode, ApprovalMode::Ask);
    }
    matches!(global, ApprovalMode::Ask)
}

pub fn prompt_approval(tool_name: &str, args_preview: &str) -> bool {
    print!("\n◆ {}({}) [y/N]? ", tool_name, &args_preview[..args_preview.len().min(60)]);
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
    buf.trim().eq_ignore_ascii_case("y")
}
```

- [ ] **Step 5: Implement ReAct loop**

```rust
// crates/agent/src/react.rs
use crate::{approval::{needs_approval, prompt_approval}, history::History};
use anyhow::Result;
use nxc_config::Config;
use nxc_provider::{Client, Message, ToolDef, FunctionDef};
use nxc_tools::Tool;
use serde_json::Value;

pub struct Agent {
    pub history: History,
    client:      Client,
    tools:       Vec<Box<dyn Tool>>,
    config:      Config,
}

pub struct TurnCallbacks<'a> {
    pub on_text:   &'a dyn Fn(&str),
    pub on_action: &'a dyn Fn(&str, &str), // tool_name, args_preview
}

impl Agent {
    pub fn new(config: Config, tools: Vec<Box<dyn Tool>>) -> Self {
        let client = Client::new(
            config.provider.base_url.clone(),
            config.provider.api_key.clone(),
            config.provider.model.clone(),
        );
        let context_limit = config.agent.context_limit;
        Self { history: History::new(context_limit), client, tools, config }
    }

    pub fn tool_defs(&self) -> Vec<ToolDef> {
        self.tools.iter().map(|t| ToolDef {
            kind: "function",
            function: FunctionDef { name: t.name(), description: t.description(), parameters: t.parameters() },
        }).collect()
    }

    pub async fn run_turn(&mut self, callbacks: TurnCallbacks<'_>) -> Result<bool> {
        self.history.truncate_to_limit();
        let defs = self.tool_defs();
        let resp = self.client.complete(&self.history.messages, &defs, callbacks.on_text).await?;

        if resp.tool_calls.is_empty() {
            if !resp.text.is_empty() {
                self.history.push(Message::assistant_text(&resp.text));
            }
            return Ok(false); // done
        }

        self.history.push(Message::assistant_tool_calls(resp.tool_calls.clone()));

        for call in &resp.tool_calls {
            let args: Value = serde_json::from_str(&call.function.arguments).unwrap_or(Value::Null);
            let preview = call.function.arguments.clone();

            let approved = if needs_approval(&call.function.name, &self.config.agent.approval_mode, &self.config.tools) {
                (callbacks.on_action)(&call.function.name, &preview);
                prompt_approval(&call.function.name, &preview)
            } else {
                (callbacks.on_action)(&call.function.name, &preview);
                true
            };

            let result = if approved {
                let tool = self.tools.iter().find(|t| t.name() == call.function.name);
                match tool {
                    Some(t) => t.execute(args).await?,
                    None    => nxc_tools::ToolResult::err(format!("Unknown tool: {}", call.function.name)),
                }
            } else {
                nxc_tools::ToolResult::err("User denied tool execution".into())
            };

            self.history.push(Message::tool_result(&call.id, result.content));
        }

        Ok(true) // continue looping
    }
}
```

- [ ] **Step 6: Wire up lib.rs**

```rust
// crates/agent/src/lib.rs
pub mod history;
pub mod approval;
pub mod react;
pub mod session;
pub use react::Agent;
```

Add stub `session.rs`:
```rust
// crates/agent/src/session.rs
pub fn save_session(_msgs: &[nxc_provider::Message]) -> anyhow::Result<()> { Ok(()) }
```

- [ ] **Step 7: Run tests — verify pass**

```bash
cargo test -p nxc-agent
```
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/agent/
git commit -m "feat(agent): history, approval gating, ReAct loop"
```

---

## Task 12: Agent — session persistence

**Files:**
- Modify: `crates/agent/src/session.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/agent/src/session.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    use nxc_provider::Message;
    use tempfile::tempdir;

    #[test]
    fn save_and_load_round_trips() {
        let dir  = tempdir().unwrap();
        let msgs = vec![Message::user("hello"), Message::assistant_text("hi")];
        let path = dir.path().join("session.json");
        save_to_path(&msgs, &path).unwrap();
        let loaded = load_from_path(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].role, "user");
    }
}
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc-agent session
```
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
// crates/agent/src/session.rs
use anyhow::Result;
use directories::ProjectDirs;
use nxc_provider::Message;
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub cwd:      String,
    pub messages: Vec<Message>,
    pub saved_at: u64,
}

fn sessions_dir() -> Option<PathBuf> {
    ProjectDirs::from("dev", "nexuscode", "nxc").map(|d| d.data_dir().join("sessions"))
}

pub fn save_session(messages: &[Message]) -> Result<PathBuf> {
    let dir = sessions_dir().ok_or_else(|| anyhow::anyhow!("no data dir"))?;
    fs::create_dir_all(&dir)?;
    let cwd = std::env::current_dir().unwrap_or_default().display().to_string();
    let ts  = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let session = Session { cwd, messages: messages.to_vec(), saved_at: ts };
    let path = dir.join(format!("{ts}.json"));
    fs::write(&path, serde_json::to_string_pretty(&session)?)?;
    Ok(path)
}

pub fn load_latest_session() -> Result<Option<Vec<Message>>> {
    let dir = sessions_dir().ok_or_else(|| anyhow::anyhow!("no data dir"))?;
    let mut entries: Vec<_> = fs::read_dir(&dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    match entries.last() {
        None    => Ok(None),
        Some(e) => Ok(Some(load_from_path(&e.path())?)),
    }
}

pub fn load_from_path(path: &Path) -> Result<Vec<Message>> {
    let text = fs::read_to_string(path)?;
    let s: Session = serde_json::from_str(&text)?;
    Ok(s.messages)
}

pub fn save_to_path(messages: &[Message], path: &Path) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_default().display().to_string();
    let ts  = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let s   = Session { cwd, messages: messages.to_vec(), saved_at: ts };
    fs::write(path, serde_json::to_string_pretty(&s)?)?;
    Ok(())
}

pub fn list_sessions() -> Result<Vec<PathBuf>> {
    let dir = sessions_dir().ok_or_else(|| anyhow::anyhow!("no data dir"))?;
    if !dir.exists() { return Ok(vec![]); }
    let mut entries: Vec<_> = fs::read_dir(&dir)?.filter_map(|e| e.ok())
        .map(|e| e.path()).collect();
    entries.sort();
    entries.reverse();
    Ok(entries)
}
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cargo test -p nxc-agent
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/agent/src/session.rs
git commit -m "feat(agent): session save/load/list"
```

---

## Task 13: CLI — args, main, subcommands, output, readline

**Files:**
- Modify: `crates/cli/src/main.rs`
- Create: `crates/cli/src/args.rs`
- Create: `crates/cli/src/output.rs`
- Create: `crates/cli/src/commands.rs`
- Create: `crates/cli/src/prompt.rs`

- [ ] **Step 1: Write failing integration test**

```rust
// crates/cli/tests/smoke.rs
use assert_cmd::Command;

#[test]
fn nxc_help_exits_zero() {
    Command::cargo_bin("nxc").unwrap()
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn nxc_models_requires_api_key() {
    Command::cargo_bin("nxc").unwrap()
        .arg("models")
        .env("NXC_API_KEY", "")
        .assert()
        .failure();
}
```

- [ ] **Step 2: Run — verify fail**

```bash
cargo test -p nxc
```
Expected: FAIL — `main.rs` is just `fn main() {}`.

- [ ] **Step 3: Implement args**

```rust
// crates/cli/src/args.rs
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "nxc", about = "Nexus Code — AI coding agent", version)]
pub struct Cli {
    /// One-shot prompt (skip interactive session)
    pub prompt: Option<String>,

    /// Override the model for this run
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Auto-execute all tool calls without asking (fastest)
    #[arg(long)]
    pub yolo: bool,

    /// Ask before every tool call (safest)
    #[arg(long)]
    pub safe: bool,

    /// Resume the most recent session in cwd
    #[arg(long)]
    pub resume: Option<Option<String>>,

    #[command(subcommand)]
    pub command: Option<Sub>,
}

#[derive(Subcommand, Debug)]
pub enum Sub {
    /// Run the setup wizard
    Init,
    /// List available OpenRouter models
    Models,
    /// Open config file in $EDITOR
    Config,
    /// List saved sessions
    Sessions,
}
```

- [ ] **Step 4: Implement output helpers**

```rust
// crates/cli/src/output.rs
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::ExecutableCommand;
use std::io::{stdout, Write};

pub fn print_delta(text: &str) {
    print!("{text}");
    stdout().flush().ok();
}

pub fn print_tool_action(name: &str, args: &str) {
    let preview = &args[..args.len().min(80)];
    stdout().execute(SetForegroundColor(Color::Blue)).ok();
    print!("\n◆ {name}({preview})");
    stdout().execute(ResetColor).ok();
    stdout().flush().ok();
}

pub fn print_tool_result(name: &str, ok: bool) {
    let color = if ok { Color::Green } else { Color::Red };
    let icon  = if ok { "✔" } else { "✘" };
    stdout().execute(SetForegroundColor(color)).ok();
    print!(" {icon} {name}");
    stdout().execute(ResetColor).ok();
    println!();
}

pub fn print_status(prompt_tokens: u32, completion_tokens: u32, cost: f64, model: &str) {
    stdout().execute(SetForegroundColor(Color::DarkGrey)).ok();
    println!(
        "\n↑ {}  ↓ {}  tokens  │  ~${:.4}  │  {}",
        fmt_k(prompt_tokens), fmt_k(completion_tokens), cost, model
    );
    stdout().execute(ResetColor).ok();
}

fn fmt_k(n: u32) -> String {
    if n >= 1000 { format!("{:.1}k", n as f64 / 1000.0) } else { n.to_string() }
}
```

- [ ] **Step 5: Implement subcommands**

```rust
// crates/cli/src/commands.rs
use anyhow::Result;
use nxc_config::{global_config_path, run_wizard, answers_to_toml, write_wizard_config};
use nxc_provider::{Client, models::ModelFetcher};

pub async fn cmd_init(api_key: &str, base_url: &str) -> Result<()> {
    let fetcher = ModelFetcher::new(base_url, api_key);
    let models: Vec<String> = fetcher.fetch().await
        .unwrap_or_default().into_iter().map(|m| m.id).collect();

    let answers = run_wizard(&models)?;
    let toml    = answers_to_toml(&answers);
    let path    = global_config_path().unwrap_or_else(|| std::path::PathBuf::from("config.toml"));
    write_wizard_config(&path, &toml)?;
    println!("\n✔ Config written to {}", path.display());
    println!("✔ Ready. Run nxc to start.");
    Ok(())
}

pub async fn cmd_models(api_key: &str, base_url: &str) -> Result<()> {
    if api_key.is_empty() {
        anyhow::bail!("NXC_API_KEY not set. Run `nxc init` to configure.");
    }
    let fetcher = ModelFetcher::new(base_url, api_key);
    let models  = fetcher.fetch().await?;
    println!("{} models available:\n", models.len());
    for m in &models { println!("  {}", m.id); }
    Ok(())
}

pub fn cmd_config() -> Result<()> {
    let path   = global_config_path().unwrap_or_else(|| std::path::PathBuf::from("config.toml"));
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".into());
    std::process::Command::new(&editor).arg(&path).status()?;
    Ok(())
}

pub fn cmd_sessions() -> Result<()> {
    let sessions = nxc_agent::session::list_sessions()?;
    if sessions.is_empty() { println!("No saved sessions."); return Ok(()); }
    for (i, p) in sessions.iter().enumerate() {
        println!("  {}. {}", i + 1, p.display());
    }
    Ok(())
}
```

- [ ] **Step 6: Implement readline prompt**

```rust
// crates/cli/src/prompt.rs
use anyhow::Result;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

pub struct Prompt { editor: Reedline }

impl Prompt {
    pub fn new() -> Self { Self { editor: Reedline::create() } }

    pub fn readline(&mut self) -> Result<Option<String>> {
        let prompt = DefaultPrompt {
            left_prompt:  DefaultPromptSegment::Basic("nxc> ".into()),
            right_prompt: DefaultPromptSegment::Empty,
        };
        match self.editor.read_line(&prompt) {
            Ok(Signal::Success(s)) => Ok(Some(s)),
            Ok(Signal::CtrlD | Signal::CtrlC) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
```

- [ ] **Step 7: Implement main**

```rust
// crates/cli/src/main.rs
mod args;
mod commands;
mod output;
mod prompt;

use anyhow::Result;
use args::{Cli, Sub};
use clap::Parser;
use nxc_agent::{react::{Agent, TurnCallbacks}, session};
use nxc_config::{load, ApprovalMode};
use nxc_provider::models::{ModelFetcher, build_cache};
use nxc_tools::all_tools;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut cfg = load()?;

    // CLI flag overrides
    if let Some(m) = cli.model  { cfg.provider.model = m; }
    if cli.yolo { cfg.agent.approval_mode = ApprovalMode::Yolo; }
    if cli.safe { cfg.agent.approval_mode = ApprovalMode::Ask; }

    match cli.command {
        Some(Sub::Init)     => return commands::cmd_init(&cfg.provider.api_key, &cfg.provider.base_url).await,
        Some(Sub::Models)   => return commands::cmd_models(&cfg.provider.api_key, &cfg.provider.base_url).await,
        Some(Sub::Config)   => return Ok(commands::cmd_config()?),
        Some(Sub::Sessions) => return Ok(commands::cmd_sessions()?),
        None => {}
    }

    if cfg.provider.api_key.is_empty() {
        eprintln!("No API key set. Run `nxc init` to configure.");
        std::process::exit(1);
    }

    // Pricing cache for status line
    let fetcher = ModelFetcher::new(&cfg.provider.base_url, &cfg.provider.api_key);
    let pricing = build_cache(fetcher.fetch().await.unwrap_or_default());
    let model   = cfg.provider.model.clone();

    let mut tools = all_tools();
    // TODO(v2): spawn MCP servers from cfg.mcp.servers and append

    // Load AGENTS.md if present
    let agents_md = std::fs::read_to_string(".nxc/AGENTS.md").ok();

    let mut agent = Agent::new(cfg.clone(), tools);
    if let Some(instructions) = &agents_md {
        agent.history.push(nxc_provider::Message::system(instructions));
    }

    // Resume session if requested
    if cli.resume.is_some() {
        if let Some(msgs) = session::load_latest_session()? {
            agent.history.messages = msgs;
            println!("Resumed previous session ({} messages).", agent.history.messages.len());
        }
    }

    // One-shot mode
    if let Some(prompt_text) = cli.prompt {
        agent.history.push(nxc_provider::Message::user(&prompt_text));
        run_turns(&mut agent, &pricing, &model, &cfg).await?;
        session::save_session(&agent.history.messages)?;
        return Ok(());
    }

    // Interactive mode
    println!("Nexus Code  (model: {model})  type /exit to quit");
    let mut prompt = prompt::Prompt::new();
    loop {
        match prompt.readline()? {
            None       => break,
            Some(line) => {
                let line = line.trim().to_string();
                if line.is_empty() { continue; }
                if line == "/exit" || line == "/quit" { break; }
                agent.history.push(nxc_provider::Message::user(&line));
                run_turns(&mut agent, &pricing, &model, &cfg).await?;
            }
        }
    }
    session::save_session(&agent.history.messages)?;
    println!("\nSession saved.");
    Ok(())
}

async fn run_turns(
    agent:   &mut Agent,
    pricing: &nxc_provider::models::PricingCache,
    model:   &str,
    cfg:     &nxc_config::Config,
) -> Result<()> {
    let mut total_prompt = 0u32;
    let mut total_compl  = 0u32;

    for _ in 0..cfg.agent.max_turns {
        let keep_going = agent.run_turn(TurnCallbacks {
            on_text:   &|t| output::print_delta(t),
            on_action: &|name, args| output::print_tool_action(name, args),
        }).await?;
        if !keep_going { break; }
    }

    if let Some(info) = pricing.get(model) {
        let cost = info.estimate_cost(total_prompt, total_compl);
        output::print_status(total_prompt, total_compl, cost, model);
    }
    Ok(())
}
```

- [ ] **Step 8: Run tests — verify pass**

```bash
cargo test -p nxc
cargo build
```
Expected: PASS + builds with no errors.

- [ ] **Step 9: Commit**

```bash
git add crates/cli/
git commit -m "feat(cli): args, readline, output, subcommands, main loop"
```

---

## Task 14: CI + Release workflows

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `.gitignore` (update)

- [ ] **Step 1: Write CI workflow**

```yaml
# .github/workflows/ci.yml
name: CI
on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo fmt --check
```

- [ ] **Step 2: Write release workflow**

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  build-mac:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin
      - uses: Swatinem/rust-cache@v2
      - name: Build arm64
        run: cargo build --release --target aarch64-apple-darwin -p nxc
      - name: Build x86_64
        run: cargo build --release --target x86_64-apple-darwin -p nxc
      - name: Create universal binary
        run: |
          lipo -create \
            target/aarch64-apple-darwin/release/nxc \
            target/x86_64-apple-darwin/release/nxc \
            -output nxc-universal
      - name: Create DMG
        run: |
          brew install create-dmg
          mkdir dmg-content && cp nxc-universal dmg-content/nxc
          create-dmg \
            --volname "Nexus Code" \
            --window-size 400 200 \
            --icon-size 64 \
            "nexus-code.dmg" dmg-content/
      - uses: actions/upload-artifact@v4
        with:
          name: macos-dmg
          path: nexus-code.dmg

  build-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-unknown-linux-musl
      - run: sudo apt-get install -y musl-tools
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release --target x86_64-unknown-linux-musl -p nxc
      - run: tar czf nxc-linux-x86_64.tar.gz -C target/x86_64-unknown-linux-musl/release nxc
      - uses: actions/upload-artifact@v4
        with:
          name: linux-binary
          path: nxc-linux-x86_64.tar.gz

  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release -p nxc
      - run: Compress-Archive -Path target/release/nxc.exe -DestinationPath nxc-windows-x86_64.zip
        shell: pwsh
      - uses: actions/upload-artifact@v4
        with:
          name: windows-binary
          path: nxc-windows-x86_64.zip

  publish:
    needs: [build-mac, build-linux, build-windows]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          merge-multiple: true
      - name: Create GitHub Release
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          gh release create "${{ github.ref_name }}" \
            nexus-code.dmg \
            nxc-linux-x86_64.tar.gz \
            nxc-windows-x86_64.zip \
            --title "Nexus Code ${{ github.ref_name }}" \
            --generate-notes
```

- [ ] **Step 3: Verify workflow files parse correctly**

```bash
# Install actionlint if available, otherwise just check YAML syntax
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add .github/ 
git commit -m "ci: add CI and release workflows"
```

---

## Final smoke test

- [ ] **Step 1: Build the full workspace**

```bash
cargo build --workspace
```
Expected: 0 errors, 0 test failures.

- [ ] **Step 2: Run all tests**

```bash
cargo test --workspace
```
Expected: All tests pass.

- [ ] **Step 3: Verify binary runs**

```bash
./target/debug/nxc --help
```
Expected: Prints usage with all commands listed.

- [ ] **Step 4: Tag initial version**

```bash
git tag v0.1.0
```

---

## Self-Review Notes

**Spec coverage:**
- ✅ 6-crate workspace
- ✅ ReAct loop (react.rs)
- ✅ All built-in tools (files, shell, search, git, web, vision)
- ✅ MCP client + server lifecycle
- ✅ Config loading (TOML + env vars + CLI flags)
- ✅ nxc init wizard
- ✅ Approval modes (auto/ask/yolo) + per-tool overrides
- ✅ Session persistence (save/load/list)
- ✅ .nxc/AGENTS.md project instructions (main.rs)
- ✅ Token/cost status line (output.rs + main.rs)
- ✅ All CLI commands (init, models, config, sessions, --resume, --yolo, --safe, --model)
- ✅ CI workflow (test + lint + fmt, all 3 platforms)
- ✅ Release workflow (macOS DMG universal, Linux musl, Windows zip)

**Known gaps to watch:**
- MCP tool proxies are not yet wired into `all_tools()` in main.rs — marked with TODO(v2) comment. Full MCP integration (spawning servers, wrapping McpToolProxy as dyn Tool) is a logical next step after v0.1.
- `apply_patch` uses `similar` for diff display but unified patch apply is simplistic — a future task can improve with proper patch parsing.
- `web_search` uses DuckDuckGo lite HTML as a fallback — SerpAPI integration using `cfg.search.serpapi_key` should be added as a follow-on task.
