use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub language: String,
    pub theme: String,
    pub line_numbers: bool,
    pub vi_mode: bool,
    pub word_wrap: bool,
    pub tab_size: usize,
    pub expand_tab: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            theme: "terminal-default".to_string(),
            line_numbers: true,
            vi_mode: false,
            word_wrap: false,
            tab_size: 4,
            expand_tab: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut config = Config::default();
        if let Some(path) = Config::config_file_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(raw) = toml_span::parse(&content) {
                        if let Ok(loaded) = Config::deserialize_from_value(&raw) {
                            config = loaded;
                        }
                    }
                }
            }
        }
        config
    }

    fn deserialize_from_value(value: &toml_span::Value) -> Result<Self> {
        let mut config = Config::default();
        if let Some(table) = value.as_table() {
            for (k, v) in table {
                let key_str = k.name.as_ref();
                match key_str {
                    "language" => if let Some(s) = v.as_str() { config.language = s.to_string(); },
                    "theme" => if let Some(s) = v.as_str() { config.theme = s.to_string(); },
                    "line_numbers" => if let Some(b) = v.as_bool() { config.line_numbers = b; },
                    "vi_mode" => if let Some(b) = v.as_bool() { config.vi_mode = b; },
                    "word_wrap" => if let Some(b) = v.as_bool() { config.word_wrap = b; },
                    "tab_size" => if let Some(i) = v.as_integer() { config.tab_size = i as usize; },
                    "expand_tab" => if let Some(b) = v.as_bool() { config.expand_tab = b; },
                    _ => {}
                }
            }
        }
        Ok(config)
    }

    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join("led"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config").join("led"))
        }
    }

    pub fn config_file_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("config.toml"))
    }

    pub fn themes_dir() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("themes"))
    }

    pub fn syntax_dir() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("syntax"))
    }

    pub fn write_key(key: &str, value: &str) -> Result<()> {
        let path = Self::config_file_path().context("Could not determine config path")?;
        let dir = path.parent().context("Could not determine config directory")?;
        
        if !dir.exists() {
            fs::create_dir_all(dir).context("Failed to create config directory")?;
        }

        let content = if path.exists() {
            fs::read_to_string(&path).context("Failed to read config file")?
        } else {
            "# led configuration file\n\n".to_string()
        };

        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut found = false;
        for line in lines.iter_mut() {
            if line.trim().starts_with(key) && line.contains('=') {
                *line = format!("{} = {}", key, value);
                found = true;
                break;
            }
        }

        if !found {
            lines.push(format!("{} = {}", key, value));
        }

        fs::write(&path, lines.join("\n")).context("Failed to write config file")?;
        Ok(())
    }
}
