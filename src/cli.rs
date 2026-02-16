use clap::Parser;
use std::str::FromStr;

use crate::agent::AgentProvider;

#[derive(Parser)]
#[command(name = "zcode")]
#[command(about = "CLI coding agent powered by LLMs")]
pub struct Cli {
    #[arg(short, long)]
    pub prompt: Option<String>,

    /// LLM provider: openai or gemini
    #[arg(long, value_parser = parse_provider, default_value = "openai")]
    pub provider: AgentProvider,
}

fn parse_provider(s: &str) -> Result<AgentProvider, String> {
    AgentProvider::from_str(s)
}
