use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::{env, fs};

const ENV_VAR_NAME: &str = "LINEAR_API_KEY";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Deserialize)]
struct ConfigFile {
    api_key: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        if let Ok(api_key) = env::var(ENV_VAR_NAME) {
            if !api_key.is_empty() {
                return Ok(Self { api_key });
            }
        }

        let config_path = Self::config_file_path()?;
        let contents = fs::read_to_string(&config_path).with_context(|| {
            format!(
                "Could not read config file at {:?}\n\n\
                To authenticate, either:\n\
                1. Set the {} environment variable, or\n\
                2. Create {:?} with:\n\n\
                api_key = \"lin_api_...\"",
                config_path, ENV_VAR_NAME, config_path
            )
        })?;

        let config_file: ConfigFile = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file at {:?}", config_path))?;

        Ok(Self {
            api_key: config_file.api_key,
        })
    }

    fn config_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("ortholinear");

        Ok(config_dir.join(CONFIG_FILE_NAME))
    }
}
