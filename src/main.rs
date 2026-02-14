use zcode::{agent::ChatGptAgent, cli::Cli, config, tools::Executor};
use clap::Parser;
use std::env;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let api_key = config::load_api_key().unwrap_or_else(|| {
        eprintln!("Set OPENAI_API_KEY env var or add api_key to ~/.config/zcode/config.toml");
        std::process::exit(1);
    });

    let workspace = env::current_dir().expect("current dir");
    let agent = ChatGptAgent::new(api_key);
    let executor = Executor::new(workspace);

    if let Some(prompt) = cli.prompt {
        run_agent(&agent, &executor, &mut Vec::new(), &prompt).await;
    } else {
        let mut messages = Vec::new();
        loop {
            if let Some(prompt) = read_prompt() {
                run_agent(&agent, &executor, &mut messages, &prompt).await;
            } else {
                break;
            }
        }
    }
}

fn read_prompt() -> Option<String> {
    print!("> ");
    std::io::Write::flush(&mut std::io::stdout()).ok()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok()?;
    let s = line.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

async fn run_agent(
    agent: &ChatGptAgent,
    executor: &Executor,
    messages: &mut Vec<zcode::agent::Message>,
    user_input: &str,
) {
    let mut next_input = Some(user_input);

    loop {
        let resp = match agent.chat(messages, next_input.take()).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error: {}", e);
                return;
            }
        };

        if let Some(tool_calls) = resp.tool_calls {
            for tc in &tool_calls {
                print!("[{}] ", tc.function.name);
                let result = match executor.execute(tc) {
                    Ok(r) => {
                        println!("-> {}", r);
                        r
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        format!("Error: {}", e)
                    }
                };
                messages.push(zcode::agent::Message::ToolResult {
                    role: "tool".into(),
                    tool_call_id: tc.id.clone(),
                    content: result,
                });
            }
            next_input = None;
            continue;
        }

        if let Some(content) = resp.content {
            if !content.is_empty() {
                println!("{}", content.trim());
            }
        }
        break;
    }
}
