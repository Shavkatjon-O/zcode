//! Terminal UI with colors for phases, tools, errors, and output.

use colored::Colorize;

pub fn phase(label: &str) {
    println!("{}", format!("▸ {} ", label).bright_cyan().bold());
}

pub fn phase_done(label: &str) {
    println!("{}", format!("  ✓ {} ", label).green());
}

pub fn step(index: usize, total: usize, text: &str) {
    println!(
        "  {}",
        format!("[{}/{}] {}", index, total, text).bright_white()
    );
}

pub fn tool_call(name: &str) {
    println!("{}", format!("  → {} ", name).yellow());
}

pub fn tool_result(s: &str) {
    // Keep result muted so assistant output stands out
    let preview = if s.len() > 200 { format!("{}…", &s[..200]) } else { s.to_string() };
    println!("{}", format!("    {}", preview).dimmed());
}

pub fn tool_error(e: &str) {
    eprintln!("{}", format!("    ✗ {}", e).red());
}

pub fn assistant_chunk(chunk: &str) {
    print!("{}", chunk.bright_white());
}

pub fn assistant_line() {
    println!();
}

pub fn error_msg(e: &str) {
    eprintln!("{}", format!("Error: {}", e).red().bold());
}

pub fn prompt_line() {
    print!("{}", "> ".bright_green().bold());
}

pub fn welcome() {
    println!(
        "{}",
        "zcode — multi-step coding agent (OpenAI). Type a prompt or Ctrl-D to exit."
            .bright_black()
    );
    println!();
}
