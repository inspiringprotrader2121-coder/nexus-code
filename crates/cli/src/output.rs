use crossterm::style::{Color, ResetColor, SetForegroundColor};
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

#[allow(dead_code)]
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
