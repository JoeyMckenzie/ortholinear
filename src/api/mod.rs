mod client;
pub mod types;

pub use client::LinearClient;
pub use types::*;

use anyhow::Result;

pub trait LinearApi: Send + Sync {
    fn fetch_teams(&self) -> impl std::future::Future<Output = Result<Vec<Team>>> + Send;
    fn fetch_cycles(
        &self,
        team_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<Cycle>>> + Send;
    fn fetch_issues(
        &self,
        team_id: Option<&str>,
        cycle_id: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Vec<Issue>>> + Send;
    fn fetch_workflow_states(
        &self,
        team_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<WorkflowState>>> + Send;
    fn update_issue_status(
        &self,
        issue_id: &str,
        state_id: &str,
    ) -> impl std::future::Future<Output = Result<Issue>> + Send;
}
