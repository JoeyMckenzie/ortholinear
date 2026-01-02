use crate::api::ApiError;
use crate::config::ConfigError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Api(#[from] ApiError),

    #[error("Failed to open browser: {0}")]
    Browser(#[from] std::io::Error),
}
