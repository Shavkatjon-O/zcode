use std::fs;
use std::path::PathBuf;

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

pub fn load_api_key() -> Option<String> {
    const ENV_VAR: &str = "OPENAI_API_KEY";
    const CONFIG_KEY: &str = "api_key";

    std::env::var(ENV_VAR).ok().or_else(|| {
        config_content().and_then(|c| {
            get_config_value(&c, ENV_VAR).or_else(|| get_config_value(&c, CONFIG_KEY))
        })
    })
}

pub fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "zcode", "zcode").map(|d| d.config_dir().to_path_buf())
}
