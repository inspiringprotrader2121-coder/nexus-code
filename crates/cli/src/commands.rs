use anyhow::Result;
use nxc_config::{global_config_path, run_wizard, answers_to_toml, write_wizard_config};
use nxc_provider::models::ModelFetcher;

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
