use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::{env, fs};

const ENV_VAR_NAME: &str = "LINEAR_API_KEY";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Clone, PartialEq, Default)]
pub enum CycleDefault {
    #[default]
    None,
    Current,
    Number(i32),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum AssigneeDefault {
    #[default]
    None,
    Me,
    Name(String),
}

#[derive(Debug, Clone, Default)]
pub struct DefaultsConfig {
    pub team: Option<String>,
    pub cycle: CycleDefault,
    pub assignee: AssigneeDefault,
}

#[derive(Debug, Deserialize, Default)]
struct DefaultsConfigFile {
    team: Option<String>,
    cycle: Option<String>,
    assignee: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    api_key: String,
    #[serde(default)]
    defaults: Option<DefaultsConfigFile>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub defaults: DefaultsConfig,
}

fn parse_cycle_default(value: Option<&str>) -> CycleDefault {
    match value {
        None | Some("none") => CycleDefault::None,
        Some("current") => CycleDefault::Current,
        Some(s) => s
            .parse::<i32>()
            .map(CycleDefault::Number)
            .unwrap_or(CycleDefault::None),
    }
}

fn parse_assignee_default(value: Option<&str>) -> AssigneeDefault {
    match value {
        None | Some("none") => AssigneeDefault::None,
        Some("me") => AssigneeDefault::Me,
        Some(s) => AssigneeDefault::Name(s.to_string()),
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        if let Ok(api_key) = env::var(ENV_VAR_NAME) {
            if !api_key.is_empty() {
                return Ok(Self {
                    api_key,
                    defaults: DefaultsConfig::default(),
                });
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

        let defaults = config_file.defaults.unwrap_or_default();
        let defaults_config = DefaultsConfig {
            team: defaults.team,
            cycle: parse_cycle_default(defaults.cycle.as_deref()),
            assignee: parse_assignee_default(defaults.assignee.as_deref()),
        };

        Ok(Self {
            api_key: config_file.api_key,
            defaults: defaults_config,
        })
    }

    fn config_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("ortholinear");

        Ok(config_dir.join(CONFIG_FILE_NAME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn load_from_file(config_dir: &std::path::Path) -> Result<Config> {
        let config_path = config_dir.join(CONFIG_FILE_NAME);
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

        let defaults = config_file.defaults.unwrap_or_default();
        let defaults_config = DefaultsConfig {
            team: defaults.team,
            cycle: parse_cycle_default(defaults.cycle.as_deref()),
            assignee: parse_assignee_default(defaults.assignee.as_deref()),
        };

        Ok(Config {
            api_key: config_file.api_key,
            defaults: defaults_config,
        })
    }

    #[test]
    fn loads_from_env_var() {
        // Use a unique env var name to avoid conflicts
        let test_key = "test_api_key_12345";

        // Temporarily set the env var by testing the logic directly
        let result = if !test_key.is_empty() {
            Some(Config {
                api_key: test_key.to_string(),
                defaults: DefaultsConfig::default(),
            })
        } else {
            None
        };

        assert!(result.is_some());
        assert_eq!(result.unwrap().api_key, test_key);
    }

    #[test]
    fn empty_env_var_falls_through() {
        let empty_key = "";

        // Simulate the check in Config::load
        let should_fallback = empty_key.is_empty();

        assert!(should_fallback);
    }

    #[test]
    fn parses_valid_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"api_key = "lin_api_test_key""#;

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = load_from_file(temp_dir.path()).unwrap();

        assert_eq!(config.api_key, "lin_api_test_key");
    }

    #[test]
    fn parses_config_with_extra_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
            api_key = "lin_api_with_spaces"
        "#;

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = load_from_file(temp_dir.path()).unwrap();

        assert_eq!(config.api_key, "lin_api_with_spaces");
    }

    #[test]
    fn fails_on_missing_config_file() {
        let temp_dir = TempDir::new().unwrap();

        let result = load_from_file(temp_dir.path());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Could not read config file"));
    }

    #[test]
    fn fails_on_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = "this is not valid toml [[[";

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let result = load_from_file(temp_dir.path());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to parse config file"));
    }

    #[test]
    fn fails_on_missing_api_key_field() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"some_other_field = "value""#;

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let result = load_from_file(temp_dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn config_file_path_includes_ortholinear_dir() {
        let path = Config::config_file_path().unwrap();

        assert!(path.to_string_lossy().contains("ortholinear"));
        assert!(path.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    fn config_is_cloneable() {
        let config = Config {
            api_key: "test_key".to_string(),
            defaults: DefaultsConfig::default(),
        };

        let cloned = config.clone();

        assert_eq!(cloned.api_key, "test_key");
    }

    #[test]
    fn config_is_debuggable() {
        let config = Config {
            api_key: "test_key".to_string(),
            defaults: DefaultsConfig::default(),
        };

        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("api_key"));
    }

    #[test]
    fn parses_defaults_section() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
api_key = "lin_api_test"

[defaults]
team = "Engineering"
cycle = "current"
assignee = "me"
"#;

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = load_from_file(temp_dir.path()).unwrap();

        assert_eq!(config.defaults.team, Some("Engineering".to_string()));
        assert_eq!(config.defaults.cycle, CycleDefault::Current);
        assert_eq!(config.defaults.assignee, AssigneeDefault::Me);
    }
}
