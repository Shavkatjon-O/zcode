//! Terminal UI with colors for phases, tools, errors, and output.

use colored::Colorize;
use std::future::Future;

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

/// Show progress while reading a file for context.
pub fn reading_file(path: &str) {
    println!("{}", format!("  ⟳ Reading {} …", path).dimmed());
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

/// Mark a file as read (optional; use after reading_file when you want a checkmark).
pub fn reading_file_done(path: &str) {
    println!("{}", format!("  ✓ {} ", path).green());
}

pub fn tool_call(name: &str) {
    println!("{}", format!("  → {} ", name).yellow());
}

/// Show tool call with optional arguments preview (e.g. "run_command" with "cargo build").
pub fn tool_call_with_args(name: &str, args_preview: Option<&str>) {
    if let Some(preview) = args_preview {
        let short = if preview.len() > 60 {
            format!("{}…", &preview[..60])
        } else {
            preview.to_string()
        };
        println!(
            "{}",
            format!("  → {} {}", name, short).yellow()
        );
    } else {
        tool_call(name);
    }
}

pub fn tool_running() {
    print!("{}", "    … ".dimmed());
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

pub fn tool_result(s: &str) {
    // Keep result muted so assistant output stands out
    let preview = if s.len() > 200 {
        format!("{}…", &s[..200])
    } else {
        s.to_string()
    };
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

/// Show "Thinking..." until the first streamed chunk or tool call (call before chat_stream).
pub fn thinking() {
    print!("{}", "  … ".dimmed());
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

/// Clear the "Thinking..." line so streamed output starts clean (e.g. print \r and spaces, then newline).
pub fn clear_thinking() {
    print!("\r    \r");
    let _ = std::io::Write::flush(&mut std::io::stdout());
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

/// Run a future while showing an animated spinner and message. When the future completes,
/// the spinner is replaced with a checkmark and the result is returned.
pub async fn with_spinner<F, T>(msg: &str, future: F) -> T
where
    F: Future<Output = T>,
{
    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
    let msg_for_spinner = msg.to_string();
    let spinner_handle = tokio::spawn(async move {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut i = 0usize;
        loop {
            tokio::select! {
                _ = &mut rx => break,
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(80)) => {
                    print!("\r  {} {} ", frames[i], msg_for_spinner);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    i = (i + 1) % frames.len();
                }
            }
        }
    });
    let result = future.await;
    let _ = tx.send(());
    let _ = spinner_handle.await;
    print!("\r  ✓ {} \n", msg);
    let _ = std::io::Write::flush(&mut std::io::stdout());
    result
}
