mod client;
pub mod error;
pub mod types;

pub use client::LinearClient;
pub use error::ApiError;
pub use types::*;

pub trait LinearApi: Send + Sync {
    fn fetch_teams(&self) -> impl std::future::Future<Output = Result<Vec<Team>, ApiError>> + Send;
    fn fetch_cycles(
        &self,
        team_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<Cycle>, ApiError>> + Send;
    fn fetch_issues(
        &self,
        team_id: Option<&str>,
        cycle_id: Option<&str>,
        assignee_id: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Vec<Issue>, ApiError>> + Send;
    fn fetch_backlog_issues(
        &self,
        team_id: &str,
        assignee_id: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Vec<Issue>, ApiError>> + Send;
    fn fetch_workflow_states(
        &self,
        team_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<WorkflowState>, ApiError>> + Send;
    fn update_issue_status(
        &self,
        issue_id: &str,
        state_id: &str,
    ) -> impl std::future::Future<Output = Result<Issue, ApiError>> + Send;
    fn fetch_viewer(&self) -> impl std::future::Future<Output = Result<User, ApiError>> + Send;
}
