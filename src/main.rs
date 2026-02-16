use zcode::{
    agent::{AgentProvider, GeminiAgent, Message, OpenAiAgent},
    cli::Cli,
    config,
    tools::Executor,
};
use clap::Parser;
use std::env;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let provider = cli.provider;

    let api_key = config::load_api_key(provider).unwrap_or_else(|| {
        let (env_var, config_hint) = match provider {
            AgentProvider::OpenAi => {
                ("OPENAI_API_KEY", "api_key in ~/.config/zcode/config.toml")
            }
            AgentProvider::Gemini => {
                ("GEMINI_API_KEY", "gemini_api_key in ~/.config/zcode/config.toml")
            }
        };
        eprintln!(
            "Set {} env var or add {} for provider {:?}",
            env_var, config_hint, provider
        );
        std::process::exit(1);
    });

    let workspace = env::current_dir().expect("current dir");
    let executor = Executor::new(workspace);

    match provider {
        AgentProvider::OpenAi => {
            let agent = OpenAiAgent::new(api_key);
            run_with_agent(&agent, &executor, cli).await;
        }
        AgentProvider::Gemini => {
            let agent = GeminiAgent::new(api_key);
            run_with_agent(&agent, &executor, cli).await;
        }
    }
}

async fn run_with_agent<A: zcode::agent::Agent>(
    agent: &A,
    executor: &Executor,
    cli: Cli,
) {
    let mut messages = Vec::new();

    if let Some(prompt) = cli.prompt {
        run_agent(agent, executor, &mut messages, &prompt).await;
    } else {
        loop {
            if let Some(prompt) = read_prompt() {
                run_agent(agent, executor, &mut messages, &prompt).await;
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

async fn run_agent<A: zcode::agent::Agent>(
    agent: &A,
    executor: &Executor,
    messages: &mut Vec<Message>,
    user_input: &str,
) {
    let mut next_input = Some(user_input);

    loop {
        let mut on_chunk = |chunk: &str| {
            print!("{}", chunk);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        };

        let resp = match agent.chat_stream(messages, next_input.take(), &mut on_chunk).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error: {}", e);
                return;
            }
        };

        if let Some(tool_calls) = resp.tool_calls {
            println!(); // newline after any streamed content
            for tc in &tool_calls {
                print!("[{}] ", tc.function.name);
                let _ = std::io::Write::flush(&mut std::io::stdout());
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
                messages.push(Message::ToolResult {
                    role: "tool".into(),
                    tool_call_id: tc.id.clone(),
                    function_name: tc.function.name.clone(),
                    content: result,
                });
            }
            next_input = None;
            continue;
        }

        if resp.content.is_some() && !resp.content.as_ref().map_or(true, |s| s.is_empty()) {
            println!(); // newline after streamed content
        }
        break;
    }
}
