use std::fs;
use std::path::PathBuf;

pub fn load_api_key() -> Option<String> {
    std::env::var("OPENAI_API_KEY").ok().or_else(|| {
        config_path().and_then(|p| {
            fs::read_to_string(p).ok().and_then(|s| {
                for line in s.lines() {
                    let line = line.trim();
                    if line.starts_with("api_key") || line.starts_with("OPENAI_API_KEY") {
                        if let Some(v) = line.split('=').nth(1) {
                            let v = v.trim().trim_matches('"').trim();
                            if !v.is_empty() {
                                return Some(v.to_string());
                            }
                        }
                    }
                }
                None
            })
        })
    })
}

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "zcode", "zcode")
        .map(|d| d.config_dir().join("config.toml"))
}

pub fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "zcode", "zcode").map(|d| d.config_dir().to_path_buf())
}
