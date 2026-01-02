use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct Connection<T> {
    pub nodes: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct TeamsResponse {
    pub teams: Connection<Team>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct TeamWithCyclesResponse {
    pub team: TeamWithCycles,
}

#[derive(Debug, Deserialize)]
pub struct TeamWithCycles {
    pub cycles: Connection<Cycle>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cycle {
    pub id: String,
    pub name: Option<String>,
    pub number: i32,
    #[serde(rename = "startsAt")]
    pub starts_at: Option<String>,
    #[serde(rename = "endsAt")]
    pub ends_at: Option<String>,
}

impl Cycle {
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("Cycle {}", self.number))
    }

    pub fn display_with_dates(&self) -> String {
        let name = self.display_name();
        match (&self.starts_at, &self.ends_at) {
            (Some(start), Some(end)) => {
                // Format dates as MM/DD
                let start_formatted = format_date_short(start);
                let end_formatted = format_date_short(end);
                format!("{} ({} - {})", name, start_formatted, end_formatted)
            }
            _ => name,
        }
    }
}

fn format_date_short(date_str: &str) -> String {
    use chrono::NaiveDate;
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        date.format("%m/%d").to_string()
    } else {
        date_str.to_string()
    }
}

#[derive(Debug, Deserialize)]
pub struct IssuesResponse {
    pub issues: Connection<Issue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Issue {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub state: WorkflowState,
    pub assignee: Option<User>,
    pub priority: i32,
    pub project: Option<Project>,
}

/// Represents a workflow state in Linear.
///
/// The `state_type` field contains one of Linear's standard workflow state types:
/// - "unstarted": Initial state (e.g., "Todo", "Backlog")
/// - "started": Active work (e.g., "In Progress")
/// - "completed": Finished work (e.g., "Done")
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowState {
    pub id: String,
    pub name: String,
    pub color: String,
    /// The type of workflow state (e.g., "unstarted", "started", "completed")
    #[serde(rename = "type")]
    pub state_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ViewerResponse {
    pub viewer: User,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowStatesResponse {
    #[serde(rename = "workflowStates")]
    pub workflow_states: Connection<WorkflowState>,
}

#[derive(Debug, Deserialize)]
pub struct IssueUpdateResponse {
    #[serde(rename = "issueUpdate")]
    pub issue_update: IssueUpdatePayload,
}

#[derive(Debug, Deserialize)]
pub struct IssueUpdatePayload {
    pub success: bool,
    pub issue: Option<Issue>,
}
