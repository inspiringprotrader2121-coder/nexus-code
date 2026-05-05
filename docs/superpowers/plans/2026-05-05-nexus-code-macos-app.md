# Nexus Code macOS App Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first working `Nexus Code.app`: a native SwiftUI macOS shell that runs a Rust coding-agent engine and talks to the existing LLM Cache Gateway through an OpenAI-compatible endpoint.

**Architecture:** Swift owns the macOS app experience: windows, project picker, chat/task form, settings, approvals, logs, and session display. Rust owns the engine: config, cache-aware provider settings, bridge events, basic tools, and session persistence. The first Swift/Rust boundary is a newline-delimited JSON process protocol through a bundled `engine-cli` helper.

**Tech Stack:** Swift 5.10+, SwiftUI, XCTest, Rust 2021, Cargo workspace, Tokio, Serde, Reqwest, Anyhow, Directories, Tempfile.

---

## File Map

```text
Package.swift
Sources/
  NexusCodeApp/
    NexusCodeApp.swift
  NexusCodeUI/
    ContentView.swift
    SettingsView.swift
    ProjectPickerView.swift
    RunLogView.swift
    AppModels.swift
    AppSettings.swift
  NexusCodeBridge/
    EngineBridge.swift
    EngineEvents.swift
Tests/
  NexusCodeUITests/
    SmokeTests.swift
    AppSettingsTests.swift
  NexusCodeBridgeTests/
    SmokeTests.swift
    EngineEventsTests.swift
rust/
  Cargo.toml
  crates/
    config/
      Cargo.toml
      src/lib.rs
    provider/
      Cargo.toml
      src/lib.rs
    tools/
      Cargo.toml
      src/lib.rs
    agent-core/
      Cargo.toml
      src/lib.rs
    engine-cli/
      Cargo.toml
      src/main.rs
```

The first implementation should keep files small. Swift view models stay inside `NexusCodeUI` until they become large enough to split. Rust crates expose narrow structs and functions so the engine can later become a real CLI without rewriting the app.

## Task 1: Swift Package macOS App Scaffold

**Files:**
- Create: `Package.swift`
- Create: `Sources/NexusCodeApp/NexusCodeApp.swift`
- Create: `Sources/NexusCodeUI/ContentView.swift`
- Create: `Sources/NexusCodeUI/AppModels.swift`
- Create: `Sources/NexusCodeBridge/EngineEvents.swift`
- Create: `Tests/NexusCodeUITests/SmokeTests.swift`
- Create: `Tests/NexusCodeBridgeTests/SmokeTests.swift`

- [ ] **Step 1: Write the package manifest**

Create `Package.swift`:

```swift
// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "NexusCode",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "NexusCode", targets: ["NexusCodeApp"])
    ],
    targets: [
        .executableTarget(
            name: "NexusCodeApp",
            dependencies: ["NexusCodeUI", "NexusCodeBridge"],
            path: "Sources/NexusCodeApp"
        ),
        .target(
            name: "NexusCodeUI",
            dependencies: ["NexusCodeBridge"],
            path: "Sources/NexusCodeUI"
        ),
        .target(
            name: "NexusCodeBridge",
            path: "Sources/NexusCodeBridge"
        ),
        .testTarget(
            name: "NexusCodeUITests",
            dependencies: ["NexusCodeUI"],
            path: "Tests/NexusCodeUITests"
        ),
        .testTarget(
            name: "NexusCodeBridgeTests",
            dependencies: ["NexusCodeBridge"],
            path: "Tests/NexusCodeBridgeTests"
        )
    ]
)
```

- [ ] **Step 2: Add minimal app models**

Create `Sources/NexusCodeUI/AppModels.swift`:

```swift
import Foundation

public struct ProjectItem: Identifiable, Equatable, Codable {
    public var id: UUID
    public var name: String
    public var path: String

    public init(id: UUID = UUID(), name: String, path: String) {
        self.id = id
        self.name = name
        self.path = path
    }
}

public struct RunLogEntry: Identifiable, Equatable {
    public enum Kind: String {
        case assistant
        case tool
        case stdout
        case stderr
        case error
        case status
    }

    public let id: UUID
    public var kind: Kind
    public var text: String

    public init(id: UUID = UUID(), kind: Kind, text: String) {
        self.id = id
        self.kind = kind
        self.text = text
    }
}
```

- [ ] **Step 3: Add the SwiftUI app entry**

Create `Sources/NexusCodeApp/NexusCodeApp.swift`:

```swift
import SwiftUI
import NexusCodeUI

@main
struct NexusCodeApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 980, minHeight: 680)
        }
        Settings {
            SettingsView()
        }
    }
}
```

- [ ] **Step 4: Add the first content view**

Create `Sources/NexusCodeUI/ContentView.swift`:

```swift
import SwiftUI

public struct ContentView: View {
    @State private var selectedProject: ProjectItem?
    @State private var prompt: String = ""
    @State private var log: [RunLogEntry] = [
        RunLogEntry(kind: .status, text: "Select a project and describe the change.")
    ]

    public init() {}

    public var body: some View {
        NavigationSplitView {
            List(selection: Binding(get: {
                selectedProject?.id
            }, set: { _ in })) {
                Section("Projects") {
                    if let selectedProject {
                        Text(selectedProject.name)
                            .tag(selectedProject.id)
                    } else {
                        Text("No project selected")
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .navigationTitle("Nexus Code")
        } detail: {
            VStack(spacing: 0) {
                HStack {
                    ProjectPickerView(selectedProject: $selectedProject)
                    Spacer()
                }
                .padding()

                RunLogView(entries: log)

                HStack(alignment: .bottom) {
                    TextEditor(text: $prompt)
                        .font(.body)
                        .frame(minHeight: 70, maxHeight: 110)
                        .overlay(RoundedRectangle(cornerRadius: 6).stroke(.quaternary))

                    Button("Run") {
                        log.append(RunLogEntry(kind: .status, text: "Task queued: \(prompt)"))
                        prompt = ""
                    }
                    .keyboardShortcut(.return, modifiers: [.command])
                    .disabled(selectedProject == nil || prompt.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
                .padding()
            }
        }
    }
}
```

- [ ] **Step 5: Add first-pass picker, log, and settings views**

Create `Sources/NexusCodeUI/ProjectPickerView.swift`:

```swift
import SwiftUI

public struct ProjectPickerView: View {
    @Binding var selectedProject: ProjectItem?

    public init(selectedProject: Binding<ProjectItem?>) {
        self._selectedProject = selectedProject
    }

    public var body: some View {
        Button {
            selectedProject = ProjectItem(name: "Coding Agent", path: "/Users/johnboy/Documents/Coding Agent")
        } label: {
            Label(selectedProject?.name ?? "Choose Project", systemImage: "folder")
        }
    }
}
```

Create `Sources/NexusCodeUI/RunLogView.swift`:

```swift
import SwiftUI

public struct RunLogView: View {
    public let entries: [RunLogEntry]

    public init(entries: [RunLogEntry]) {
        self.entries = entries
    }

    public var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 8) {
                ForEach(entries) { entry in
                    Text(entry.text)
                        .font(.system(.body, design: .monospaced))
                        .foregroundStyle(color(for: entry.kind))
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .padding()
        }
        .background(Color(nsColor: .textBackgroundColor))
    }

    private func color(for kind: RunLogEntry.Kind) -> Color {
        switch kind {
        case .assistant: return .primary
        case .tool: return .blue
        case .stdout: return .secondary
        case .stderr, .error: return .red
        case .status: return .secondary
        }
    }
}
```

Create `Sources/NexusCodeUI/SettingsView.swift`:

```swift
import SwiftUI

public struct SettingsView: View {
    public init() {}

    public var body: some View {
        Form {
            Text("Settings will be added in the next task.")
        }
        .padding()
        .frame(width: 520)
    }
}
```

- [ ] **Step 6: Add bridge bootstrap file**

Create `Sources/NexusCodeBridge/EngineEvents.swift`:

```swift
public enum EngineBridgeBootstrap {
    public static let isAvailable = true
}
```

- [ ] **Step 7: Add smoke test files**

Create `Tests/NexusCodeUITests/SmokeTests.swift`:

```swift
import XCTest
@testable import NexusCodeUI

final class SmokeTests: XCTestCase {
    func testProjectItemStoresNameAndPath() {
        let project = ProjectItem(name: "Coding Agent", path: "/Users/johnboy/Documents/Coding Agent")

        XCTAssertEqual(project.name, "Coding Agent")
        XCTAssertEqual(project.path, "/Users/johnboy/Documents/Coding Agent")
    }
}
```

Create `Tests/NexusCodeBridgeTests/SmokeTests.swift`:

```swift
import XCTest
@testable import NexusCodeBridge

final class BridgeSmokeTests: XCTestCase {
    func testBridgeTargetBuilds() {
        XCTAssertTrue(EngineBridgeBootstrap.isAvailable)
    }
}
```

- [ ] **Step 8: Build the Swift package**

Run: `swift build`

Expected: build succeeds and produces `.build/debug/NexusCode`.

- [ ] **Step 9: Commit**

```bash
git add Package.swift Sources Tests
git commit -m "feat: scaffold SwiftUI macOS app"
```

## Task 2: Settings Model for Cache Gateway

**Files:**
- Create: `Sources/NexusCodeUI/AppSettings.swift`
- Modify: `Sources/NexusCodeUI/SettingsView.swift`
- Modify: `Tests/NexusCodeUITests/AppSettingsTests.swift`

- [ ] **Step 1: Write settings tests**

Create `Tests/NexusCodeUITests/AppSettingsTests.swift`:

```swift
import XCTest
@testable import NexusCodeUI

final class AppSettingsTests: XCTestCase {
    func testDefaultsPointAtLocalCacheGateway() {
        let settings = AppSettings.default

        XCTAssertEqual(settings.cacheBaseURL.absoluteString, "http://127.0.0.1:8787/v1")
        XCTAssertEqual(settings.cacheMode, .private)
        XCTAssertEqual(settings.provider, "anthropic")
        XCTAssertEqual(settings.workflow, "coding-agent")
    }

    func testAuthorizationAndCacheHeaders() {
        let settings = AppSettings(
            cacheBaseURL: URL(string: "https://cache.example.com/v1")!,
            gatewayAPIKey: "gateway-key",
            provider: "anthropic",
            modelAlias: "claude-sonnet",
            cacheMode: .private,
            workflow: "coding-agent"
        )

        XCTAssertEqual(settings.requestHeaders["Authorization"], "Bearer gateway-key")
        XCTAssertEqual(settings.requestHeaders["X-LLM-Cache-Mode"], "private")
        XCTAssertEqual(settings.requestHeaders["X-LLM-Provider"], "anthropic")
        XCTAssertEqual(settings.requestHeaders["X-LLM-Model-Alias"], "claude-sonnet")
        XCTAssertEqual(settings.requestHeaders["X-LLM-Workflow"], "coding-agent")
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `swift test --filter AppSettingsTests`

Expected: fail because `AppSettings` does not exist.

- [ ] **Step 3: Implement settings model**

Create `Sources/NexusCodeUI/AppSettings.swift`:

```swift
import Foundation

public struct AppSettings: Equatable, Codable {
    public enum CacheMode: String, Codable, CaseIterable, Identifiable {
        case bypass
        case `private`
        case `public`

        public var id: String { rawValue }
    }

    public var cacheBaseURL: URL
    public var gatewayAPIKey: String
    public var provider: String
    public var modelAlias: String
    public var cacheMode: CacheMode
    public var workflow: String

    public static let `default` = AppSettings(
        cacheBaseURL: URL(string: "http://127.0.0.1:8787/v1")!,
        gatewayAPIKey: "demo-key-change-me",
        provider: "anthropic",
        modelAlias: "claude-sonnet",
        cacheMode: .private,
        workflow: "coding-agent"
    )

    public init(
        cacheBaseURL: URL,
        gatewayAPIKey: String,
        provider: String,
        modelAlias: String,
        cacheMode: CacheMode,
        workflow: String
    ) {
        self.cacheBaseURL = cacheBaseURL
        self.gatewayAPIKey = gatewayAPIKey
        self.provider = provider
        self.modelAlias = modelAlias
        self.cacheMode = cacheMode
        self.workflow = workflow
    }

    public var requestHeaders: [String: String] {
        var headers: [String: String] = [
            "X-LLM-Cache-Mode": cacheMode.rawValue,
            "X-LLM-Provider": provider,
            "X-LLM-Model-Alias": modelAlias,
            "X-LLM-Workflow": workflow
        ]

        if !gatewayAPIKey.isEmpty {
            headers["Authorization"] = "Bearer \(gatewayAPIKey)"
        }

        return headers
    }
}
```

- [ ] **Step 4: Implement settings view**

Replace `Sources/NexusCodeUI/SettingsView.swift`:

```swift
import SwiftUI

public struct SettingsView: View {
    @State private var settings = AppSettings.default

    public init() {}

    public var body: some View {
        Form {
            TextField("Cache Gateway URL", text: Binding(
                get: { settings.cacheBaseURL.absoluteString },
                set: { value in
                    if let url = URL(string: value) {
                        settings.cacheBaseURL = url
                    }
                }
            ))

            SecureField("Gateway API Key", text: $settings.gatewayAPIKey)
            TextField("Provider", text: $settings.provider)
            TextField("Model Alias", text: $settings.modelAlias)

            Picker("Cache Mode", selection: $settings.cacheMode) {
                ForEach(AppSettings.CacheMode.allCases) { mode in
                    Text(mode.rawValue).tag(mode)
                }
            }

            TextField("Workflow", text: $settings.workflow)
        }
        .padding()
        .frame(width: 560)
    }
}
```

- [ ] **Step 5: Run tests**

Run: `swift test --filter AppSettingsTests`

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add Sources/NexusCodeUI Tests/NexusCodeUITests
git commit -m "feat: add cache gateway settings"
```

## Task 3: Rust Workspace and Cache-Aware Config

**Files:**
- Create: `rust/Cargo.toml`
- Create: `rust/crates/config/Cargo.toml`
- Create: `rust/crates/config/src/lib.rs`
- Create: `rust/crates/provider/Cargo.toml`
- Create: `rust/crates/provider/src/lib.rs`
- Create: `rust/crates/tools/Cargo.toml`
- Create: `rust/crates/tools/src/lib.rs`
- Create: `rust/crates/agent-core/Cargo.toml`
- Create: `rust/crates/agent-core/src/lib.rs`
- Create: `rust/crates/engine-cli/Cargo.toml`
- Create: `rust/crates/engine-cli/src/main.rs`

- [ ] **Step 1: Create Rust workspace manifests**

Create `rust/Cargo.toml`:

```toml
[workspace]
members = [
  "crates/config",
  "crates/provider",
  "crates/tools",
  "crates/agent-core",
  "crates/engine-cli"
]
resolver = "2"

[workspace.dependencies]
anyhow = "1"
directories = "5"
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
tokio = { version = "1", features = ["full"] }
```

Create each crate manifest:

```toml
# rust/crates/config/Cargo.toml
[package]
name = "nexus-config"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
directories = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
```

```toml
# rust/crates/provider/Cargo.toml
[package]
name = "nexus-provider"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
nexus-config = { path = "../config" }
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
```

```toml
# rust/crates/tools/Cargo.toml
[package]
name = "nexus-tools"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
```

```toml
# rust/crates/agent-core/Cargo.toml
[package]
name = "nexus-agent-core"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
nexus-config = { path = "../config" }
nexus-provider = { path = "../provider" }
nexus-tools = { path = "../tools" }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
```

```toml
# rust/crates/engine-cli/Cargo.toml
[package]
name = "nexus-engine-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "nexus-engine-cli"
path = "src/main.rs"

[dependencies]
anyhow = { workspace = true }
nexus-agent-core = { path = "../agent-core" }
nexus-config = { path = "../config" }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
```

- [ ] **Step 2: Add failing config test**

Create `rust/crates/config/src/lib.rs`:

```rust
use std::collections::BTreeMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_target_local_cache_gateway() {
        let settings = EngineSettings::default();

        assert_eq!(settings.cache_base_url, "http://127.0.0.1:8787/v1");
        assert_eq!(settings.cache_mode, CacheMode::Private);
        assert_eq!(settings.provider, "anthropic");
        assert_eq!(settings.model_alias, "claude-sonnet");
    }

    #[test]
    fn builds_cache_headers() {
        let settings = EngineSettings {
            cache_base_url: "https://cache.example.com/v1".to_string(),
            gateway_api_key: "gateway-key".to_string(),
            provider: "anthropic".to_string(),
            model_alias: "claude-sonnet".to_string(),
            cache_mode: CacheMode::Private,
            workflow: "coding-agent".to_string(),
        };

        let headers = settings.cache_headers();
        assert_eq!(headers.get("authorization").unwrap(), "Bearer gateway-key");
        assert_eq!(headers.get("x-llm-cache-mode").unwrap(), "private");
        assert_eq!(headers.get("x-llm-provider").unwrap(), "anthropic");
        assert_eq!(headers.get("x-llm-model-alias").unwrap(), "claude-sonnet");
        assert_eq!(headers.get("x-llm-workflow").unwrap(), "coding-agent");
    }
}
```

- [ ] **Step 3: Run config test to verify it fails**

Run: `cargo test -p nexus-config` from `rust/`.

Expected: fail because `EngineSettings` and `CacheMode` are not defined.

- [ ] **Step 4: Implement config types**

Replace `rust/crates/config/src/lib.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheMode {
    Bypass,
    Private,
    Public,
}

impl CacheMode {
    pub fn as_header_value(self) -> &'static str {
        match self {
            CacheMode::Bypass => "bypass",
            CacheMode::Private => "private",
            CacheMode::Public => "public",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineSettings {
    pub cache_base_url: String,
    pub gateway_api_key: String,
    pub provider: String,
    pub model_alias: String,
    pub cache_mode: CacheMode,
    pub workflow: String,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            cache_base_url: "http://127.0.0.1:8787/v1".to_string(),
            gateway_api_key: "demo-key-change-me".to_string(),
            provider: "anthropic".to_string(),
            model_alias: "claude-sonnet".to_string(),
            cache_mode: CacheMode::Private,
            workflow: "coding-agent".to_string(),
        }
    }
}

impl EngineSettings {
    pub fn cache_headers(&self) -> BTreeMap<String, String> {
        let mut headers = BTreeMap::from([
            ("x-llm-cache-mode".to_string(), self.cache_mode.as_header_value().to_string()),
            ("x-llm-provider".to_string(), self.provider.clone()),
            ("x-llm-model-alias".to_string(), self.model_alias.clone()),
            ("x-llm-workflow".to_string(), self.workflow.clone()),
        ]);

        if !self.gateway_api_key.is_empty() {
            headers.insert("authorization".to_string(), format!("Bearer {}", self.gateway_api_key));
        }

        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_target_local_cache_gateway() {
        let settings = EngineSettings::default();

        assert_eq!(settings.cache_base_url, "http://127.0.0.1:8787/v1");
        assert_eq!(settings.cache_mode, CacheMode::Private);
        assert_eq!(settings.provider, "anthropic");
        assert_eq!(settings.model_alias, "claude-sonnet");
    }

    #[test]
    fn builds_cache_headers() {
        let settings = EngineSettings {
            cache_base_url: "https://cache.example.com/v1".to_string(),
            gateway_api_key: "gateway-key".to_string(),
            provider: "anthropic".to_string(),
            model_alias: "claude-sonnet".to_string(),
            cache_mode: CacheMode::Private,
            workflow: "coding-agent".to_string(),
        };

        let headers = settings.cache_headers();
        assert_eq!(headers.get("authorization").unwrap(), "Bearer gateway-key");
        assert_eq!(headers.get("x-llm-cache-mode").unwrap(), "private");
        assert_eq!(headers.get("x-llm-provider").unwrap(), "anthropic");
        assert_eq!(headers.get("x-llm-model-alias").unwrap(), "claude-sonnet");
        assert_eq!(headers.get("x-llm-workflow").unwrap(), "coding-agent");
    }
}
```

- [ ] **Step 5: Add stub library files**

Create `rust/crates/provider/src/lib.rs`:

```rust
pub fn provider_crate_ready() -> bool {
    true
}
```

Create `rust/crates/tools/src/lib.rs`:

```rust
pub fn tools_crate_ready() -> bool {
    true
}
```

Create `rust/crates/agent-core/src/lib.rs`:

```rust
pub fn agent_core_ready() -> bool {
    true
}
```

Create `rust/crates/engine-cli/src/main.rs`:

```rust
fn main() {
    println!(r#"{"type":"ready"}"#);
}
```

- [ ] **Step 6: Run Rust tests**

Run: `cargo test` from `rust/`.

Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add rust
git commit -m "feat: scaffold Rust engine workspace"
```

## Task 4: Bridge Protocol Events

**Files:**
- Modify: `Sources/NexusCodeBridge/EngineEvents.swift`
- Create: `Tests/NexusCodeBridgeTests/EngineEventsTests.swift`
- Modify: `rust/crates/agent-core/src/lib.rs`

- [ ] **Step 1: Write Swift event decoding tests**

Create `Tests/NexusCodeBridgeTests/EngineEventsTests.swift`:

```swift
import XCTest
@testable import NexusCodeBridge

final class EngineEventsTests: XCTestCase {
    func testDecodesAssistantDelta() throws {
        let data = #"{"type":"assistant_delta","text":"Hello"}"#.data(using: .utf8)!
        let event = try JSONDecoder().decode(EngineEvent.self, from: data)

        XCTAssertEqual(event, .assistantDelta(text: "Hello"))
    }

    func testEncodesStartTaskCommand() throws {
        let command = EngineCommand.startTask(projectPath: "/tmp/project", prompt: "Fix tests")
        let data = try JSONEncoder().encode(command)
        let object = try JSONSerialization.jsonObject(with: data) as! [String: Any]

        XCTAssertEqual(object["type"] as? String, "start_task")
        XCTAssertEqual(object["project_path"] as? String, "/tmp/project")
        XCTAssertEqual(object["prompt"] as? String, "Fix tests")
    }
}
```

- [ ] **Step 2: Run Swift bridge tests to verify they fail**

Run: `swift test --filter EngineEventsTests`

Expected: fail because `EngineEvent` and `EngineCommand` do not exist.

- [ ] **Step 3: Implement Swift bridge event types**

Replace `Sources/NexusCodeBridge/EngineEvents.swift`:

```swift
import Combine
import Foundation

public enum EngineCommand: Equatable, Encodable {
    case startTask(projectPath: String, prompt: String)
    case approveTool(toolCallId: String, approved: Bool)
    case cancelTask

    enum CodingKeys: String, CodingKey {
        case type
        case projectPath = "project_path"
        case prompt
        case toolCallId = "tool_call_id"
        case approved
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case let .startTask(projectPath, prompt):
            try container.encode("start_task", forKey: .type)
            try container.encode(projectPath, forKey: .projectPath)
            try container.encode(prompt, forKey: .prompt)
        case let .approveTool(toolCallId, approved):
            try container.encode("approve_tool", forKey: .type)
            try container.encode(toolCallId, forKey: .toolCallId)
            try container.encode(approved, forKey: .approved)
        case .cancelTask:
            try container.encode("cancel_task", forKey: .type)
        }
    }
}

public enum EngineEvent: Equatable, Decodable {
    case ready
    case assistantDelta(text: String)
    case toolRequested(id: String, name: String, summary: String)
    case toolOutput(id: String, stream: String, text: String)
    case taskFinished(sessionId: String)
    case error(message: String)

    enum CodingKeys: String, CodingKey {
        case type
        case text
        case id
        case name
        case summary
        case stream
        case sessionId = "session_id"
        case message
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)

        switch type {
        case "ready":
            self = .ready
        case "assistant_delta":
            self = .assistantDelta(text: try container.decode(String.self, forKey: .text))
        case "tool_requested":
            self = .toolRequested(
                id: try container.decode(String.self, forKey: .id),
                name: try container.decode(String.self, forKey: .name),
                summary: try container.decode(String.self, forKey: .summary)
            )
        case "tool_output":
            self = .toolOutput(
                id: try container.decode(String.self, forKey: .id),
                stream: try container.decode(String.self, forKey: .stream),
                text: try container.decode(String.self, forKey: .text)
            )
        case "task_finished":
            self = .taskFinished(sessionId: try container.decode(String.self, forKey: .sessionId))
        case "error":
            self = .error(message: try container.decode(String.self, forKey: .message))
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type,
                in: container,
                debugDescription: "Unknown engine event type: \(type)"
            )
        }
    }
}
```

- [ ] **Step 4: Implement matching Rust event types**

Replace `rust/crates/agent-core/src/lib.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineCommand {
    StartTask { project_path: String, prompt: String },
    ApproveTool { tool_call_id: String, approved: bool },
    CancelTask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    Ready,
    AssistantDelta { text: String },
    ToolRequested { id: String, name: String, summary: String },
    ToolOutput { id: String, stream: String, text: String },
    TaskFinished { session_id: String },
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_assistant_delta() {
        let json = serde_json::to_string(&EngineEvent::AssistantDelta {
            text: "Hello".to_string(),
        })
        .unwrap();

        assert_eq!(json, r#"{"type":"assistant_delta","text":"Hello"}"#);
    }
}
```

- [ ] **Step 5: Run event tests**

Run: `swift test --filter EngineEventsTests`

Expected: pass.

Run: `cargo test -p nexus-agent-core` from `rust/`.

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add Sources/NexusCodeBridge Tests/NexusCodeBridgeTests rust/crates/agent-core
git commit -m "feat: define engine bridge protocol"
```

## Task 5: Engine CLI Mock Task Runner

**Files:**
- Modify: `rust/crates/engine-cli/src/main.rs`
- Modify: `rust/crates/agent-core/src/lib.rs`

- [ ] **Step 1: Add engine task runner test**

Add this test module to `rust/crates/agent-core/src/lib.rs`:

```rust
#[cfg(test)]
mod runner_tests {
    use super::*;

    #[tokio::test]
    async fn mock_runner_emits_expected_events() {
        let events = run_mock_task("/tmp/project", "Fix tests").await;

        assert_eq!(events[0], EngineEvent::AssistantDelta {
            text: "Inspecting /tmp/project for task: Fix tests".to_string(),
        });
        assert_eq!(events[1], EngineEvent::ToolRequested {
            id: "tool_list_dir".to_string(),
            name: "list_dir".to_string(),
            summary: "List project root".to_string(),
        });
        assert_eq!(events.last().unwrap(), &EngineEvent::TaskFinished {
            session_id: "mock-session".to_string(),
        });
    }
}
```

- [ ] **Step 2: Run Rust test to verify it fails**

Run: `cargo test -p nexus-agent-core runner_tests`

Expected: fail because `run_mock_task` does not exist.

- [ ] **Step 3: Implement mock runner**

Add this function above the test modules in `rust/crates/agent-core/src/lib.rs`:

```rust
pub async fn run_mock_task(project_path: &str, prompt: &str) -> Vec<EngineEvent> {
    vec![
        EngineEvent::AssistantDelta {
            text: format!("Inspecting {project_path} for task: {prompt}"),
        },
        EngineEvent::ToolRequested {
            id: "tool_list_dir".to_string(),
            name: "list_dir".to_string(),
            summary: "List project root".to_string(),
        },
        EngineEvent::ToolOutput {
            id: "tool_list_dir".to_string(),
            stream: "stdout".to_string(),
            text: "Mock tool output from project root".to_string(),
        },
        EngineEvent::TaskFinished {
            session_id: "mock-session".to_string(),
        },
    ]
}
```

- [ ] **Step 4: Implement stdin/stdout engine CLI**

Replace `rust/crates/engine-cli/src/main.rs`:

```rust
use anyhow::Result;
use nexus_agent_core::{run_mock_task, EngineCommand, EngineEvent};
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<()> {
    emit(&EngineEvent::Ready)?;

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let command: EngineCommand = serde_json::from_str(&line)?;
        match command {
            EngineCommand::StartTask { project_path, prompt } => {
                for event in run_mock_task(&project_path, &prompt).await {
                    emit(&event)?;
                }
            }
            EngineCommand::ApproveTool { .. } => {}
            EngineCommand::CancelTask => {
                emit(&EngineEvent::Error {
                    message: "Task cancelled".to_string(),
                })?;
            }
        }
    }

    Ok(())
}

fn emit(event: &EngineEvent) -> Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, event)?;
    writeln!(stdout)?;
    stdout.flush()?;
    Ok(())
}
```

- [ ] **Step 5: Run engine CLI manually**

Run from `rust/`:

```bash
printf '{"type":"start_task","project_path":"/tmp/project","prompt":"Fix tests"}\n' | cargo run -p nexus-engine-cli
```

Expected output includes `{"type":"ready"}` and `{"type":"task_finished","session_id":"mock-session"}`.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/agent-core rust/crates/engine-cli
git commit -m "feat: add mock engine task runner"
```

## Task 6: Swift EngineBridge Process Wrapper

**Files:**
- Create: `Sources/NexusCodeBridge/EngineBridge.swift`
- Modify: `Sources/NexusCodeUI/ContentView.swift`

- [ ] **Step 1: Implement bridge process wrapper**

Create `Sources/NexusCodeBridge/EngineBridge.swift`:

```swift
import Combine
import Foundation

public final class EngineBridge: ObservableObject {
    private let executableURL: URL
    private var process: Process?
    private var inputPipe: Pipe?
    private var outputPipe: Pipe?
    private let decoder = JSONDecoder()
    private let encoder = JSONEncoder()

    public var onEvent: ((EngineEvent) -> Void)?
    public var onErrorText: ((String) -> Void)?

    public init(executableURL: URL) {
        self.executableURL = executableURL
    }

    public func start() throws {
        let process = Process()
        let inputPipe = Pipe()
        let outputPipe = Pipe()
        let errorPipe = Pipe()

        process.executableURL = executableURL
        process.standardInput = inputPipe
        process.standardOutput = outputPipe
        process.standardError = errorPipe

        outputPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            self?.consume(data: handle.availableData)
        }

        errorPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            guard let text = String(data: handle.availableData, encoding: .utf8), !text.isEmpty else {
                return
            }
            self?.onErrorText?(text)
        }

        try process.run()

        self.process = process
        self.inputPipe = inputPipe
        self.outputPipe = outputPipe
    }

    public func send(_ command: EngineCommand) throws {
        guard let inputPipe else {
            throw EngineBridgeError.notStarted
        }

        let data = try encoder.encode(command)
        inputPipe.fileHandleForWriting.write(data)
        inputPipe.fileHandleForWriting.write(Data([0x0A]))
    }

    public func stop() {
        outputPipe?.fileHandleForReading.readabilityHandler = nil
        process?.terminate()
        process = nil
        inputPipe = nil
        outputPipe = nil
    }

    private func consume(data: Data) {
        guard !data.isEmpty, let text = String(data: data, encoding: .utf8) else {
            return
        }

        for line in text.split(separator: "\n") {
            guard let lineData = String(line).data(using: .utf8) else {
                continue
            }

            do {
                onEvent?(try decoder.decode(EngineEvent.self, from: lineData))
            } catch {
                onErrorText?("Could not decode engine event: \(error)")
            }
        }
    }
}

public enum EngineBridgeError: Error, Equatable {
    case notStarted
}
```

- [ ] **Step 2: Wire ContentView to accept mock log updates without launching the process yet**

Modify the `Button("Run")` action in `Sources/NexusCodeUI/ContentView.swift`:

```swift
Button("Run") {
    guard let selectedProject else { return }
    let task = prompt.trimmingCharacters(in: .whitespacesAndNewlines)
    log.append(RunLogEntry(kind: .status, text: "Project: \(selectedProject.path)"))
    log.append(RunLogEntry(kind: .assistant, text: "Queued task: \(task)"))
    prompt = ""
}
```

- [ ] **Step 3: Build Swift**

Run: `swift build`

Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add Sources/NexusCodeBridge Sources/NexusCodeUI
git commit -m "feat: add Swift engine bridge"
```

## Task 7: Basic Rust Tools

**Files:**
- Modify: `rust/crates/tools/src/lib.rs`

- [ ] **Step 1: Write tool tests**

Replace `rust/crates/tools/src/lib.rs`:

```rust
use anyhow::Result;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_dir_returns_names() {
        let temp = tempfile::tempdir().unwrap();
        tokio::fs::write(temp.path().join("README.md"), "hello").await.unwrap();

        let output = list_dir(temp.path()).await.unwrap();

        assert!(output.contains("README.md"));
    }

    #[tokio::test]
    async fn read_file_returns_contents() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("README.md");
        tokio::fs::write(&file, "hello").await.unwrap();

        let output = read_file(&file).await.unwrap();

        assert_eq!(output, "hello");
    }
}
```

- [ ] **Step 2: Run tools tests to verify they fail**

Run from `rust/`: `cargo test -p nexus-tools`

Expected: fail because `list_dir` and `read_file` are missing.

- [ ] **Step 3: Implement tools**

Replace `rust/crates/tools/src/lib.rs`:

```rust
use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

pub async fn list_dir(path: impl AsRef<Path>) -> Result<String> {
    let mut entries = tokio::fs::read_dir(path).await?;
    let mut names = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        names.push(entry.file_name().to_string_lossy().to_string());
    }

    names.sort();
    Ok(names.join("\n"))
}

pub async fn read_file(path: impl AsRef<Path>) -> Result<String> {
    Ok(tokio::fs::read_to_string(path).await?)
}

pub async fn run_shell(project_path: impl AsRef<Path>, command: &str) -> Result<String> {
    let output = Command::new("/bin/zsh")
        .arg("-lc")
        .arg(command)
        .current_dir(project_path)
        .output()
        .await?;

    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_dir_returns_names() {
        let temp = tempfile::tempdir().unwrap();
        tokio::fs::write(temp.path().join("README.md"), "hello").await.unwrap();

        let output = list_dir(temp.path()).await.unwrap();

        assert!(output.contains("README.md"));
    }

    #[tokio::test]
    async fn read_file_returns_contents() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("README.md");
        tokio::fs::write(&file, "hello").await.unwrap();

        let output = read_file(&file).await.unwrap();

        assert_eq!(output, "hello");
    }
}
```

- [ ] **Step 4: Add tempfile dev dependency**

Add to `rust/crates/tools/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 5: Run tests**

Run from `rust/`: `cargo test -p nexus-tools`

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/tools
git commit -m "feat: add basic engine tools"
```

## Task 8: Provider Request Construction for Cache Gateway

**Files:**
- Modify: `rust/crates/provider/src/lib.rs`

- [ ] **Step 1: Write provider request test**

Replace `rust/crates/provider/src/lib.rs`:

```rust
use nexus_config::EngineSettings;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_url_uses_cache_base_url() {
        let settings = EngineSettings::default();
        let request = ChatRequest::new(settings, "Fix tests");

        assert_eq!(request.url, "http://127.0.0.1:8787/v1/chat/completions");
        assert_eq!(request.headers.get("x-llm-cache-mode").unwrap(), "private");
    }
}
```

- [ ] **Step 2: Run provider tests to verify they fail**

Run from `rust/`: `cargo test -p nexus-provider`

Expected: fail because `ChatRequest` does not exist.

- [ ] **Step 3: Implement provider request model**

Replace `rust/crates/provider/src/lib.rs`:

```rust
use nexus_config::EngineSettings;
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequest {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: serde_json::Value,
}

impl ChatRequest {
    pub fn new(settings: EngineSettings, prompt: &str) -> Self {
        let base = settings.cache_base_url.trim_end_matches('/');
        Self {
            url: format!("{base}/chat/completions"),
            headers: settings.cache_headers(),
            body: json!({
                "model": settings.model_alias,
                "messages": [
                    { "role": "system", "content": "You are Nexus Code, a local-first macOS coding agent." },
                    { "role": "user", "content": prompt }
                ],
                "stream": true
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_url_uses_cache_base_url() {
        let settings = EngineSettings::default();
        let request = ChatRequest::new(settings, "Fix tests");

        assert_eq!(request.url, "http://127.0.0.1:8787/v1/chat/completions");
        assert_eq!(request.headers.get("x-llm-cache-mode").unwrap(), "private");
    }
}
```

- [ ] **Step 4: Run tests**

Run from `rust/`: `cargo test -p nexus-provider`

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/provider
git commit -m "feat: build cache gateway provider requests"
```

## Task 9: Session Persistence

**Files:**
- Modify: `rust/crates/agent-core/src/lib.rs`
- Modify: `rust/crates/agent-core/Cargo.toml`

- [ ] **Step 1: Add session save test**

Add to `rust/crates/agent-core/src/lib.rs`:

```rust
#[cfg(test)]
mod session_tests {
    use super::*;

    #[tokio::test]
    async fn saves_session_transcript() {
        let temp = tempfile::tempdir().unwrap();
        let session = SessionTranscript {
            session_id: "session-1".to_string(),
            project_path: "/tmp/project".to_string(),
            prompt: "Fix tests".to_string(),
            events: vec![EngineEvent::AssistantDelta { text: "Hello".to_string() }],
        };

        let path = save_session_to_dir(temp.path(), &session).await.unwrap();
        let saved = tokio::fs::read_to_string(path).await.unwrap();

        assert!(saved.contains("session-1"));
        assert!(saved.contains("Fix tests"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run from `rust/`: `cargo test -p nexus-agent-core session_tests`

Expected: fail because `SessionTranscript` and `save_session_to_dir` do not exist.

- [ ] **Step 3: Implement session persistence**

Add to `rust/crates/agent-core/src/lib.rs`:

```rust
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTranscript {
    pub session_id: String,
    pub project_path: String,
    pub prompt: String,
    pub events: Vec<EngineEvent>,
}

pub async fn save_session_to_dir(dir: impl AsRef<Path>, session: &SessionTranscript) -> anyhow::Result<PathBuf> {
    tokio::fs::create_dir_all(dir.as_ref()).await?;
    let path = dir.as_ref().join(format!("{}.json", session.session_id));
    let json = serde_json::to_string_pretty(session)?;
    tokio::fs::write(&path, json).await?;
    Ok(path)
}
```

- [ ] **Step 4: Add tempfile dev dependency**

Add to `rust/crates/agent-core/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 5: Run tests**

Run from `rust/`: `cargo test -p nexus-agent-core`

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/agent-core
git commit -m "feat: save engine sessions"
```

## Task 10: App Wiring Smoke Milestone

**Files:**
- Modify: `Sources/NexusCodeUI/ContentView.swift`
- Modify: `Sources/NexusCodeUI/ProjectPickerView.swift`
- Modify: `.gitignore`

- [ ] **Step 1: Update project picker to use macOS folder panel**

Replace `Sources/NexusCodeUI/ProjectPickerView.swift`:

```swift
import SwiftUI
import AppKit

public struct ProjectPickerView: View {
    @Binding var selectedProject: ProjectItem?

    public init(selectedProject: Binding<ProjectItem?>) {
        self._selectedProject = selectedProject
    }

    public var body: some View {
        Button {
            let panel = NSOpenPanel()
            panel.canChooseFiles = false
            panel.canChooseDirectories = true
            panel.allowsMultipleSelection = false

            if panel.runModal() == .OK, let url = panel.url {
                selectedProject = ProjectItem(name: url.lastPathComponent, path: url.path)
            }
        } label: {
            Label(selectedProject?.name ?? "Choose Project", systemImage: "folder")
        }
    }
}
```

- [ ] **Step 2: Add development .gitignore**

Create or update `.gitignore`:

```gitignore
.build/
rust/target/
**/.DS_Store
.nexus/sessions/
```

- [ ] **Step 3: Run full verification**

Run:

```bash
swift build
swift test
(cd rust && cargo test)
```

Expected: all pass.

- [ ] **Step 4: Run app executable**

Run:

```bash
swift run NexusCode
```

Expected: the app opens, lets you pick a project folder, type a task, and append a queued task to the run log.

- [ ] **Step 5: Commit**

```bash
git add Sources .gitignore
git commit -m "feat: wire first macOS smoke milestone"
```

## Self-Review Notes

- Spec coverage: this plan covers the SwiftUI app shell, cache gateway settings, Rust engine workspace, JSON bridge, mock engine runner, basic tools, provider cache request construction, session persistence, project picker, and smoke verification.
- Deferred by design: real OpenAI streaming, full ReAct tool-call parsing, MCP tools, packaging/signing, and dedicated server deployment. These are beyond the first milestone and remain compatible with this structure.
- Red-flag scan: no unresolved future-fill steps or missing implementation steps remain.
