use clap::Parser;

#[derive(Parser)]
#[command(name = "zcode")]
#[command(about = "CLI coding agent powered by LLMs")]
pub struct Cli {
    #[arg(short, long)]
    pub prompt: Option<String>,
}
