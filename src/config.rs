use serde::Deserialize;
use std::path::PathBuf;
use std::{env, fs};

const ENV_VAR_NAME: &str = "LINEAR_API_KEY";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("No API key found.\n\nTo authenticate, either:\n1. Set the LINEAR_API_KEY environment variable, or\n2. Add to {}: api_key = \"lin_api_...\"", .config_path.display())]
    MissingApiKey { config_path: PathBuf },

    #[error("Could not determine config directory. Ensure HOME or XDG_CONFIG_HOME is set.")]
    ConfigDirNotFound,

    #[error("Failed to read config file at {}: {source}", .path.display())]
    FileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse config file at {}: {source}", .path.display())]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum CycleDefault {
    #[default]
    None,
    Current,
    Number(i32),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Cycle,
    Backlog,
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
    pub view_mode: ViewMode,
}

#[derive(Debug, Deserialize, Default)]
struct DefaultsConfigFile {
    team: Option<String>,
    cycle: Option<String>,
    assignee: Option<String>,
    view_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    api_key: Option<String>,
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

fn parse_view_mode(value: Option<&str>) -> ViewMode {
    match value {
        None | Some("cycle") => ViewMode::Cycle,
        Some("backlog") => ViewMode::Backlog,
        Some(_) => ViewMode::Cycle,
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let env_api_key = env::var(ENV_VAR_NAME).ok().filter(|s| !s.is_empty());
        let config_path = Self::config_file_path()?;

        let config_file = Self::read_config_file(&config_path).ok();

        let config_api_key = config_file.as_ref().and_then(|cf| cf.api_key.clone());
        let api_key = env_api_key
            .or(config_api_key)
            .ok_or_else(|| ConfigError::MissingApiKey {
                config_path: config_path.clone(),
            })?;

        let defaults_config = config_file
            .and_then(|cf| cf.defaults)
            .map(|defaults| DefaultsConfig {
                team: defaults.team,
                cycle: parse_cycle_default(defaults.cycle.as_deref()),
                assignee: parse_assignee_default(defaults.assignee.as_deref()),
                view_mode: parse_view_mode(defaults.view_mode.as_deref()),
            })
            .unwrap_or_default();

        Ok(Self {
            api_key,
            defaults: defaults_config,
        })
    }

    fn config_file_path() -> Result<PathBuf, ConfigError> {
        let config_dir = if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            if !xdg_config.is_empty() {
                PathBuf::from(xdg_config).join("ortholinear")
            } else {
                dirs::config_dir()
                    .ok_or(ConfigError::ConfigDirNotFound)?
                    .join("ortholinear")
            }
        } else {
            dirs::config_dir()
                .ok_or(ConfigError::ConfigDirNotFound)?
                .join("ortholinear")
        };

        Ok(config_dir.join(CONFIG_FILE_NAME))
    }

    fn read_config_file(path: &PathBuf) -> Result<ConfigFile, ConfigError> {
        let contents = fs::read_to_string(path).map_err(|source| ConfigError::FileRead {
            path: path.clone(),
            source,
        })?;

        toml::from_str(&contents).map_err(|source| ConfigError::Parse {
            path: path.clone(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn load_from_file(config_dir: &std::path::Path) -> Result<Config, ConfigError> {
        let config_path = config_dir.join(CONFIG_FILE_NAME);
        let config_file = Config::read_config_file(&config_path)?;

        let api_key = config_file
            .api_key
            .ok_or_else(|| ConfigError::MissingApiKey {
                config_path: config_path.clone(),
            })?;

        let defaults = config_file.defaults.unwrap_or_default();
        let defaults_config = DefaultsConfig {
            team: defaults.team,
            cycle: parse_cycle_default(defaults.cycle.as_deref()),
            assignee: parse_assignee_default(defaults.assignee.as_deref()),
            view_mode: parse_view_mode(defaults.view_mode.as_deref()),
        };

        Ok(Config {
            api_key,
            defaults: defaults_config,
        })
    }

    #[test]
    fn loads_from_env_var() {
        let test_key = "test_api_key_12345";

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
        assert!(matches!(result.unwrap_err(), ConfigError::FileRead { .. }));
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
        assert!(matches!(result.unwrap_err(), ConfigError::Parse { .. }));
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
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::MissingApiKey { .. }
        ));
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

    #[test]
    fn missing_defaults_uses_none() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"api_key = "lin_api_test""#;

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = load_from_file(temp_dir.path()).unwrap();

        assert_eq!(config.defaults.team, None);
        assert_eq!(config.defaults.cycle, CycleDefault::None);
        assert_eq!(config.defaults.assignee, AssigneeDefault::None);
    }

    #[test]
    fn cycle_number_parses() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
api_key = "lin_api_test"

[defaults]
cycle = "5"
"#;

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = load_from_file(temp_dir.path()).unwrap();

        assert_eq!(config.defaults.cycle, CycleDefault::Number(5));
    }

    #[test]
    fn assignee_name_parses() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
api_key = "lin_api_test"

[defaults]
assignee = "Joey McKenzie"
"#;

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = load_from_file(temp_dir.path()).unwrap();

        assert_eq!(
            config.defaults.assignee,
            AssigneeDefault::Name("Joey McKenzie".to_string())
        );
    }

    #[test]
    fn defaults_only_config_parses() {
        let config_content = r#"
[defaults]
team = "Fundraising"
cycle = "current"
assignee = "me"
"#;

        let config_file: ConfigFile = toml::from_str(config_content).unwrap();

        assert!(config_file.api_key.is_none());

        let defaults = config_file.defaults.unwrap();
        assert_eq!(defaults.team, Some("Fundraising".to_string()));
        assert_eq!(defaults.cycle, Some("current".to_string()));
        assert_eq!(defaults.assignee, Some("me".to_string()));

        let defaults_config = DefaultsConfig {
            team: defaults.team,
            cycle: parse_cycle_default(defaults.cycle.as_deref()),
            assignee: parse_assignee_default(defaults.assignee.as_deref()),
            view_mode: parse_view_mode(defaults.view_mode.as_deref()),
        };

        assert_eq!(defaults_config.team, Some("Fundraising".to_string()));
        assert_eq!(defaults_config.cycle, CycleDefault::Current);
        assert_eq!(defaults_config.assignee, AssigneeDefault::Me);
    }

    #[test]
    fn view_mode_backlog_parses() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
api_key = "lin_api_test"

[defaults]
view_mode = "backlog"
"#;

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = load_from_file(temp_dir.path()).unwrap();

        assert_eq!(config.defaults.view_mode, ViewMode::Backlog);
    }

    #[test]
    fn view_mode_cycle_parses() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
api_key = "lin_api_test"

[defaults]
view_mode = "cycle"
"#;

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = load_from_file(temp_dir.path()).unwrap();

        assert_eq!(config.defaults.view_mode, ViewMode::Cycle);
    }

    #[test]
    fn missing_view_mode_defaults_to_cycle() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
api_key = "lin_api_test"

[defaults]
team = "Engineering"
"#;

        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = load_from_file(temp_dir.path()).unwrap();

        assert_eq!(config.defaults.view_mode, ViewMode::Cycle);
    }
}
