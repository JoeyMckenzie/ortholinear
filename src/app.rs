use crate::api::{Cycle, Issue, LinearClient, Team, WorkflowState};
use crate::fuzzy::{filter_items, FilteredItem};
use anyhow::Result;

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum Mode {
    #[default]
    Normal,
    IssueFilter,
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

    pub issue_filter: String,
    pub filtered_issues: Vec<FilteredItem<Issue>>,
    pub selected_issue_index: usize,

    pub team_filter: String,
    pub filtered_teams: Vec<FilteredItem<Team>>,
    pub selected_team_index: usize,

    pub cycle_filter: String,
    pub filtered_cycles: Vec<FilteredItem<Cycle>>,
    pub selected_cycle_index: usize,

    pub status_filter: String,
    pub filtered_states: Vec<FilteredItem<WorkflowState>>,
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
            issue_filter: String::new(),
            filtered_issues: Vec::new(),
            selected_issue_index: 0,
            team_filter: String::new(),
            filtered_teams: Vec::new(),
            selected_team_index: 0,
            cycle_filter: String::new(),
            filtered_cycles: Vec::new(),
            selected_cycle_index: 0,
            status_filter: String::new(),
            filtered_states: Vec::new(),
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
                self.update_filtered_teams();
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

        match self.client.fetch_cycles(&team_id).await {
            Ok(cycles) => {
                self.cycles = cycles;
                self.update_filtered_cycles();
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
                self.update_filtered_states();
            }
            Err(e) => {
                self.error = Some(format!("Failed to load workflow states: {}", e));
            }
        }

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
                self.issue_filter.clear();
                self.update_filtered_issues();
                self.selected_issue_index = 0;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load issues: {}", e));
            }
        }

        self.loading = false;
        Ok(())
    }

    fn update_filtered_issues(&mut self) {
        self.filtered_issues = filter_items(&self.issues, &self.issue_filter, |issue| {
            format!("{} {}", issue.identifier, issue.title)
        });
    }

    fn update_filtered_teams(&mut self) {
        self.filtered_teams = filter_items(&self.teams, &self.team_filter, |team| {
            format!("{} {}", team.name, team.key)
        });
    }

    fn update_filtered_cycles(&mut self) {
        self.filtered_cycles = filter_items(&self.cycles, &self.cycle_filter, |cycle| {
            cycle.display_name()
        });
    }

    fn update_filtered_states(&mut self) {
        self.filtered_states = filter_items(&self.workflow_states, &self.status_filter, |state| {
            state.name.clone()
        });
    }

    pub fn filter_input(&mut self, c: char) {
        match self.mode {
            Mode::IssueFilter => {
                self.issue_filter.push(c);
                self.update_filtered_issues();
                self.selected_issue_index = 0;
            }
            Mode::TeamSelect => {
                self.team_filter.push(c);
                self.update_filtered_teams();
                self.selected_team_index = 0;
            }
            Mode::CycleSelect => {
                self.cycle_filter.push(c);
                self.update_filtered_cycles();
                self.selected_cycle_index = 0;
            }
            Mode::StatusSelect => {
                self.status_filter.push(c);
                self.update_filtered_states();
                self.selected_status_index = 0;
            }
            Mode::Normal => {}
        }
    }

    pub fn filter_backspace(&mut self) {
        match self.mode {
            Mode::IssueFilter => {
                self.issue_filter.pop();
                self.update_filtered_issues();
                self.selected_issue_index = 0;
            }
            Mode::TeamSelect => {
                self.team_filter.pop();
                self.update_filtered_teams();
                self.selected_team_index = 0;
            }
            Mode::CycleSelect => {
                self.cycle_filter.pop();
                self.update_filtered_cycles();
                self.selected_cycle_index = 0;
            }
            Mode::StatusSelect => {
                self.status_filter.pop();
                self.update_filtered_states();
                self.selected_status_index = 0;
            }
            Mode::Normal => {}
        }
    }

    pub fn current_filter(&self) -> &str {
        match self.mode {
            Mode::IssueFilter => &self.issue_filter,
            Mode::TeamSelect => &self.team_filter,
            Mode::CycleSelect => &self.cycle_filter,
            Mode::StatusSelect => &self.status_filter,
            Mode::Normal => "",
        }
    }

    pub async fn update_selected_issue_status(&mut self, _state: &WorkflowState) -> Result<()> {
        let Some(filtered_item) = self.filtered_states.get(self.selected_status_index) else {
            return Ok(());
        };
        let state = filtered_item.item.clone();

        let Some(issue_filtered) = self.filtered_issues.get(self.selected_issue_index) else {
            return Ok(());
        };
        let issue_id = issue_filtered.item.id.clone();
        let original_index = issue_filtered.original_index;

        self.loading = true;

        match self.client.update_issue_status(&issue_id, &state.id).await {
            Ok(updated_issue) => {
                if let Some(issue) = self.issues.get_mut(original_index) {
                    *issue = updated_issue;
                }
                self.update_filtered_issues();
            }
            Err(e) => {
                self.error = Some(format!("Failed to update status: {}", e));
            }
        }

        self.loading = false;
        self.status_filter.clear();
        self.mode = Mode::Normal;
        Ok(())
    }

    pub async fn select_team_from_filter(&mut self) -> Result<()> {
        if let Some(filtered) = self.filtered_teams.get(self.selected_team_index) {
            let team = filtered.item.clone();
            self.current_team = Some(team);
            self.team_filter.clear();
            self.load_team_data().await?;
        }
        self.mode = Mode::Normal;
        Ok(())
    }

    pub async fn select_cycle_from_filter(&mut self) -> Result<()> {
        if let Some(filtered) = self.filtered_cycles.get(self.selected_cycle_index) {
            self.current_cycle = Some(filtered.item.clone());
            self.cycle_filter.clear();
            self.load_issues().await?;
        }
        self.mode = Mode::Normal;
        Ok(())
    }

    pub fn next_issue(&mut self) {
        if !self.filtered_issues.is_empty() {
            self.selected_issue_index =
                (self.selected_issue_index + 1).min(self.filtered_issues.len() - 1);
        }
    }

    pub fn previous_issue(&mut self) {
        self.selected_issue_index = self.selected_issue_index.saturating_sub(1);
    }

    pub fn first_issue(&mut self) {
        self.selected_issue_index = 0;
    }

    pub fn last_issue(&mut self) {
        if !self.filtered_issues.is_empty() {
            self.selected_issue_index = self.filtered_issues.len() - 1;
        }
    }

    pub fn selected_issue(&self) -> Option<&Issue> {
        self.filtered_issues
            .get(self.selected_issue_index)
            .map(|f| &f.item)
    }

    pub fn next_picker_item(&mut self) {
        match self.mode {
            Mode::TeamSelect => {
                if !self.filtered_teams.is_empty() {
                    self.selected_team_index =
                        (self.selected_team_index + 1).min(self.filtered_teams.len() - 1);
                }
            }
            Mode::CycleSelect => {
                if !self.filtered_cycles.is_empty() {
                    self.selected_cycle_index =
                        (self.selected_cycle_index + 1).min(self.filtered_cycles.len() - 1);
                }
            }
            Mode::StatusSelect => {
                if !self.filtered_states.is_empty() {
                    self.selected_status_index =
                        (self.selected_status_index + 1).min(self.filtered_states.len() - 1);
                }
            }
            Mode::IssueFilter => {
                self.next_issue();
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
            Mode::IssueFilter => {
                self.previous_issue();
            }
            Mode::Normal => {}
        }
    }

    pub fn enter_issue_filter(&mut self) {
        self.mode = Mode::IssueFilter;
        self.issue_filter.clear();
        self.update_filtered_issues();
        self.selected_issue_index = 0;
    }

    pub fn enter_team_select(&mut self) {
        self.mode = Mode::TeamSelect;
        self.team_filter.clear();
        self.update_filtered_teams();
        self.selected_team_index = self
            .current_team
            .as_ref()
            .and_then(|t| self.filtered_teams.iter().position(|ft| ft.item.id == t.id))
            .unwrap_or(0);
    }

    pub fn enter_cycle_select(&mut self) {
        self.mode = Mode::CycleSelect;
        self.cycle_filter.clear();
        self.update_filtered_cycles();
        self.selected_cycle_index = self
            .current_cycle
            .as_ref()
            .and_then(|c| {
                self.filtered_cycles
                    .iter()
                    .position(|fc| fc.item.id == c.id)
            })
            .unwrap_or(0);
    }

    pub fn enter_status_select(&mut self) {
        let current_state_id = self.selected_issue().map(|i| i.state.id.clone());
        if let Some(state_id) = current_state_id {
            self.mode = Mode::StatusSelect;
            self.status_filter.clear();
            self.update_filtered_states();
            self.selected_status_index = self
                .filtered_states
                .iter()
                .position(|s| s.item.id == state_id)
                .unwrap_or(0);
        }
    }

    pub fn confirm_issue_filter(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn cancel_picker(&mut self) {
        match self.mode {
            Mode::IssueFilter => {
                self.issue_filter.clear();
                self.update_filtered_issues();
            }
            Mode::TeamSelect => {
                self.team_filter.clear();
            }
            Mode::CycleSelect => {
                self.cycle_filter.clear();
            }
            Mode::StatusSelect => {
                self.status_filter.clear();
            }
            Mode::Normal => {}
        }
        self.mode = Mode::Normal;
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn clear_issue_filter(&mut self) {
        self.issue_filter.clear();
        self.update_filtered_issues();
        self.selected_issue_index = 0;
    }
}
