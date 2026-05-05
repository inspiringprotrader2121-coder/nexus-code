use anyhow::Result;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

pub struct WizardAnswers {
    pub api_key:       String,
    pub model:         String,
    pub approval_mode: String,
    pub ask_for_bash:  bool,
    pub ask_for_git:   bool,
}

/// Run the interactive setup wizard. `models` is a list of available model IDs
/// fetched from OpenRouter (may be empty if offline).
pub fn run_wizard(models: &[String]) -> Result<WizardAnswers> {
    println!("\n✦ Welcome to Nexus Code!\n");

    let api_key = prompt_secret("OpenRouter API key")?;

    let display: Vec<String> = if models.is_empty() {
        vec![
            "anthropic/claude-sonnet-4-6".to_string(),
            "anthropic/claude-haiku-4-5-20251001".to_string(),
            "openai/gpt-4o".to_string(),
        ]
    } else {
        models.iter().take(10).cloned().collect()
    };

    println!("\nAvailable models:");
    for (i, m) in display.iter().enumerate() {
        println!("  {}. {m}", i + 1);
    }

    let model_input = prompt("Model (number or full id) [1]")?;
    let model = match model_input.trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= display.len() => display[n - 1].clone(),
        _ if model_input.trim().is_empty() => display[0].clone(),
        _ => model_input.trim().to_string(),
    };

    let approval_raw = prompt("Default approval mode (auto/ask/yolo) [auto]")?;
    let approval_mode = if approval_raw.trim().is_empty() {
        "auto".to_string()
    } else {
        approval_raw.trim().to_string()
    };

    let bash_raw = prompt("Always ask before running bash commands? (y/N)")?;
    let ask_for_bash = bash_raw.trim().eq_ignore_ascii_case("y");

    let git_raw = prompt("Always ask before git commits? (y/N)")?;
    let ask_for_git = git_raw.trim().eq_ignore_ascii_case("y");

    Ok(WizardAnswers { api_key, model, approval_mode, ask_for_bash, ask_for_git })
}

/// Convert wizard answers to a TOML config string.
pub fn answers_to_toml(a: &WizardAnswers) -> String {
    let mut tool_approvals = String::new();
    if a.ask_for_bash {
        tool_approvals.push_str("bash = \"ask\"\n");
    }
    if a.ask_for_git {
        tool_approvals.push_str("git_commit = \"ask\"\n");
    }

    format!(
        "[provider]\napi_key  = \"{}\"\nmodel    = \"{}\"\nbase_url = \"https://openrouter.ai/api/v1\"\n\n[agent]\napproval_mode = \"{}\"\nmax_turns     = 50\ncontext_limit = 128000\n\n[tools.approval]\n{}",
        a.api_key, a.model, a.approval_mode, tool_approvals
    )
}

/// Write the TOML string to the given path, creating parent directories.
pub fn write_wizard_config(path: &Path, toml: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
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

fn prompt_secret(label: &str) -> Result<String> {
    print!("? {label}: ");
    io::stdout().flush()?;
    // Best effort: stdin may not be a tty in tests/CI, fall back to plain readline
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

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
        assert!(toml.contains("git_commit"));
    }

    #[test]
    fn write_wizard_config_creates_file() {
        let dir  = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        write_wizard_config(&path, "[provider]\napi_key = \"x\"").unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("api_key"));
    }
}
