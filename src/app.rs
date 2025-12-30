use crate::api::{Cycle, Issue, LinearClient, Team, WorkflowState};
use anyhow::Result;

#[derive(Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    TeamSelect,
    CycleSelect,
    StatusSelect,
}

pub struct App {
    pub client: LinearClient,

    pub teams: Vec<Team>,
    pub cycles: Vec<Cycle>,
    pub issues: Vec<Issue>,
    pub workflow_states: Vec<WorkflowState>,

    pub mode: Mode,
    pub selected_issue_index: usize,
    pub selected_team_index: usize,
    pub selected_cycle_index: usize,
    pub selected_status_index: usize,

    pub current_team: Option<Team>,
    pub current_cycle: Option<Cycle>,

    pub loading: bool,
    pub error: Option<String>,
}

impl App {
    pub fn new(client: LinearClient) -> Self {
        Self {
            client,
            teams: Vec::new(),
            cycles: Vec::new(),
            issues: Vec::new(),
            workflow_states: Vec::new(),
            mode: Mode::Normal,
            selected_issue_index: 0,
            selected_team_index: 0,
            selected_cycle_index: 0,
            selected_status_index: 0,
            current_team: None,
            current_cycle: None,
            loading: false,
            error: None,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        self.loading = true;
        self.error = None;

        match self.client.fetch_teams().await {
            Ok(teams) => {
                self.teams = teams;
                if let Some(team) = self.teams.first().cloned() {
                    self.current_team = Some(team);
                    self.load_team_data().await?;
                }
            }
            Err(e) => {
                self.error = Some(format!("Failed to load teams: {}", e));
            }
        }

        self.loading = false;
        Ok(())
    }

    pub async fn load_team_data(&mut self) -> Result<()> {
        let Some(team) = &self.current_team else {
            return Ok(());
        };

        self.loading = true;
        let team_id = team.id.clone();

        // Fetch cycles and workflow states for this team
        match self.client.fetch_cycles(&team_id).await {
            Ok(cycles) => {
                self.cycles = cycles;
                // Auto-select active cycle if any
                self.current_cycle = self.cycles.first().cloned();
                self.selected_cycle_index = 0;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load cycles: {}", e));
            }
        }

        match self.client.fetch_workflow_states(&team_id).await {
            Ok(states) => {
                self.workflow_states = states;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load workflow states: {}", e));
            }
        }

        // Load issues
        self.load_issues().await?;
        self.loading = false;
        Ok(())
    }

    pub async fn load_issues(&mut self) -> Result<()> {
        self.loading = true;
        self.error = None;

        let team_id = self.current_team.as_ref().map(|t| t.id.as_str());
        let cycle_id = self.current_cycle.as_ref().map(|c| c.id.as_str());

        match self.client.fetch_issues(team_id, cycle_id).await {
            Ok(issues) => {
                self.issues = issues;
                self.selected_issue_index = 0;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load issues: {}", e));
            }
        }

        self.loading = false;
        Ok(())
    }

    pub async fn update_selected_issue_status(&mut self, state: &WorkflowState) -> Result<()> {
        let Some(issue) = self.selected_issue() else {
            return Ok(());
        };

        let issue_id = issue.id.clone();
        self.loading = true;

        match self.client.update_issue_status(&issue_id, &state.id).await {
            Ok(updated_issue) => {
                // Update the issue in our local list
                if let Some(issue) = self.issues.get_mut(self.selected_issue_index) {
                    *issue = updated_issue;
                }
            }
            Err(e) => {
                self.error = Some(format!("Failed to update status: {}", e));
            }
        }

        self.loading = false;
        self.mode = Mode::Normal;
        Ok(())
    }

    pub async fn select_team(&mut self, index: usize) -> Result<()> {
        if let Some(team) = self.teams.get(index).cloned() {
            self.current_team = Some(team);
            self.selected_team_index = index;
            self.load_team_data().await?;
        }
        self.mode = Mode::Normal;
        Ok(())
    }

    pub async fn select_cycle(&mut self, index: usize) -> Result<()> {
        self.current_cycle = self.cycles.get(index).cloned();
        self.selected_cycle_index = index;
        self.load_issues().await?;
        self.mode = Mode::Normal;
        Ok(())
    }

    // Navigation
    pub fn next_issue(&mut self) {
        if !self.issues.is_empty() {
            self.selected_issue_index = (self.selected_issue_index + 1).min(self.issues.len() - 1);
        }
    }

    pub fn previous_issue(&mut self) {
        self.selected_issue_index = self.selected_issue_index.saturating_sub(1);
    }

    pub fn first_issue(&mut self) {
        self.selected_issue_index = 0;
    }

    pub fn last_issue(&mut self) {
        if !self.issues.is_empty() {
            self.selected_issue_index = self.issues.len() - 1;
        }
    }

    pub fn selected_issue(&self) -> Option<&Issue> {
        self.issues.get(self.selected_issue_index)
    }

    // Picker navigation
    pub fn next_picker_item(&mut self) {
        match self.mode {
            Mode::TeamSelect => {
                if !self.teams.is_empty() {
                    self.selected_team_index =
                        (self.selected_team_index + 1).min(self.teams.len() - 1);
                }
            }
            Mode::CycleSelect => {
                if !self.cycles.is_empty() {
                    self.selected_cycle_index =
                        (self.selected_cycle_index + 1).min(self.cycles.len() - 1);
                }
            }
            Mode::StatusSelect => {
                if !self.workflow_states.is_empty() {
                    self.selected_status_index =
                        (self.selected_status_index + 1).min(self.workflow_states.len() - 1);
                }
            }
            Mode::Normal => {}
        }
    }

    pub fn previous_picker_item(&mut self) {
        match self.mode {
            Mode::TeamSelect => {
                self.selected_team_index = self.selected_team_index.saturating_sub(1);
            }
            Mode::CycleSelect => {
                self.selected_cycle_index = self.selected_cycle_index.saturating_sub(1);
            }
            Mode::StatusSelect => {
                self.selected_status_index = self.selected_status_index.saturating_sub(1);
            }
            Mode::Normal => {}
        }
    }

    pub fn enter_team_select(&mut self) {
        self.mode = Mode::TeamSelect;
        self.selected_team_index = self
            .current_team
            .as_ref()
            .and_then(|t| self.teams.iter().position(|team| team.id == t.id))
            .unwrap_or(0);
    }

    pub fn enter_cycle_select(&mut self) {
        self.mode = Mode::CycleSelect;
        self.selected_cycle_index = self
            .current_cycle
            .as_ref()
            .and_then(|c| self.cycles.iter().position(|cycle| cycle.id == c.id))
            .unwrap_or(0);
    }

    pub fn enter_status_select(&mut self) {
        let current_state_id = self.selected_issue().map(|i| i.state.id.clone());
        if let Some(state_id) = current_state_id {
            self.mode = Mode::StatusSelect;
            self.selected_status_index = self
                .workflow_states
                .iter()
                .position(|s| s.id == state_id)
                .unwrap_or(0);
        }
    }

    pub fn cancel_picker(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }
}
