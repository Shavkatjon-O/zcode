use crate::agent::AgentProvider;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "zcode", "zcode")
        .map(|d| d.config_dir().join("config.toml"))
}

fn config_content() -> Option<String> {
    config_path().and_then(|p| fs::read_to_string(p).ok())
}

fn get_config_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(key) {
            if let Some(v) = line.split('=').nth(1) {
                let v = v.trim().trim_matches('"').trim();
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

pub fn load_provider() -> AgentProvider {
    std::env::var("ZCODE_PROVIDER")
        .ok()
        .and_then(|s| AgentProvider::from_str(&s).ok())
        .or_else(|| {
            config_content().and_then(|c| get_config_value(&c, "provider"))
                .and_then(|s| AgentProvider::from_str(&s).ok())
        })
        .unwrap_or(AgentProvider::OpenAi)
}

pub fn load_api_key(provider: AgentProvider) -> Option<String> {
    let (env_var, config_key) = match provider {
        AgentProvider::OpenAi => ("OPENAI_API_KEY", "api_key"),
        AgentProvider::Gemini => ("GEMINI_API_KEY", "gemini_api_key"),
    };

    std::env::var(env_var).ok().or_else(|| {
        config_content().and_then(|c| {
            get_config_value(&c, env_var).or_else(|| get_config_value(&c, config_key))
        })
    })
}

pub fn load_api_key_openai() -> Option<String> {
    load_api_key(AgentProvider::OpenAi)
}

pub fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "zcode", "zcode").map(|d| d.config_dir().to_path_buf())
}
