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
                let ext  = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("png");
                let mime = match ext { "jpg" | "jpeg" => "image/jpeg", "gif" => "image/gif", "webp" => "image/webp", _ => "image/png" };
                Ok(ToolResult::ok(format!("data:{mime};base64,{}", STANDARD.encode(&bytes))))
            }
        }
    }
}

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
