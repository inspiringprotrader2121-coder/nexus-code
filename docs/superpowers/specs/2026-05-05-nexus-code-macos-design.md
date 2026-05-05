# Nexus Code macOS — Design Spec
_Date: 2026-05-05_

## Overview

Nexus Code starts as a native macOS coding-agent app. The app provides the project picker, chat surface, run log, approvals, settings, and session browser. A Rust agent engine does the actual coding work: provider calls, ReAct loop, file tools, shell tools, git tools, MCP tools, and session persistence.

The existing terminal-first `nxc` design remains useful for the engine, but the first product target is now `Nexus Code.app`. A CLI can still ship later as a thin wrapper over the same Rust engine.

## Goals

- Build a real Mac app first, not only a command-line tool.
- Keep the agent core in Rust so it can later power a CLI, daemon, or other desktop shell.
- Use the existing LLM Cache Gateway as the first provider endpoint.
- Make the cache endpoint configurable so it can move from local development to a dedicated server later.
- Preserve the existing agent-plan direction: OpenAI-compatible provider, ReAct loop, built-in tools, MCP, approvals, and saved sessions.

## Non-Goals

- The app will not embed the cache gateway in the first version.
- The first version will not require App Store packaging.
- The first version will not implement multi-user/team hosting.
- The first version will not depend on cloud infrastructure beyond the configured LLM/cache endpoint.

## Product Shape

The first screen is the working app:

- Sidebar with recent projects and saved sessions.
- Main task/chat panel for giving the agent work.
- Run log showing streamed assistant output, tool calls, command output, and errors.
- Approval prompts for risky tools such as shell commands, file writes, deletes, and commits.
- Settings panel for provider, cache gateway, model, approval mode, and MCP servers.

The app launches into a local-first workflow: pick a folder, describe the change, review tool actions, and inspect the result.

## Architecture

```text
Nexus Code.app
  SwiftUI macOS shell
    App state, windows, settings, approvals, project/session UI
    Rust engine bridge
      nxc-agent-core
        config
        provider
        ReAct loop
        tools
        MCP
        session store
      LLM Cache Gateway
        local default: http://127.0.0.1:8787/v1
        future: dedicated server URL
```

## Repository Layout

```text
NexusCode/
├── Package.swift                         # Swift app package
├── Sources/
│   ├── NexusCodeApp/                     # SwiftUI app entry
│   ├── NexusCodeUI/                      # views, navigation, settings
│   └── NexusCodeBridge/                  # Rust process/FFI bridge
├── rust/
│   ├── Cargo.toml                        # Rust workspace
│   └── crates/
│       ├── agent-core/                   # ReAct loop and orchestration
│       ├── config/                       # settings, env, project config
│       ├── provider/                     # OpenAI-compatible HTTP/SSE client
│       ├── tools/                        # file, shell, search, git, web, vision
│       ├── mcp/                          # MCP subprocess client
│       └── engine-cli/                   # local bridge executable
├── docs/
└── .github/workflows/
```

The first bridge should be a local executable protocol, not direct FFI. The Swift app spawns the Rust `engine-cli` helper and exchanges newline-delimited JSON events. This is easier to debug, safer during early development, and still lets the Rust core become a standalone CLI later.

## Engine Bridge

The Swift app starts `engine-cli` with a selected project path and sends JSON commands over stdin:

```json
{ "type": "start_task", "projectPath": "/Users/johnboy/Documents/MyApp", "prompt": "Fix login tests" }
{ "type": "approve_tool", "toolCallId": "tool_123", "approved": true }
{ "type": "cancel_task" }
```

The engine streams JSON events over stdout:

```json
{ "type": "assistant_delta", "text": "I will inspect the failing tests..." }
{ "type": "tool_requested", "id": "tool_123", "name": "bash", "summary": "npm test" }
{ "type": "tool_output", "id": "tool_123", "stream": "stdout", "text": "..." }
{ "type": "task_finished", "sessionId": "..." }
{ "type": "error", "message": "..." }
```

stderr is reserved for engine diagnostics and shown in the app's developer log.

## LLM Cache Gateway Integration

The provider remains OpenAI-compatible. The default base URL is:

```text
http://127.0.0.1:8787/v1
```

The app stores a configurable cache gateway URL so the same build can later point to a dedicated server. The settings UI includes:

- Cache gateway base URL.
- Gateway API key.
- Provider selector/header.
- Model alias.
- Cache mode: `bypass`, `private`, or `public`.
- Optional workflow and prompt-template labels.
- Health check status using the gateway admin or health endpoint when configured.

Requests sent through the gateway include cache headers when enabled:

```text
Authorization: Bearer <gateway-api-key>
X-LLM-Cache-Mode: private
X-LLM-Provider: anthropic
X-LLM-Model-Alias: claude-sonnet
X-LLM-Workflow: coding-agent
```

Secrets, file-heavy context, tool-call turns, and attachment-like turns default to private or bypass mode. Public cache must be explicit.

## Agent Loop

The Rust engine follows the same ReAct loop as the earlier plan:

1. Send messages and tool definitions to the configured OpenAI-compatible endpoint.
2. Stream assistant deltas to Swift.
3. Parse tool calls.
4. Ask Swift for approval when policy requires it.
5. Execute tools in the selected project folder.
6. Stream tool output back to Swift.
7. Append observations and continue until the model finishes.

The first version supports one active task per app window. Cancellation stops the provider stream, terminates running child processes where possible, saves partial session state, and reports the final status.

## Tools

The first version includes:

- `read_file`
- `write_file`
- `apply_patch`
- `list_dir`
- `delete_file`
- `bash`
- `grep_codebase`
- `find_files`
- `git_status`
- `git_diff`
- `git_log`

Later versions add:

- `git_commit`
- `git_branch`
- `fetch_url`
- `web_search`
- `read_image`
- MCP plugin tools

## Approval Modes

The app exposes three modes:

- Ask: every tool call needs approval.
- Auto: low-risk read/search tools run automatically; write, delete, shell, and git mutation ask.
- Yolo: all tools execute automatically, with visible logs.

Per-tool overrides are stored in settings and passed to the Rust engine at task start.

## Data Storage

macOS app data lives under:

```text
~/Library/Application Support/Nexus Code/
```

Stored data includes:

- App settings.
- Recent project list.
- Session metadata.
- Session transcripts.
- Engine logs.

Project-specific instructions may live in:

```text
.nexus/AGENTS.md
.nexus/config.toml
```

The engine also supports `.nxc` aliases for compatibility with the earlier CLI design.

## Error Handling

- Provider or cache gateway connection failures show a clear banner with the base URL and retry action.
- Cache health failures do not prevent running if the user chooses bypass mode or another endpoint.
- Tool failures stream back into the run log and become observations for the model.
- Permission failures explain which folder or command failed.
- Engine crashes are captured by Swift, logged, and surfaced with a restart action.

## Testing

Rust tests cover config loading, provider request construction, cache headers, tool execution, approval policy, session persistence, and bridge event encoding.

Swift tests cover settings persistence, bridge process lifecycle, event decoding, view models, and approval state transitions.

End-to-end smoke tests run the engine against a mock OpenAI-compatible server and verify that the app can start a task, receive streamed events, approve a tool, and finish.

## First Milestone

The first usable milestone is a local development macOS app that can:

1. Select a project folder.
2. Configure `http://127.0.0.1:8787/v1` or another cache gateway URL.
3. Send a coding task to the Rust engine.
4. Stream assistant output and tool logs.
5. Ask for approval before risky tools.
6. Read/list/search files and run shell commands.
7. Save a session transcript under Application Support.

This is enough to prove the Mac app shape, the Rust engine bridge, and the cache gateway integration before broadening tool coverage and packaging.
