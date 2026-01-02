use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Failed to connect to Linear API: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Linear API returned {status}: {body}")]
    Http { status: StatusCode, body: String },

    #[error("GraphQL error: {}", .messages.join(", "))]
    GraphQL { messages: Vec<String> },

    #[error("Linear API returned no data for: {context}")]
    MissingData { context: &'static str },

    #[error("Failed to update issue status")]
    UpdateFailed,
}
