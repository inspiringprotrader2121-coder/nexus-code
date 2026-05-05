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
        json!({"type":"object","properties":{"path":{"type":"string","description":"Absolute or relative file path"}},"required":["path"]})
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
    fn description(&self) -> &'static str { "Write or overwrite a file with the given content" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let path    = args["path"].as_str().unwrap_or("");
        let content = args["content"].as_str().unwrap_or("");
        if let Some(parent) = Path::new(path).parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Ok(ToolResult::err(format!("write_file mkdir error: {e}")));
            }
        }
        match fs::write(path, content) {
            Ok(_)  => Ok(ToolResult::ok(format!("wrote {path} ({} bytes)", content.len()))),
            Err(e) => Ok(ToolResult::err(format!("write_file error: {e}"))),
        }
    }
}

#[async_trait]
impl Tool for ApplyPatch {
    fn name(&self)        -> &'static str { "apply_patch" }
    fn description(&self) -> &'static str { "Apply a unified diff patch string to a file" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"patch":{"type":"string","description":"Unified diff patch"}},"required":["path","patch"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let path  = args["path"].as_str().unwrap_or("");
        let patch = args["patch"].as_str().unwrap_or("");
        let original = match fs::read_to_string(path) {
            Ok(s)  => s,
            Err(e) => return Ok(ToolResult::err(format!("apply_patch: cannot read {path}: {e}"))),
        };
        let trailing_newline = original.ends_with('\n');
        let is_diff = patch.lines().any(|l| l.starts_with("---") || l.starts_with("+++"));
        let mut result = if is_diff {
            let orig_lines: Vec<&str> = original.lines().collect();
            let mut out_lines: Vec<String> = Vec::new();
            let mut orig_idx: usize = 0;
            let mut in_hunk = false;
            let mut hunk_orig_start: usize = 0;

            for line in patch.lines() {
                if line.starts_with("---") || line.starts_with("+++") || line.starts_with('\\') {
                    continue;
                }
                if line.starts_with("@@") {
                    let nums: Vec<&str> = line.split_whitespace().collect();
                    if nums.len() >= 3 {
                        let orig_range = nums[1].trim_start_matches('-');
                        let orig_start_str = orig_range.split(',').next().unwrap_or("1");
                        hunk_orig_start = orig_start_str.parse::<usize>().unwrap_or(1).saturating_sub(1);
                    }
                    while orig_idx < hunk_orig_start {
                        if orig_idx < orig_lines.len() {
                            out_lines.push(orig_lines[orig_idx].to_string());
                        }
                        orig_idx += 1;
                    }
                    in_hunk = true;
                    continue;
                }
                if in_hunk {
                    if let Some(added) = line.strip_prefix('+') {
                        out_lines.push(added.to_string());
                    } else if line.starts_with('-') {
                        orig_idx += 1;
                    } else {
                        let ctx = line.strip_prefix(' ').unwrap_or(line);
                        out_lines.push(ctx.to_string());
                        orig_idx += 1;
                    }
                }
            }
            while orig_idx < orig_lines.len() {
                out_lines.push(orig_lines[orig_idx].to_string());
                orig_idx += 1;
            }
            out_lines.join("\n")
        } else {
            patch.to_string()
        };
        if trailing_newline && !result.ends_with('\n') {
            result.push('\n');
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
        json!({"type":"object","properties":{"path":{"type":"string","description":"Directory path (default: .)"}},"required":["path"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or(".");
        match fs::read_dir(path) {
            Err(e) => Ok(ToolResult::err(format!("list_dir error: {e}"))),
            Ok(entries) => {
                let mut lines: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            format!("{name}/")
                        } else {
                            name
                        }
                    })
                    .collect();
                lines.sort();
                Ok(ToolResult::ok(lines.join("\n")))
            }
        }
    }
}

#[async_trait]
impl Tool for DeleteFile {
    fn name(&self)        -> &'static str { "delete_file" }
    fn description(&self) -> &'static str { "Delete a file at the given path" }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn read_file_returns_contents() {
        let dir  = tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, "hello world").unwrap();
        let res = ReadFile.execute(serde_json::json!({"path": path.to_str().unwrap()})).await.unwrap();
        assert!(!res.is_error);
        assert_eq!(res.content, "hello world");
    }

    #[tokio::test]
    async fn write_file_creates_file() {
        let dir  = tempdir().unwrap();
        let path = dir.path().join("out.txt");
        let res  = WriteFile.execute(serde_json::json!({"path": path.to_str().unwrap(), "content": "written"})).await.unwrap();
        assert!(!res.is_error);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "written");
    }

    #[tokio::test]
    async fn list_dir_returns_entries() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "").unwrap();
        std::fs::write(dir.path().join("b.rs"), "").unwrap();
        let res = ListDir.execute(serde_json::json!({"path": dir.path().to_str().unwrap()})).await.unwrap();
        assert!(res.content.contains("a.rs"));
        assert!(res.content.contains("b.rs"));
    }

    #[tokio::test]
    async fn delete_file_removes_file() {
        let dir  = tempdir().unwrap();
        let path = dir.path().join("del.txt");
        std::fs::write(&path, "bye").unwrap();
        let res = DeleteFile.execute(serde_json::json!({"path": path.to_str().unwrap()})).await.unwrap();
        assert!(!res.is_error);
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn read_file_missing_returns_error() {
        let res = ReadFile.execute(serde_json::json!({"path": "/nonexistent/file.txt"})).await.unwrap();
        assert!(res.is_error);
    }

    #[tokio::test]
    async fn apply_patch_replaces_lines() {
        let dir  = tempdir().unwrap();
        let path = dir.path().join("src.txt");
        std::fs::write(&path, "line1\nline2\nline3\n").unwrap();
        let patch = "--- src.txt\n+++ src.txt\n@@ -1,3 +1,3 @@\n line1\n-line2\n+LINE2\n line3\n";
        let res = ApplyPatch.execute(serde_json::json!({"path": path.to_str().unwrap(), "patch": patch})).await.unwrap();
        assert!(!res.is_error);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "line1\nLINE2\nline3\n");
    }

    #[tokio::test]
    async fn apply_patch_missing_file_returns_error() {
        let res = ApplyPatch.execute(serde_json::json!({"path": "/no/such/file.txt", "patch": "--- a\n+++ b\n"})).await.unwrap();
        assert!(res.is_error);
    }
}
