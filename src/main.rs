use zcode::{cli::Cli, config, tools::Executor};
use clap::Parser;
use std::env;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let api_key = config::load_api_key().unwrap_or_else(|| {
        eprintln!(
            "Set OPENAI_API_KEY env var or add api_key in ~/.config/zcode/config.toml"
        );
        std::process::exit(1);
    });

    let workspace = env::current_dir().expect("current dir");
    let executor = Executor::new(workspace);

    if let Some(prompt) = cli.prompt {
        zcode::run::run_once(&api_key, &executor, &prompt).await;
    } else {
        zcode::run::run_repl(&api_key, &executor).await;
    }
}
