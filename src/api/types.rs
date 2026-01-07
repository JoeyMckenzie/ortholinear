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

    let date_part = if date_str.len() >= 10 {
        &date_str[..10]
    } else {
        date_str
    };

    if let Ok(date) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
        date.format("%m-%d-%Y").to_string()
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
    pub team: Option<Team>,
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
    #[allow(dead_code)]
    pub color: String,
    /// Workflow state category: "Backlog", "Active", "Completed", or "Canceled"
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
    #[allow(dead_code)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct Comment {
    #[allow(dead_code)]
    pub id: String,
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub user: Option<User>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueHistory {
    #[allow(dead_code)]
    pub id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub actor: Option<User>,
    #[serde(rename = "fromState")]
    pub from_state: Option<WorkflowState>,
    #[serde(rename = "toState")]
    pub to_state: Option<WorkflowState>,
    #[serde(rename = "fromAssignee")]
    pub from_assignee: Option<User>,
    #[serde(rename = "toAssignee")]
    pub to_assignee: Option<User>,
}

#[derive(Debug, Deserialize)]
pub struct IssueActivityResponse {
    pub issue: Option<IssueWithActivity>,
}

#[derive(Debug, Deserialize)]
pub struct IssueWithActivity {
    pub comments: Connection<Comment>,
    pub history: Connection<IssueHistory>,
}

// NOTE: Currently tracking essential activity types only (comments, status, assignee).
// Future expansion may include: priority changes, cycle/project changes, labels, estimates, etc.
#[derive(Debug, Clone)]
pub enum TimelineEvent {
    Comment {
        user: String,
        body: String,
        created_at: String,
    },
    StatusChange {
        actor: String,
        from: String,
        to: String,
        created_at: String,
    },
    AssigneeChange {
        actor: String,
        from: Option<String>,
        to: Option<String>,
        created_at: String,
    },
}

impl TimelineEvent {
    pub fn created_at(&self) -> &str {
        match self {
            TimelineEvent::Comment { created_at, .. } => created_at,
            TimelineEvent::StatusChange { created_at, .. } => created_at,
            TimelineEvent::AssigneeChange { created_at, .. } => created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomView {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    #[allow(dead_code)]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CustomViewsResponse {
    #[serde(rename = "customViews")]
    pub custom_views: Connection<CustomView>,
}

#[derive(Debug, Deserialize)]
pub struct CustomViewIssuesResponse {
    #[serde(rename = "customView")]
    pub custom_view: CustomViewWithIssues,
}

#[derive(Debug, Deserialize)]
pub struct CustomViewWithIssues {
    pub issues: Connection<Issue>,
}
