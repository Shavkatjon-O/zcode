use clap::Parser;
use std::str::FromStr;

use crate::agent::AgentProvider;

#[derive(Parser)]
#[command(name = "zcode")]
#[command(about = "CLI coding agent powered by LLMs")]
pub struct Cli {
    #[arg(short, long)]
    pub prompt: Option<String>,

    /// LLM provider: openai or gemini (default: openai, or ZCODE_PROVIDER env / config)
    #[arg(long, value_parser = parse_provider)]
    pub provider: Option<AgentProvider>,
}

fn parse_provider(s: &str) -> Result<AgentProvider, String> {
    AgentProvider::from_str(s)
}
