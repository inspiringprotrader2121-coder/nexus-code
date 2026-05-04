# Nexus Code — Design Spec
_Date: 2026-05-04_

## Overview

Nexus Code (`nxc`) is a Rust-based AI coding agent for the terminal. It connects to any OpenAI-compatible API (defaulting to OpenRouter) and operates as a ReAct loop — reasoning, calling tools, observing results, and repeating until the task is done. It ships as a single binary with no runtime dependency.

---

## Architecture

### Workspace layout

```
nexus-code/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── cli/                    # entry point, arg parsing (clap), streaming output, readline (reedline)
│   ├── agent/                  # ReAct loop, conversation state, tool dispatch, session management
│   ├── tools/                  # all built-in tool implementations
│   ├── mcp/                    # MCP client, plugin discovery, subprocess lifecycle
│   ├── provider/               # OpenRouter HTTP client, streaming SSE, token counting
│   └── config/                 # TOML loading, env var overrides, nxc init wizard
├── .github/
│   └── workflows/
│       ├── ci.yml              # test + lint on every PR
│       └── release.yml         # cross-platform builds + publish on git tag
└── docs/
```

### Crate responsibilities

| Crate | Job |
|-------|-----|
| `cli` | Parse args, manage readline prompt, stream tokens to terminal, display cost counter |
| `agent` | ReAct loop, conversation history, tool call detection, approval gating, session persistence |
| `tools` | Implement each built-in tool as a struct implementing a `Tool` trait |
| `mcp` | Spawn MCP server subprocesses, speak JSON-RPC, expose their tools via the same `Tool` trait |
| `provider` | HTTP client for OpenRouter, SSE streaming, token counting, model listing |
| `config` | Load and merge config layers, run `nxc init` wizard, validate |

### Key dependencies

| Purpose | Crate |
|---------|-------|
| Async runtime | `tokio` |
| HTTP + SSE streaming | `reqwest` |
| CLI args | `clap` |
| Interactive readline | `reedline` |
| Terminal control | `crossterm` |
| Config + serialization | `serde`, `toml`, `serde_json` |
| Gitignore-aware search | `ignore` |
| Diff display | `similar` |
| Git operations | `git2` |
| Platform data dirs | `directories` |

---

## Agent Loop (ReAct)

Each turn follows this cycle:

1. **THINK** — send current message history + tool definitions to OpenRouter, stream the response
2. **PARSE** — detect `tool_use` blocks in the streamed response
3. **APPROVE** — check the active approval mode and prompt the user if required
4. **ACT** — dispatch to the matched built-in tool or MCP plugin, capture result
5. **OBSERVE** — append the tool result to conversation history, loop back to step 1
6. **DONE** — when the model responds with no tool calls, stream the final answer and stop

Maximum turns is configurable (`agent.max_turns`, default 50). If the limit is hit, the agent reports what it completed and stops cleanly.

---

## Built-in Tools

All tools implement a shared `Tool` trait: `name()`, `description()`, `parameters()` (JSON Schema), `execute()`.

### File tools
- `read_file(path)` — read file contents
- `write_file(path, content)` — write or overwrite a file
- `apply_patch(path, patch)` — apply a unified diff to a file
- `list_dir(path)` — list directory contents
- `delete_file(path)` — delete a file

### Shell
- `bash(command)` — run any shell command, stream stdout/stderr live to the terminal

### Search
- `grep_codebase(pattern, path?)` — ripgrep-style text search, gitignore-aware via `ignore` crate
- `find_files(glob)` — find files by glob pattern, gitignore-aware

### Web
- `web_search(query)` — search the web (via OpenRouter's built-in search or SerpAPI key in config)
- `fetch_url(url)` — fetch a URL and return its content as markdown

### Git
- `git_status()` — working tree status
- `git_diff(path?)` — staged and unstaged diffs
- `git_log(n?)` — recent commit history
- `git_commit(message)` — stage all changes and commit
- `git_branch(name?)` — list or create branches

### Vision
- `read_image(path)` — base64-encode a local image file for vision-capable models

---

## MCP Plugin System

MCP servers are declared in `config.toml`. On session start, `nxc` spawns each server as a subprocess and connects via JSON-RPC (stdio transport). The server's tools are discovered and registered alongside built-ins — the agent sees no difference between a built-in and an MCP tool.

```toml
[[mcp.servers]]
name    = "linear"
command = "npx"
args    = ["-y", "@linear/mcp-server"]
env     = { LINEAR_API_KEY = "${LINEAR_API_KEY}" }
```

MCP subprocesses are killed cleanly when the session ends.

---

## Approval Modes

Three global modes, set by flag or config:

| Mode | Behaviour |
|------|-----------|
| `--safe` | Every tool call pauses: `◆ bash("npm test") [y/N]?` |
| `auto` (default) | Executes immediately, prints confirmation: `✔ wrote src/main.rs (42 lines)` |
| `--yolo` | Silent auto-execute, no interruptions |

Per-tool overrides in `config.toml` take precedence over the global mode:

```toml
[tools.approval]
bash       = "ask"
git_commit = "ask"
read_file  = "auto"
```

---

## Configuration

### Load order (last wins)

1. `~/.config/nxc/config.toml` — global user defaults
2. `.nxc/config.toml` — project-level overrides (safe to commit)
3. Environment variables — `NXC_API_KEY`, `NXC_MODEL`, `NXC_APPROVAL`, etc.
4. CLI flags — `--model`, `--yolo`, `--safe` (highest priority)

### Full config.toml reference

```toml
[provider]
api_key  = "sk-or-..."           # or NXC_API_KEY env var
model    = "anthropic/claude-sonnet-4-6"
base_url = "https://openrouter.ai/api/v1"  # any OpenAI-compat base URL

[agent]
approval_mode = "auto"           # "auto" | "ask" | "yolo"
max_turns     = 50
context_limit = 128000           # drop oldest messages (keep system prompt) when approaching this

[search]
serpapi_key = ""                 # optional — enables web_search via SerpAPI instead of OpenRouter

[tools.approval]
bash       = "ask"
git_commit = "ask"
read_file  = "auto"

[[mcp.servers]]
name    = "linear"
command = "npx"
args    = ["-y", "@linear/mcp-server"]
env     = { LINEAR_API_KEY = "${LINEAR_API_KEY}" }
```

### nxc init wizard

On first run (or `nxc init`), an interactive wizard prompts for API key, default model (browsable list fetched live from OpenRouter), approval mode, and per-tool overrides. Writes the result to `~/.config/nxc/config.toml`.

---

## Project Instructions File

If `.nxc/AGENTS.md` exists in the current working directory, its contents are prepended to the system prompt at the start of every session. This is the primary mechanism for project-specific context:

```markdown
# My Project
- Tests live in /tests, run with `cargo test`
- Never edit files in /generated/
- Always use async/await, never block the thread
- Prefer small focused functions over large ones
```

---

## Session Persistence

Each session is serialised to a platform-appropriate data directory when the user exits:
- **macOS**: `~/Library/Application Support/nxc/sessions/`
- **Linux**: `~/.local/share/nxc/sessions/`
- **Windows**: `%APPDATA%\nxc\sessions\`

Sessions store the full message history and metadata (model, working directory, token counts). The `directories` crate resolves the correct path at runtime.

```
nxc --resume              # resume most recent session in cwd
nxc --resume <session-id> # resume a specific session
nxc sessions              # list saved sessions
```

---

## Token & Cost Display

A status line is rendered at the bottom of the terminal during a session:

```
↑ 4.2k  ↓ 1.1k  tokens  │  ~$0.003  │  anthropic/claude-sonnet-4-6
```

- Token counts are tracked from OpenRouter's `usage` response fields
- Cost is estimated using per-model pricing fetched from OpenRouter's `/models` endpoint on startup and cached locally
- The status line updates after each turn

---

## CLI Commands

```
nxc                              # start interactive session in cwd
nxc "fix the login bug"          # one-shot prompt, then exit
nxc init                         # run setup wizard
nxc models                       # list available OpenRouter models
nxc config                       # open config file in $EDITOR
nxc sessions                     # list saved sessions
nxc --resume                     # resume most recent session in cwd
nxc --yolo "refactor auth"       # full auto, no prompts
nxc --safe                       # always ask before every tool
nxc --model gpt-4o "explain X"   # override model for this run
```

---

## Distribution & Release

### Targets

| Platform | Artifact |
|----------|----------|
| macOS | `nexus-code.dmg` (universal binary: arm64 + x86_64 via `lipo`) |
| Linux | `nxc-linux-x86_64.tar.gz` (static musl binary) |
| Windows | `nxc-windows-x86_64.zip` (MSVC target) |

### Release pipeline (GitHub Actions)

Triggered on a `v*` git tag:

1. Build macOS arm64 + x86_64 on `macos-latest`
2. Combine with `lipo` into universal binary
3. Package into styled DMG with `create-dmg`
4. Build Linux static binary on `ubuntu-latest` (musl target)
5. Build Windows binary on `windows-latest`
6. Upload all artifacts to GitHub Release via `gh release create`
7. Auto-update Homebrew tap formula

### Install methods

```bash
# Homebrew (recommended)
brew install your-tap/nxc

# macOS — download nexus-code.dmg from GitHub Releases, drag nxc to /usr/local/bin

# curl install script
curl -fsSL https://nexuscode.dev/install.sh | sh

# Cargo
cargo install nexus-code
```

---

## Out of Scope (v1)

- Semantic / embedding-based code search
- Multi-agent spawning / parallel sub-agents
- Built-in web UI or TUI panels
- LSP integration
- Clipboard access
