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
