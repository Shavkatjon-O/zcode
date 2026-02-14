use crate::agent::ToolCall;
use std::fs;
use std::io::Write;
use std::process::Command;

pub struct Executor {
    workspace: std::path::PathBuf,
}

impl Executor {
    pub fn new(workspace: std::path::PathBuf) -> Self {
        Self { workspace }
    }

    pub fn execute(&self, tool_call: &ToolCall) -> Result<String, String> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).map_err(|e| e.to_string())?;

        match tool_call.function.name.as_str() {
            "create_file" | "write_file" => {
                let path = args["path"].as_str().ok_or("Missing path")?;
                let content = args["content"].as_str().ok_or("Missing content")?;
                let full_path = self.workspace.join(path);
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut f = fs::File::create(&full_path).map_err(|e| e.to_string())?;
                f.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
                Ok(format!("Created {}", path))
            }
            "read_file" => {
                let path = args["path"].as_str().ok_or("Missing path")?;
                let full_path = self.workspace.join(path);
                let content = fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
                Ok(content)
            }
            "list_dir" => {
                let path = args["path"].as_str().unwrap_or(".");
                let full_path = self.workspace.join(path);
                let entries = fs::read_dir(&full_path).map_err(|e| e.to_string())?;
                let mut names: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                names.sort();
                Ok(names.join("\n"))
            }
            "run_command" => {
                let cmd = args["command"].as_str().ok_or("Missing command")?;
                let output = Command::new("sh")
                    .args(["-c", cmd])
                    .current_dir(&self.workspace)
                    .output()
                    .map_err(|e| e.to_string())?;
                let mut result = String::from_utf8_lossy(&output.stdout).to_string();
                if !output.stderr.is_empty() {
                    result.push_str(&format!("\nstderr: {}", String::from_utf8_lossy(&output.stderr)));
                }
                if !output.status.success() {
                    result.push_str(&format!("\nexit code: {}", output.status));
                }
                Ok(result)
            }
            "create_directory" => {
                let path = args["path"].as_str().ok_or("Missing path")?;
                let full_path = self.workspace.join(path);
                fs::create_dir_all(&full_path).map_err(|e| e.to_string())?;
                Ok(format!("Created directory {}", path))
            }
            _ => Err(format!("Unknown tool: {}", tool_call.function.name)),
        }
    }
}
