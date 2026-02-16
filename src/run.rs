//! Multi-step reasoning pipeline: plan → gather context → execute todos → final check.

use crate::agent::{Message, OpenAiAgent, ToolCall};
use crate::tools::Executor;
use crate::ui;
use serde::Deserialize;

const PLANNER_MODEL: &str = "gpt-4o-mini";
const EXECUTOR_MODEL: &str = "gpt-4o";

const PLANNER_SYSTEM: &str = r#"You are a coding task planner. Given a user request and the project root directory listing, output a JSON object (and nothing else) with:
- "summary": one-line summary of the task
- "paths_to_read": array of file/dir paths to read for context (e.g. ["src/main.rs", "Cargo.toml"]). Use at most 8 paths. Omit if not needed.
- "todos": array of 1–8 concrete step descriptions to complete the task (e.g. "Add a config module", "Update main to use config")

Output only valid JSON, no markdown or explanation."#;

const FINAL_CHECK_SYSTEM: &str = "You are a coding assistant. In one short sentence, say whether the task is complete or what the user might want to do next. No code.";

/// Plan from the planner model (JSON).
#[derive(Debug, Deserialize)]
struct Plan {
    summary: Option<String>,
    paths_to_read: Option<Vec<String>>,
    todos: Option<Vec<String>>,
}

fn extract_json(text: &str) -> Option<&str> {
    let text = text.trim();
    if let Some(s) = text.strip_prefix("```json") {
        return s.trim_end().strip_suffix("```").map(|s| s.trim());
    }
    if let Some(s) = text.strip_prefix("```") {
        return s.trim_end().strip_suffix("```").map(|s| s.trim());
    }
    Some(text)
}

fn list_dir_call(path: &str) -> ToolCall {
    ToolCall {
        id: "ctx_list".into(),
        type_: "function".into(),
        function: crate::agent::FunctionCall {
            name: "list_dir".into(),
            arguments: format!(r#"{{"path":"{}"}}"#, path),
        },
    }
}

fn read_file_call(path: &str) -> ToolCall {
    ToolCall {
        id: "ctx_read".into(),
        type_: "function".into(),
        function: crate::agent::FunctionCall {
            name: "read_file".into(),
            arguments: format!(r#"{{"path":"{}"}}"#, path),
        },
    }
}

pub async fn run_once(api_key: &str, executor: &Executor, user_prompt: &str) {
    let planner = OpenAiAgent::new(api_key.to_string()).with_model(PLANNER_MODEL);
    let exec_agent = OpenAiAgent::new(api_key.to_string()).with_model(EXECUTOR_MODEL);

    // --- Phase 1: Gather root listing for planner ---
    ui::phase("Gathering project layout");
    let root_listing = executor
        .execute(&list_dir_call("."))
        .unwrap_or_else(|e| format!("(list_dir failed: {})", e));
    ui::phase_done("Project layout");

    // --- Phase 2: Plan (cheap model) ---
    ui::phase("Planning");
    let plan_user = format!(
        "User request:\n{}\n\nRoot directory listing:\n{}",
        user_prompt, root_listing
    );
    let plan_text = match planner.completion(PLANNER_SYSTEM, &plan_user).await {
        Ok(t) => t,
        Err(e) => {
            ui::error_msg(&e);
            return;
        }
    };
    let plan_json = extract_json(&plan_text).unwrap_or(&plan_text);
    let plan: Plan = match serde_json::from_str(plan_json) {
        Ok(p) => p,
        Err(e) => {
            ui::error_msg(&format!("Failed to parse plan: {}. Raw: {}", e, plan_text));
            return;
        }
    };
    let todos = plan.todos.unwrap_or_else(|| vec!["Complete the user request.".into()]);
    let summary = plan.summary.as_deref().unwrap_or("Task");
    ui::phase_done("Planning");
    for (i, t) in todos.iter().enumerate() {
        ui::step(i + 1, todos.len(), t);
    }

    // --- Phase 3: Gather context (read paths_from_plan) ---
    let paths_to_read = plan.paths_to_read.unwrap_or_default();
    let mut context_parts = vec![format!("Root listing:\n{}", root_listing)];
    for path in paths_to_read.iter().take(8) {
        if let Ok(content) = executor.execute(&read_file_call(path)) {
            context_parts.push(format!("--- {} ---\n{}", path, content));
        }
    }
    let context_block = context_parts.join("\n\n");

    // --- Phase 4: Execute with strong model (tools + stream) ---
    ui::phase("Executing");
    let initial_user = format!(
        "Context:\n{}\n\nTask: {}\n\nUser request: {}",
        context_block, summary, user_prompt
    );
    let mut messages: Vec<Message> = vec![Message::Role {
        role: "user".into(),
        content: initial_user,
    }];

    loop {
        let mut on_chunk = |chunk: &str| {
            ui::assistant_chunk(chunk);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        };

        let resp = match exec_agent
            .chat_stream(&mut messages, None, &mut on_chunk)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                ui::assistant_line();
                ui::error_msg(&e);
                break;
            }
        };

        if let Some(tool_calls) = resp.tool_calls {
            ui::assistant_line();
            for tc in &tool_calls {
                ui::tool_call(&tc.function.name);
                let result = match executor.execute(tc) {
                    Ok(r) => {
                        ui::tool_result(&r);
                        r
                    }
                    Err(e) => {
                        ui::tool_error(&e);
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
            continue;
        }

        if resp.content.as_ref().map_or(false, |s| !s.is_empty()) {
            ui::assistant_line();
        }
        break;
    }

    // --- Phase 5: Final check (cheap model) ---
    ui::phase("Final check");
    let done_summary = format!(
        "Task was: {}. User said: {}",
        summary, user_prompt
    );
    match planner.completion(FINAL_CHECK_SYSTEM, &done_summary).await {
        Ok(s) if !s.trim().is_empty() => {
            ui::phase_done(&s.trim());
        }
        _ => {
            ui::phase_done("Done.");
        }
    }
}

pub async fn run_repl(api_key: &str, executor: &Executor) {
    ui::welcome();
    loop {
        ui::prompt_line();
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            break;
        }
        let prompt = line.trim().to_string();
        if prompt.is_empty() {
            continue;
        }
        println!();
        run_once(api_key, executor, &prompt).await;
        println!();
    }
}