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

    if let Some(m) = cli.model  { cfg.provider.model = m; }
    if cli.yolo { cfg.agent.approval_mode = ApprovalMode::Yolo; }
    if cli.safe { cfg.agent.approval_mode = ApprovalMode::Ask; }

    match cli.command {
        Some(Sub::Init)     => return commands::cmd_init(&cfg.provider.api_key, &cfg.provider.base_url).await,
        Some(Sub::Models)   => return commands::cmd_models(&cfg.provider.api_key, &cfg.provider.base_url).await,
        Some(Sub::Config)   => return commands::cmd_config(),
        Some(Sub::Sessions) => return commands::cmd_sessions(),
        None => {}
    }

    if cfg.provider.api_key.is_empty() {
        eprintln!("No API key set. Run `nxc init` to configure.");
        std::process::exit(1);
    }

    let fetcher = ModelFetcher::new(&cfg.provider.base_url, &cfg.provider.api_key);
    let pricing = build_cache(fetcher.fetch().await.unwrap_or_default());
    let model   = cfg.provider.model.clone();

    let tools = all_tools();

    let agents_md = std::fs::read_to_string(".nxc/AGENTS.md").ok();

    let mut agent = Agent::new(cfg.clone(), tools);
    if let Some(instructions) = &agents_md {
        agent.history.push(nxc_provider::Message::system(instructions));
    }

    if cli.resume {
        if let Some(msgs) = session::load_latest_session()? {
            agent.history.messages = msgs;
            println!("Resumed previous session ({} messages).", agent.history.messages.len());
        }
    }

    if let Some(prompt_text) = cli.prompt {
        agent.history.push(nxc_provider::Message::user(&prompt_text));
        run_turns(&mut agent, &pricing, &model, &cfg).await?;
        session::save_session(&agent.history.messages)?;
        return Ok(());
    }

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
    for _ in 0..cfg.agent.max_turns {
        let keep_going = agent.run_turn(TurnCallbacks {
            on_text:   &|t| output::print_delta(t),
            on_action: &|name, args| output::print_tool_action(name, args),
        }).await?;
        if !keep_going { break; }
    }

    if let Some(info) = pricing.get(model) {
        let cost = info.estimate_cost(0, 0);
        output::print_status(0, 0, cost, model);
    }
    Ok(())
}
