use crate::api::{Cycle, Issue, LinearApi, Team, TimelineEvent, User, WorkflowState};
use crate::config::{AssigneeDefault, Config, CycleDefault};
use crate::error::AppError;
use crate::fuzzy::{filter_items, FilteredItem};
use chrono::NaiveDate;
use std::collections::HashMap;

#[cfg(test)]
use crate::api::ApiError;

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Mode {
    #[default]
    Normal,
    DetailView,
    IssueFilter,
    TeamSelect,
    CycleSelect,
    StatusSelect,
    Search,
    SearchResults,
    ViewSelect,
}

pub struct App<C: LinearApi> {
    pub client: C,
    pub config: Config,

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

    pub backlog_mode: bool,

    pub viewer: Option<User>,
    pub filter_my_issues: bool,

    pub loading: bool,
    pub error: Option<String>,

    pub detail_scroll_offset: u16,
    pub detail_content_height: u16,
    pub detail_viewport_height: u16,

    pub pending_description_edit: Option<String>,
    pub timeline_events: Vec<TimelineEvent>,
    pub timeline_loading: bool,
    pub timeline_cache: HashMap<String, Vec<TimelineEvent>>,

    pub search_query: String,
    pub search_results: Vec<Issue>,
    pub in_search_context: bool,
    pub selected_search_index: usize,
}

/// Parse a date string that may be in ISO 8601 format (e.g., "2024-12-30T00:00:00.000Z")
/// or simple date format (e.g., "2024-12-30")
fn parse_date(s: &str) -> Option<NaiveDate> {
    // Take first 10 chars to handle ISO 8601 format
    let date_part = if s.len() >= 10 { &s[..10] } else { s };
    NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
}

pub fn find_current_cycle(cycles: &[Cycle]) -> Option<&Cycle> {
    let today = chrono::Utc::now().date_naive();

    cycles.iter().find(|cycle| {
        let starts = cycle.starts_at.as_ref().and_then(|s| parse_date(s));
        let ends = cycle.ends_at.as_ref().and_then(|s| parse_date(s));

        match (starts, ends) {
            (Some(start), Some(end)) => today >= start && today <= end,
            _ => false,
        }
    })
}

/// Merge comments and history into a sorted timeline.
/// Only includes status and assignee changes from history.
pub fn merge_timeline_events(
    comments: Vec<crate::api::Comment>,
    history: Vec<crate::api::IssueHistory>,
) -> Vec<TimelineEvent> {
    let mut events: Vec<TimelineEvent> = Vec::new();

    // Convert comments
    for comment in comments {
        events.push(TimelineEvent::Comment {
            user: comment
                .user
                .map(|u| u.name)
                .unwrap_or_else(|| "Unknown".to_string()),
            body: comment.body,
            created_at: comment.created_at,
        });
    }

    // Convert history (only status and assignee changes)
    for entry in history {
        let actor_name = entry
            .actor
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "System".to_string());

        // Status change
        if entry.from_state.is_some() || entry.to_state.is_some() {
            let from = entry
                .from_state
                .map(|s| s.name)
                .unwrap_or_else(|| "None".to_string());
            let to = entry
                .to_state
                .map(|s| s.name)
                .unwrap_or_else(|| "None".to_string());
            events.push(TimelineEvent::StatusChange {
                actor: actor_name.clone(),
                from,
                to,
                created_at: entry.created_at.clone(),
            });
        }

        // Assignee change
        if entry.from_assignee.is_some() || entry.to_assignee.is_some() {
            events.push(TimelineEvent::AssigneeChange {
                actor: actor_name,
                from: entry.from_assignee.map(|u| u.name),
                to: entry.to_assignee.map(|u| u.name),
                created_at: entry.created_at,
            });
        }
    }

    // Sort by created_at ascending (oldest first)
    events.sort_by(|a, b| a.created_at().cmp(b.created_at()));

    events
}

impl<C: LinearApi> App<C> {
    pub fn new(client: C, config: Config) -> Self {
        let filter_my_issues = matches!(config.defaults.assignee, AssigneeDefault::Me);

        Self {
            client,
            config,
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
            backlog_mode: false,
            viewer: None,
            filter_my_issues,
            loading: false,
            error: None,
            detail_scroll_offset: 0,
            detail_content_height: 0,
            detail_viewport_height: 0,
            pending_description_edit: None,
            timeline_events: Vec::new(),
            timeline_loading: false,
            timeline_cache: HashMap::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            in_search_context: false,
            selected_search_index: 0,
        }
    }

    pub async fn init(&mut self) -> Result<(), AppError> {
        self.loading = true;
        self.error = None;

        self.viewer = self.client.fetch_viewer().await.ok();

        self.backlog_mode = matches!(
            self.config.defaults.view_mode,
            crate::config::ViewMode::Backlog
        );

        match self.client.fetch_teams().await {
            Ok(teams) => {
                self.teams = teams;
                self.update_filtered_teams();

                // Apply team default from config, or fall back to first team
                let default_team = self.config.defaults.team.as_ref().and_then(|name| {
                    self.teams
                        .iter()
                        .find(|t| {
                            t.name.eq_ignore_ascii_case(name) || t.key.eq_ignore_ascii_case(name)
                        })
                        .cloned()
                });
                let team = default_team.or_else(|| self.teams.first().cloned());

                if let Some(team) = team {
                    self.current_team = Some(team);
                    self.load_team_data().await?;

                    // If backlog mode is configured, switch to backlog view
                    if self.backlog_mode {
                        self.load_backlog_issues().await?;
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Failed to load teams: {}", e));
            }
        }

        self.loading = false;
        Ok(())
    }

    pub async fn load_team_data(&mut self) -> Result<(), AppError> {
        let Some(team) = &self.current_team else {
            return Ok(());
        };

        self.loading = true;
        let team_id = team.id.clone();

        match self.client.fetch_cycles(&team_id).await {
            Ok(cycles) => {
                self.cycles = cycles;
                self.update_filtered_cycles();
                // Apply cycle default
                self.current_cycle = match &self.config.defaults.cycle {
                    CycleDefault::Current => find_current_cycle(&self.cycles).cloned(),
                    CycleDefault::Number(n) => self.cycles.iter().find(|c| c.number == *n).cloned(),
                    CycleDefault::None => self.cycles.first().cloned(),
                };
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

    pub async fn load_issues(&mut self) -> Result<(), AppError> {
        self.loading = true;
        self.error = None;
        self.clear_timeline_cache();

        let team_id = self.current_team.as_ref().map(|t| t.id.as_str());
        let cycle_id = self.current_cycle.as_ref().map(|c| c.id.as_str());
        let assignee_id = if self.filter_my_issues {
            self.viewer.as_ref().map(|v| v.id.as_str())
        } else {
            None
        };

        match self
            .client
            .fetch_issues(team_id, cycle_id, assignee_id)
            .await
        {
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

    pub async fn load_backlog_issues(&mut self) -> Result<(), AppError> {
        self.loading = true;
        self.error = None;
        self.clear_timeline_cache();

        let Some(team) = &self.current_team else {
            self.loading = false;
            return Ok(());
        };

        let team_id = team.id.as_str();
        let assignee_id = if self.filter_my_issues {
            self.viewer.as_ref().map(|v| v.id.as_str())
        } else {
            None
        };

        match self.client.fetch_backlog_issues(team_id, assignee_id).await {
            Ok(issues) => {
                self.issues = issues;
                self.issue_filter.clear();
                self.update_filtered_issues();
                self.selected_issue_index = 0;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load backlog: {}", e));
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
            Mode::Normal | Mode::DetailView | Mode::Search | Mode::SearchResults | Mode::ViewSelect => {}
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
            Mode::Normal | Mode::DetailView | Mode::Search | Mode::SearchResults | Mode::ViewSelect => {}
        }
    }

    pub fn current_filter(&self) -> &str {
        match self.mode {
            Mode::IssueFilter => &self.issue_filter,
            Mode::TeamSelect => &self.team_filter,
            Mode::CycleSelect => &self.cycle_filter,
            Mode::StatusSelect => &self.status_filter,
            Mode::Normal | Mode::DetailView | Mode::Search | Mode::SearchResults | Mode::ViewSelect => "",
        }
    }

    pub async fn update_selected_issue_status(
        &mut self,
        _state: &WorkflowState,
    ) -> Result<(), AppError> {
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

    pub fn get_description_for_edit(&self) -> String {
        // Use pending edit if available (retry case), otherwise use current description
        if let Some(pending) = &self.pending_description_edit {
            return pending.clone();
        }
        self.selected_issue()
            .and_then(|i| i.description.clone())
            .unwrap_or_default()
    }

    pub async fn update_selected_issue_description(
        &mut self,
        new_description: &str,
    ) -> Result<(), AppError> {
        let Some(issue_filtered) = self.filtered_issues.get(self.selected_issue_index) else {
            return Ok(());
        };
        let issue_id = issue_filtered.item.id.clone();
        let original_index = issue_filtered.original_index;

        self.loading = true;

        match self
            .client
            .update_issue_description(&issue_id, new_description)
            .await
        {
            Ok(updated_issue) => {
                if let Some(issue) = self.issues.get_mut(original_index) {
                    *issue = updated_issue;
                }
                self.update_filtered_issues();
                self.pending_description_edit = None;
                self.error = None;
            }
            Err(e) => {
                // Store the edit for retry
                self.pending_description_edit = Some(new_description.to_string());
                self.error = Some(format!(
                    "Failed to save description: {} - press 'e' to retry with your changes",
                    e
                ));
            }
        }

        self.loading = false;
        Ok(())
    }

    pub fn clear_pending_description_edit(&mut self) {
        self.pending_description_edit = None;
    }

    pub async fn load_timeline(&mut self) -> Result<(), AppError> {
        let Some(issue) = self.current_issue() else {
            return Ok(());
        };

        let issue_id = issue.id.clone();

        // Check cache first
        if let Some(cached) = self.timeline_cache.get(&issue_id) {
            self.timeline_events = cached.clone();
            return Ok(());
        }

        self.timeline_loading = true;
        self.timeline_events.clear();

        match self.client.fetch_issue_activity(&issue_id).await {
            Ok((comments, history)) => {
                let events = merge_timeline_events(comments, history);
                self.timeline_cache.insert(issue_id, events.clone());
                self.timeline_events = events;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load activity: {}", e));
            }
        }

        self.timeline_loading = false;
        Ok(())
    }

    pub fn clear_timeline(&mut self) {
        self.timeline_events.clear();
        self.timeline_loading = false;
    }

    pub fn clear_timeline_cache(&mut self) {
        self.timeline_cache.clear();
        self.clear_timeline();
    }

    /// Load timeline from cache if available for the current issue
    fn load_timeline_from_cache(&mut self) {
        if let Some(issue) = self.current_issue() {
            if let Some(cached) = self.timeline_cache.get(&issue.id) {
                self.timeline_events = cached.clone();
                return;
            }
        }
        self.timeline_events.clear();
    }

    pub async fn select_team_from_filter(&mut self) -> Result<(), AppError> {
        if let Some(filtered) = self.filtered_teams.get(self.selected_team_index) {
            let team = filtered.item.clone();
            self.current_team = Some(team);
            self.team_filter.clear();
            self.load_team_data().await?;
        }
        self.mode = Mode::Normal;
        Ok(())
    }

    pub async fn select_cycle_from_filter(&mut self) -> Result<(), AppError> {
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
            let new_index = (self.selected_issue_index + 1).min(self.filtered_issues.len() - 1);
            if new_index != self.selected_issue_index {
                self.selected_issue_index = new_index;
                self.detail_scroll_offset = 0;
                self.load_timeline_from_cache();
            }
        }
    }

    pub fn previous_issue(&mut self) {
        let new_index = self.selected_issue_index.saturating_sub(1);
        if new_index != self.selected_issue_index {
            self.selected_issue_index = new_index;
            self.detail_scroll_offset = 0;
            self.load_timeline_from_cache();
        }
    }

    pub fn first_issue(&mut self) {
        if self.selected_issue_index != 0 {
            self.selected_issue_index = 0;
            self.detail_scroll_offset = 0;
            self.load_timeline_from_cache();
        }
    }

    pub fn last_issue(&mut self) {
        if !self.filtered_issues.is_empty() {
            let last = self.filtered_issues.len() - 1;
            if self.selected_issue_index != last {
                self.selected_issue_index = last;
                self.detail_scroll_offset = 0;
                self.load_timeline_from_cache();
            }
        }
    }

    pub fn selected_issue(&self) -> Option<&Issue> {
        self.filtered_issues
            .get(self.selected_issue_index)
            .map(|f| &f.item)
    }

    pub fn current_issue(&self) -> Option<&Issue> {
        if self.in_search_context {
            self.selected_search_result()
        } else {
            self.selected_issue()
        }
    }

    pub async fn enter_detail_view(&mut self) -> Result<(), AppError> {
        let has_issue = if self.in_search_context {
            self.selected_search_result().is_some()
        } else {
            self.selected_issue().is_some()
        };

        if has_issue {
            self.mode = Mode::DetailView;
            self.detail_scroll_offset = 0;

            // Load timeline if we have an issue and cache doesn't have it
            let issue_id = if self.in_search_context {
                self.selected_search_result().map(|i| i.id.clone())
            } else {
                self.selected_issue().map(|i| i.id.clone())
            };

            if let Some(id) = issue_id {
                if !self.timeline_cache.contains_key(&id) {
                    self.load_timeline().await?;
                } else {
                    self.load_timeline_from_cache();
                }
            }
        }
        Ok(())
    }

    pub fn exit_detail_view(&mut self) {
        if self.in_search_context {
            self.mode = Mode::SearchResults;
        } else {
            self.mode = Mode::Normal;
        }
    }

    pub fn scroll_detail_down(&mut self) {
        let max_scroll = self
            .detail_content_height
            .saturating_sub(self.detail_viewport_height);
        if self.detail_scroll_offset < max_scroll {
            self.detail_scroll_offset += 1;
        }
    }

    pub fn scroll_detail_up(&mut self) {
        self.detail_scroll_offset = self.detail_scroll_offset.saturating_sub(1);
    }

    pub fn scroll_detail_top(&mut self) {
        self.detail_scroll_offset = 0;
    }

    pub fn scroll_detail_bottom(&mut self) {
        self.detail_scroll_offset = self
            .detail_content_height
            .saturating_sub(self.detail_viewport_height);
    }

    pub fn open_selected_issue(&self) -> Result<(), AppError> {
        if let Some(issue) = self.selected_issue() {
            open::that(&issue.url)?;
        }
        Ok(())
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
            Mode::Normal | Mode::DetailView | Mode::Search | Mode::SearchResults | Mode::ViewSelect => {}
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
            Mode::Normal | Mode::DetailView | Mode::Search | Mode::SearchResults | Mode::ViewSelect => {}
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
            Mode::Normal | Mode::DetailView | Mode::Search | Mode::SearchResults | Mode::ViewSelect => {}
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

    pub fn toggle_my_issues(&mut self) {
        if self.viewer.is_some() {
            self.filter_my_issues = !self.filter_my_issues;
        }
    }

    pub async fn jump_to_current_cycle(&mut self) -> Result<(), AppError> {
        if let Some(current) = find_current_cycle(&self.cycles) {
            self.current_cycle = Some(current.clone());
            self.cycle_filter.clear();
            self.load_issues().await?;
        }
        Ok(())
    }

    pub fn enter_search(&mut self) {
        self.mode = Mode::Search;
        self.search_query.clear();
    }

    pub fn cancel_search(&mut self) {
        self.search_query.clear();
        self.mode = Mode::Normal;
    }

    pub fn search_input(&mut self, c: char) {
        self.search_query.push(c);
    }

    pub fn search_backspace(&mut self) {
        self.search_query.pop();
    }

    pub async fn execute_search(&mut self) -> Result<(), AppError> {
        if self.search_query.trim().is_empty() {
            self.cancel_search();
            return Ok(());
        }

        self.loading = true;
        let result = self.client.search_issues(&self.search_query).await;
        self.loading = false;

        match result {
            Ok(issues) => {
                self.search_results = issues;
                self.in_search_context = true;
                self.selected_search_index = 0;
                self.mode = Mode::SearchResults;
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
        Ok(())
    }

    pub fn exit_search_results(&mut self) {
        self.mode = Mode::Normal;
        self.in_search_context = false;
        self.search_results.clear();
        self.search_query.clear();
        self.selected_search_index = 0;
    }

    pub fn next_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.selected_search_index =
                (self.selected_search_index + 1).min(self.search_results.len().saturating_sub(1));
        }
    }

    pub fn previous_search_result(&mut self) {
        self.selected_search_index = self.selected_search_index.saturating_sub(1);
    }

    pub fn selected_search_result(&self) -> Option<&Issue> {
        self.search_results.get(self.selected_search_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Comment, IssueHistory, LinearApi, WorkflowState};
    use crate::config::{Config, DefaultsConfig};

    fn mock_config() -> Config {
        Config {
            api_key: "test-key".to_string(),
            defaults: DefaultsConfig::default(),
        }
    }

    struct MockClient {
        teams: Vec<Team>,
        cycles: Vec<Cycle>,
        issues: Vec<Issue>,
        workflow_states: Vec<WorkflowState>,
        viewer: Option<User>,
    }

    impl MockClient {
        fn new() -> Self {
            Self {
                teams: vec![
                    Team {
                        id: "team-1".to_string(),
                        name: "Engineering".to_string(),
                        key: "ENG".to_string(),
                    },
                    Team {
                        id: "team-2".to_string(),
                        name: "Design".to_string(),
                        key: "DES".to_string(),
                    },
                ],
                cycles: vec![
                    Cycle {
                        id: "cycle-1".to_string(),
                        name: Some("Sprint 1".to_string()),
                        number: 1,
                        starts_at: None,
                        ends_at: None,
                    },
                    Cycle {
                        id: "cycle-2".to_string(),
                        name: Some("Sprint 2".to_string()),
                        number: 2,
                        starts_at: None,
                        ends_at: None,
                    },
                ],
                issues: vec![
                    Issue {
                        id: "issue-1".to_string(),
                        identifier: "ENG-1".to_string(),
                        title: "Fix login bug".to_string(),
                        description: Some("Users can't log in".to_string()),
                        url: "https://linear.app/test/issue/ENG-1".to_string(),
                        state: WorkflowState {
                            id: "state-1".to_string(),
                            name: "Todo".to_string(),
                            color: "#ccc".to_string(),
                            state_type: "unstarted".to_string(),
                        },
                        assignee: None,
                        priority: 1,
                        project: None,
                        team: None,
                    },
                    Issue {
                        id: "issue-2".to_string(),
                        identifier: "ENG-2".to_string(),
                        title: "Add feature flag".to_string(),
                        description: None,
                        url: "https://linear.app/test/issue/ENG-2".to_string(),
                        state: WorkflowState {
                            id: "state-2".to_string(),
                            name: "In Progress".to_string(),
                            color: "#00f".to_string(),
                            state_type: "started".to_string(),
                        },
                        assignee: None,
                        priority: 2,
                        project: None,
                        team: None,
                    },
                    Issue {
                        id: "issue-3".to_string(),
                        identifier: "ENG-3".to_string(),
                        title: "Fix performance issue".to_string(),
                        description: None,
                        url: "https://linear.app/test/issue/ENG-3".to_string(),
                        state: WorkflowState {
                            id: "state-1".to_string(),
                            name: "Todo".to_string(),
                            color: "#ccc".to_string(),
                            state_type: "unstarted".to_string(),
                        },
                        assignee: None,
                        priority: 3,
                        project: None,
                        team: None,
                    },
                ],
                workflow_states: vec![
                    WorkflowState {
                        id: "state-1".to_string(),
                        name: "Todo".to_string(),
                        color: "#ccc".to_string(),
                        state_type: "unstarted".to_string(),
                    },
                    WorkflowState {
                        id: "state-2".to_string(),
                        name: "In Progress".to_string(),
                        color: "#00f".to_string(),
                        state_type: "started".to_string(),
                    },
                    WorkflowState {
                        id: "state-3".to_string(),
                        name: "Done".to_string(),
                        color: "#0f0".to_string(),
                        state_type: "completed".to_string(),
                    },
                ],
                viewer: Some(User {
                    id: "user-1".to_string(),
                    name: "Test User".to_string(),
                }),
            }
        }
    }

    impl LinearApi for MockClient {
        async fn fetch_teams(&self) -> Result<Vec<Team>, ApiError> {
            Ok(self.teams.clone())
        }

        async fn fetch_cycles(&self, _team_id: &str) -> Result<Vec<Cycle>, ApiError> {
            Ok(self.cycles.clone())
        }

        async fn fetch_issues(
            &self,
            _team_id: Option<&str>,
            _cycle_id: Option<&str>,
            _assignee_id: Option<&str>,
        ) -> Result<Vec<Issue>, ApiError> {
            Ok(self.issues.clone())
        }

        async fn fetch_backlog_issues(
            &self,
            _team_id: &str,
            _assignee_id: Option<&str>,
        ) -> Result<Vec<Issue>, ApiError> {
            Ok(self.issues.clone())
        }

        async fn fetch_workflow_states(
            &self,
            _team_id: &str,
        ) -> Result<Vec<WorkflowState>, ApiError> {
            Ok(self.workflow_states.clone())
        }

        async fn update_issue_status(
            &self,
            _issue_id: &str,
            state_id: &str,
        ) -> Result<Issue, ApiError> {
            let mut issue = self.issues[0].clone();
            issue.state = self
                .workflow_states
                .iter()
                .find(|s| s.id == state_id)
                .cloned()
                .unwrap_or(issue.state);
            Ok(issue)
        }

        async fn update_issue_description(
            &self,
            _issue_id: &str,
            description: &str,
        ) -> Result<Issue, ApiError> {
            let mut issue = self.issues[0].clone();
            issue.description = Some(description.to_string());
            Ok(issue)
        }

        async fn fetch_viewer(&self) -> Result<User, ApiError> {
            self.viewer
                .clone()
                .ok_or(ApiError::MissingData { context: "viewer" })
        }

        async fn fetch_issue_activity(
            &self,
            _issue_id: &str,
        ) -> Result<(Vec<Comment>, Vec<IssueHistory>), ApiError> {
            Ok((Vec::new(), Vec::new()))
        }

        async fn search_issues(&self, _query: &str) -> Result<Vec<Issue>, ApiError> {
            Ok(self.issues.clone())
        }
    }

    fn create_test_app() -> App<MockClient> {
        App::new(MockClient::new(), mock_config())
    }

    #[tokio::test]
    async fn init_loads_teams() {
        let mut app = create_test_app();
        app.init().await.unwrap();

        assert_eq!(app.teams.len(), 2);
        assert_eq!(app.teams[0].name, "Engineering");
        assert_eq!(app.teams[1].name, "Design");
    }

    #[tokio::test]
    async fn init_sets_current_team() {
        let mut app = create_test_app();
        app.init().await.unwrap();

        assert!(app.current_team.is_some());
        assert_eq!(app.current_team.as_ref().unwrap().name, "Engineering");
    }

    #[tokio::test]
    async fn init_loads_issues() {
        let mut app = create_test_app();
        app.init().await.unwrap();

        assert_eq!(app.issues.len(), 3);
        assert_eq!(app.filtered_issues.len(), 3);
    }

    #[tokio::test]
    async fn init_loads_cycles() {
        let mut app = create_test_app();
        app.init().await.unwrap();

        assert_eq!(app.cycles.len(), 2);
        assert!(app.current_cycle.is_some());
    }

    #[tokio::test]
    async fn init_loads_workflow_states() {
        let mut app = create_test_app();
        app.init().await.unwrap();

        assert_eq!(app.workflow_states.len(), 3);
    }

    #[test]
    fn next_issue_increments_index() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());
        app.selected_issue_index = 0;

        app.next_issue();
        assert_eq!(app.selected_issue_index, 1);

        app.next_issue();
        assert_eq!(app.selected_issue_index, 2);
    }

    #[test]
    fn next_issue_stops_at_last() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());
        app.selected_issue_index = 2;

        app.next_issue();
        assert_eq!(app.selected_issue_index, 2);
    }

    #[test]
    fn previous_issue_decrements_index() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());
        app.selected_issue_index = 2;

        app.previous_issue();
        assert_eq!(app.selected_issue_index, 1);

        app.previous_issue();
        assert_eq!(app.selected_issue_index, 0);
    }

    #[test]
    fn previous_issue_stops_at_first() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());
        app.selected_issue_index = 0;

        app.previous_issue();
        assert_eq!(app.selected_issue_index, 0);
    }

    #[test]
    fn first_issue_jumps_to_beginning() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());
        app.selected_issue_index = 2;

        app.first_issue();
        assert_eq!(app.selected_issue_index, 0);
    }

    #[test]
    fn last_issue_jumps_to_end() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());
        app.selected_issue_index = 0;

        app.last_issue();
        assert_eq!(app.selected_issue_index, 2);
    }

    #[test]
    fn enter_issue_filter_changes_mode() {
        let mut app = create_test_app();
        app.enter_issue_filter();

        assert_eq!(app.mode, Mode::IssueFilter);
        assert!(app.issue_filter.is_empty());
    }

    #[test]
    fn enter_team_select_changes_mode() {
        let mut app = create_test_app();
        app.enter_team_select();

        assert_eq!(app.mode, Mode::TeamSelect);
        assert!(app.team_filter.is_empty());
    }

    #[test]
    fn enter_cycle_select_changes_mode() {
        let mut app = create_test_app();
        app.enter_cycle_select();

        assert_eq!(app.mode, Mode::CycleSelect);
        assert!(app.cycle_filter.is_empty());
    }

    #[test]
    fn filter_input_appends_character() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.mode = Mode::IssueFilter;

        app.filter_input('a');
        assert_eq!(app.issue_filter, "a");

        app.filter_input('b');
        assert_eq!(app.issue_filter, "ab");
    }

    #[test]
    fn filter_backspace_removes_character() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.mode = Mode::IssueFilter;
        app.issue_filter = "abc".to_string();

        app.filter_backspace();
        assert_eq!(app.issue_filter, "ab");

        app.filter_backspace();
        assert_eq!(app.issue_filter, "a");
    }

    #[test]
    fn filter_backspace_handles_empty() {
        let mut app = create_test_app();
        app.mode = Mode::IssueFilter;
        app.issue_filter = String::new();

        app.filter_backspace();
        assert!(app.issue_filter.is_empty());
    }

    #[test]
    fn cancel_picker_returns_to_normal_mode() {
        let mut app = create_test_app();
        app.mode = Mode::IssueFilter;
        app.issue_filter = "test".to_string();

        app.cancel_picker();

        assert_eq!(app.mode, Mode::Normal);
        assert!(app.issue_filter.is_empty());
    }

    #[test]
    fn confirm_issue_filter_returns_to_normal_mode() {
        let mut app = create_test_app();
        app.mode = Mode::IssueFilter;
        app.issue_filter = "test".to_string();

        app.confirm_issue_filter();

        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.issue_filter, "test"); // Filter should be preserved
    }

    #[test]
    fn clear_error_removes_error() {
        let mut app = create_test_app();
        app.error = Some("Test error".to_string());

        app.clear_error();

        assert!(app.error.is_none());
    }

    #[test]
    fn clear_issue_filter_resets_filter() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.issue_filter = "test".to_string();
        app.selected_issue_index = 2;

        app.clear_issue_filter();

        assert!(app.issue_filter.is_empty());
        assert_eq!(app.selected_issue_index, 0);
    }

    #[test]
    fn filtering_issues_updates_filtered_list() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.mode = Mode::IssueFilter;

        app.filter_input('f');
        app.filter_input('i');
        app.filter_input('x');

        // Should match "Fix login bug" and "Fix performance issue"
        assert_eq!(app.filtered_issues.len(), 2);
    }

    #[test]
    fn selected_issue_returns_correct_issue() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());
        app.selected_issue_index = 1;

        let issue = app.selected_issue();
        assert!(issue.is_some());
        assert_eq!(issue.unwrap().identifier, "ENG-2");
    }

    #[test]
    fn selected_issue_returns_none_when_empty() {
        let mut app = create_test_app();
        app.filtered_issues = Vec::new();

        assert!(app.selected_issue().is_none());
    }

    #[tokio::test]
    async fn select_team_from_filter_changes_current_team() {
        let mut app = create_test_app();
        app.init().await.unwrap();
        app.enter_team_select();
        app.selected_team_index = 1; // Select "Design" team

        app.select_team_from_filter().await.unwrap();

        assert_eq!(app.current_team.as_ref().unwrap().name, "Design");
        assert_eq!(app.mode, Mode::Normal);
    }

    #[tokio::test]
    async fn select_cycle_from_filter_changes_current_cycle() {
        let mut app = create_test_app();
        app.init().await.unwrap();
        app.enter_cycle_select();
        app.selected_cycle_index = 1; // Select second cycle

        app.select_cycle_from_filter().await.unwrap();

        assert_eq!(
            app.current_cycle.as_ref().unwrap().name,
            Some("Sprint 2".to_string())
        );
        assert_eq!(app.mode, Mode::Normal);
    }

    #[tokio::test]
    async fn enter_detail_view_changes_mode() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());

        app.enter_detail_view().await.unwrap();

        assert_eq!(app.mode, Mode::DetailView);
        assert_eq!(app.detail_scroll_offset, 0);
    }

    #[tokio::test]
    async fn enter_detail_view_does_nothing_when_no_issues() {
        let mut app = create_test_app();
        app.filtered_issues = Vec::new();

        app.enter_detail_view().await.unwrap();

        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn exit_detail_view_returns_to_normal() {
        let mut app = create_test_app();
        app.mode = Mode::DetailView;

        app.exit_detail_view();

        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn scroll_detail_down_increments_offset() {
        let mut app = create_test_app();
        app.detail_content_height = 100;
        app.detail_viewport_height = 20;
        app.detail_scroll_offset = 0;

        app.scroll_detail_down();

        assert_eq!(app.detail_scroll_offset, 1);
    }

    #[test]
    fn scroll_detail_down_stops_at_max() {
        let mut app = create_test_app();
        app.detail_content_height = 25;
        app.detail_viewport_height = 20;
        app.detail_scroll_offset = 5; // Already at max (25 - 20 = 5)

        app.scroll_detail_down();

        assert_eq!(app.detail_scroll_offset, 5);
    }

    #[test]
    fn scroll_detail_up_decrements_offset() {
        let mut app = create_test_app();
        app.detail_scroll_offset = 5;

        app.scroll_detail_up();

        assert_eq!(app.detail_scroll_offset, 4);
    }

    #[test]
    fn scroll_detail_up_stops_at_zero() {
        let mut app = create_test_app();
        app.detail_scroll_offset = 0;

        app.scroll_detail_up();

        assert_eq!(app.detail_scroll_offset, 0);
    }

    #[test]
    fn scroll_detail_top_jumps_to_zero() {
        let mut app = create_test_app();
        app.detail_scroll_offset = 50;

        app.scroll_detail_top();

        assert_eq!(app.detail_scroll_offset, 0);
    }

    #[test]
    fn scroll_detail_bottom_jumps_to_max() {
        let mut app = create_test_app();
        app.detail_content_height = 100;
        app.detail_viewport_height = 20;
        app.detail_scroll_offset = 0;

        app.scroll_detail_bottom();

        assert_eq!(app.detail_scroll_offset, 80);
    }

    #[test]
    fn changing_issue_resets_scroll_offset() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());
        app.detail_scroll_offset = 10;

        app.next_issue();

        assert_eq!(app.detail_scroll_offset, 0);
    }

    #[test]
    fn find_current_cycle_returns_matching() {
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string();
        let next_week = (chrono::Utc::now() + chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string();

        let cycles = vec![
            Cycle {
                id: "cycle-old".to_string(),
                name: Some("Old Cycle".to_string()),
                number: 1,
                starts_at: Some("2020-01-01".to_string()),
                ends_at: Some("2020-01-14".to_string()),
            },
            Cycle {
                id: "cycle-current".to_string(),
                name: Some("Current Cycle".to_string()),
                number: 2,
                starts_at: Some(yesterday),
                ends_at: Some(next_week),
            },
        ];

        let current = find_current_cycle(&cycles);

        assert!(current.is_some());
        assert_eq!(current.unwrap().id, "cycle-current");
    }

    #[test]
    fn find_current_cycle_returns_none_when_no_match() {
        let cycles = vec![Cycle {
            id: "cycle-old".to_string(),
            name: Some("Old Cycle".to_string()),
            number: 1,
            starts_at: Some("2020-01-01".to_string()),
            ends_at: Some("2020-01-14".to_string()),
        }];

        let current = find_current_cycle(&cycles);

        assert!(current.is_none());
    }

    #[test]
    fn find_current_cycle_handles_iso8601_dates() {
        // Linear API returns ISO 8601 format dates
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%dT00:00:00.000Z")
            .to_string();
        let tomorrow = (chrono::Utc::now() + chrono::Duration::days(1))
            .format("%Y-%m-%dT00:00:00.000Z")
            .to_string();

        let cycles = vec![Cycle {
            id: "current-cycle".to_string(),
            name: Some("Current".to_string()),
            number: 1,
            starts_at: Some(yesterday),
            ends_at: Some(tomorrow),
        }];

        let current = find_current_cycle(&cycles);

        assert!(current.is_some());
        assert_eq!(current.unwrap().id, "current-cycle");
    }

    #[tokio::test]
    async fn init_fetches_viewer() {
        let mut app = create_test_app();

        app.init().await.unwrap();

        assert!(app.viewer.is_some());
        assert_eq!(app.viewer.as_ref().unwrap().name, "Test User");
    }

    #[tokio::test]
    async fn init_applies_my_issues_default() {
        use crate::config::AssigneeDefault;

        let mut config = mock_config();
        config.defaults.assignee = AssigneeDefault::Me;

        let mut app = App::new(MockClient::new(), config);
        app.init().await.unwrap();

        assert!(app.filter_my_issues);
    }

    #[tokio::test]
    async fn init_no_assignee_default_means_all_issues() {
        let config = mock_config(); // defaults to AssigneeDefault::None

        let mut app = App::new(MockClient::new(), config);
        app.init().await.unwrap();

        assert!(!app.filter_my_issues);
    }

    #[tokio::test]
    async fn init_applies_current_cycle_default() {
        use crate::config::CycleDefault;

        let mut config = mock_config();
        config.defaults.cycle = CycleDefault::Current;

        let mut client = MockClient::new();
        // Set cycle dates to include today - current cycle is NOT the first in the list
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let tomorrow = (chrono::Utc::now() + chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        client.cycles = vec![
            Cycle {
                id: "old-cycle".to_string(),
                name: Some("Old Cycle".to_string()),
                number: 1,
                starts_at: Some("2020-01-01".to_string()),
                ends_at: Some("2020-01-14".to_string()),
            },
            Cycle {
                id: "current-cycle".to_string(),
                name: Some("Current".to_string()),
                number: 2,
                starts_at: Some(yesterday),
                ends_at: Some(tomorrow),
            },
        ];

        let mut app = App::new(client, config);
        app.init().await.unwrap();

        assert!(app.current_cycle.is_some());
        assert_eq!(app.current_cycle.as_ref().unwrap().id, "current-cycle");
    }

    #[tokio::test]
    async fn toggle_my_issues_flips_filter() {
        let mut app = create_test_app();
        app.init().await.unwrap(); // This fetches the viewer

        assert!(!app.filter_my_issues);

        app.toggle_my_issues();
        assert!(app.filter_my_issues);

        app.toggle_my_issues();
        assert!(!app.filter_my_issues);
    }

    #[tokio::test]
    async fn init_applies_team_default_by_name() {
        let mut config = mock_config();
        config.defaults.team = Some("Design".to_string());

        let mut app = App::new(MockClient::new(), config);
        app.init().await.unwrap();

        // Should select Design team (2nd in list), not Engineering (1st)
        assert!(app.current_team.is_some());
        assert_eq!(app.current_team.as_ref().unwrap().name, "Design");
    }

    #[tokio::test]
    async fn init_applies_team_default_by_key() {
        let mut config = mock_config();
        config.defaults.team = Some("DES".to_string()); // Team key

        let mut app = App::new(MockClient::new(), config);
        app.init().await.unwrap();

        // Should match by key
        assert!(app.current_team.is_some());
        assert_eq!(app.current_team.as_ref().unwrap().name, "Design");
    }

    #[tokio::test]
    async fn init_applies_team_default_case_insensitive() {
        let mut config = mock_config();
        config.defaults.team = Some("design".to_string()); // lowercase

        let mut app = App::new(MockClient::new(), config);
        app.init().await.unwrap();

        // Should match case-insensitively
        assert!(app.current_team.is_some());
        assert_eq!(app.current_team.as_ref().unwrap().name, "Design");
    }

    #[tokio::test]
    async fn init_falls_back_to_first_team_when_default_not_found() {
        let mut config = mock_config();
        config.defaults.team = Some("NonexistentTeam".to_string());

        let mut app = App::new(MockClient::new(), config);
        app.init().await.unwrap();

        // Should fall back to first team
        assert!(app.current_team.is_some());
        assert_eq!(app.current_team.as_ref().unwrap().name, "Engineering");
    }

    #[tokio::test]
    async fn test_toggle_backlog_mode() {
        let mut app = create_test_app();
        app.init().await.unwrap();

        // Initially in cycle mode
        assert!(!app.backlog_mode);

        // Toggle to backlog
        app.backlog_mode = true;
        assert!(app.backlog_mode);

        // Toggle back to cycle
        app.backlog_mode = false;
        assert!(!app.backlog_mode);
    }

    #[test]
    fn merge_timeline_events_sorts_by_date() {
        use crate::api::{Comment, IssueHistory};

        let comments = vec![
            Comment {
                id: "c1".to_string(),
                body: "Second comment".to_string(),
                created_at: "2024-01-02T10:00:00Z".to_string(),
                user: Some(User {
                    id: "u1".to_string(),
                    name: "Alice".to_string(),
                }),
            },
            Comment {
                id: "c2".to_string(),
                body: "First comment".to_string(),
                created_at: "2024-01-01T10:00:00Z".to_string(),
                user: Some(User {
                    id: "u2".to_string(),
                    name: "Bob".to_string(),
                }),
            },
        ];

        let history = vec![];

        let events = super::merge_timeline_events(comments, history);

        assert_eq!(events.len(), 2);
        // Should be sorted oldest first
        assert!(matches!(&events[0], super::TimelineEvent::Comment { user, .. } if user == "Bob"));
        assert!(
            matches!(&events[1], super::TimelineEvent::Comment { user, .. } if user == "Alice")
        );
    }

    #[test]
    fn merge_timeline_events_includes_status_changes() {
        use crate::api::{Comment, IssueHistory};

        let comments = vec![];
        let history = vec![IssueHistory {
            id: "h1".to_string(),
            created_at: "2024-01-01T10:00:00Z".to_string(),
            actor: Some(User {
                id: "u1".to_string(),
                name: "Alice".to_string(),
            }),
            from_state: Some(WorkflowState {
                id: "s1".to_string(),
                name: "Todo".to_string(),
                color: "#ccc".to_string(),
                state_type: "unstarted".to_string(),
            }),
            to_state: Some(WorkflowState {
                id: "s2".to_string(),
                name: "In Progress".to_string(),
                color: "#00f".to_string(),
                state_type: "started".to_string(),
            }),
            from_assignee: None,
            to_assignee: None,
        }];

        let events = super::merge_timeline_events(comments, history);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            super::TimelineEvent::StatusChange { actor, from, to, .. }
            if actor == "Alice" && from == "Todo" && to == "In Progress"
        ));
    }

    #[test]
    fn merge_timeline_events_includes_assignee_changes() {
        use crate::api::{Comment, IssueHistory};

        let comments = vec![];
        let history = vec![IssueHistory {
            id: "h1".to_string(),
            created_at: "2024-01-01T10:00:00Z".to_string(),
            actor: Some(User {
                id: "u1".to_string(),
                name: "Alice".to_string(),
            }),
            from_state: None,
            to_state: None,
            from_assignee: None,
            to_assignee: Some(User {
                id: "u2".to_string(),
                name: "Bob".to_string(),
            }),
        }];

        let events = super::merge_timeline_events(comments, history);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            super::TimelineEvent::AssigneeChange { actor, from, to, .. }
            if actor == "Alice" && from.is_none() && to.as_deref() == Some("Bob")
        ));
    }

    #[test]
    fn merge_timeline_events_handles_empty_inputs() {
        use crate::api::{Comment, IssueHistory};

        let events =
            super::merge_timeline_events(Vec::<Comment>::new(), Vec::<IssueHistory>::new());

        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn exit_detail_view_keeps_timeline() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());
        app.enter_detail_view().await.unwrap();

        // Manually add some events to simulate loaded state
        app.timeline_events.push(super::TimelineEvent::Comment {
            user: "Test".to_string(),
            body: "Test".to_string(),
            created_at: "2024-01-01".to_string(),
        });

        app.exit_detail_view();

        assert_eq!(app.mode, Mode::Normal);
        // Timeline should persist when exiting detail view
        assert!(!app.timeline_events.is_empty());
    }

    #[test]
    fn changing_issue_clears_uncached_timeline() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());

        // Manually add timeline events (not in cache)
        app.timeline_events.push(super::TimelineEvent::Comment {
            user: "Test".to_string(),
            body: "Test".to_string(),
            created_at: "2024-01-01".to_string(),
        });

        app.next_issue();

        // Timeline should be cleared when selecting different issue (no cache for new issue)
        assert!(app.timeline_events.is_empty());
    }

    #[test]
    fn changing_issue_loads_from_cache() {
        let mut app = create_test_app();
        app.issues = app.client.issues.clone();
        app.filtered_issues = crate::fuzzy::filter_items(&app.issues, "", |i| i.title.clone());

        // Cache timeline for second issue
        let second_issue_id = app.filtered_issues[1].item.id.clone();
        app.timeline_cache.insert(
            second_issue_id,
            vec![super::TimelineEvent::Comment {
                user: "Cached".to_string(),
                body: "Cached comment".to_string(),
                created_at: "2024-01-01".to_string(),
            }],
        );

        // Navigate to second issue
        app.next_issue();

        // Timeline should load from cache
        assert_eq!(app.timeline_events.len(), 1);
        assert!(matches!(
            &app.timeline_events[0],
            super::TimelineEvent::Comment { user, .. } if user == "Cached"
        ));
    }

    #[test]
    fn enter_search_changes_mode() {
        let mut app = create_test_app();
        app.enter_search();
        assert_eq!(app.mode, Mode::Search);
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn cancel_search_returns_to_normal() {
        let mut app = create_test_app();
        app.mode = Mode::Search;
        app.search_query = "test".to_string();
        app.cancel_search();
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn search_input_appends_character() {
        let mut app = create_test_app();
        app.mode = Mode::Search;
        app.search_input('a');
        app.search_input('b');
        assert_eq!(app.search_query, "ab");
    }

    #[test]
    fn search_backspace_removes_character() {
        let mut app = create_test_app();
        app.search_query = "test".to_string();
        app.search_backspace();
        assert_eq!(app.search_query, "tes");
    }

    #[test]
    fn search_backspace_handles_empty() {
        let mut app = create_test_app();
        app.search_backspace();
        assert!(app.search_query.is_empty());
    }

    #[tokio::test]
    async fn execute_search_enters_results_mode() {
        let mut app = create_test_app();
        app.mode = Mode::Search;
        app.search_query = "test".to_string();
        app.execute_search().await.unwrap();
        assert_eq!(app.mode, Mode::SearchResults);
        assert!(app.in_search_context);
        assert_eq!(app.selected_search_index, 0);
    }

    #[tokio::test]
    async fn execute_search_empty_query_cancels() {
        let mut app = create_test_app();
        app.mode = Mode::Search;
        app.search_query.clear();
        app.execute_search().await.unwrap();
        assert_eq!(app.mode, Mode::Normal);
        assert!(!app.in_search_context);
    }

    #[test]
    fn exit_search_results_restores_normal() {
        let mut app = create_test_app();
        app.mode = Mode::SearchResults;
        app.in_search_context = true;
        app.search_results = vec![];
        app.exit_search_results();
        assert_eq!(app.mode, Mode::Normal);
        assert!(!app.in_search_context);
        assert!(app.search_results.is_empty());
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn next_search_result_increments() {
        let mut app = create_test_app();
        app.search_results = app.client.issues.clone();
        app.selected_search_index = 0;
        app.next_search_result();
        assert_eq!(app.selected_search_index, 1);
    }

    #[test]
    fn next_search_result_stops_at_end() {
        let mut app = create_test_app();
        app.search_results = app.client.issues.clone();
        app.selected_search_index = app.search_results.len().saturating_sub(1);
        app.next_search_result();
        assert_eq!(
            app.selected_search_index,
            app.search_results.len().saturating_sub(1)
        );
    }

    #[test]
    fn previous_search_result_decrements() {
        let mut app = create_test_app();
        app.search_results = app.client.issues.clone();
        app.selected_search_index = 1;
        app.previous_search_result();
        assert_eq!(app.selected_search_index, 0);
    }

    #[test]
    fn previous_search_result_stops_at_start() {
        let mut app = create_test_app();
        app.search_results = app.client.issues.clone();
        app.selected_search_index = 0;
        app.previous_search_result();
        assert_eq!(app.selected_search_index, 0);
    }

    #[tokio::test]
    async fn enter_detail_view_from_search_results() {
        let mut app = create_test_app();
        app.mode = Mode::SearchResults;
        app.in_search_context = true;
        app.search_results = app.client.issues.clone();
        app.selected_search_index = 0;
        app.enter_detail_view().await.unwrap();
        assert_eq!(app.mode, Mode::DetailView);
        assert!(app.in_search_context); // Should remain true
    }

    #[test]
    fn exit_detail_view_returns_to_search_results() {
        let mut app = create_test_app();
        app.mode = Mode::DetailView;
        app.in_search_context = true;
        app.exit_detail_view();
        assert_eq!(app.mode, Mode::SearchResults);
        assert!(app.in_search_context);
    }
}
