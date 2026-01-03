use crate::api::LinearApi;
use crate::app::{find_current_cycle, App};
use crate::ui::common::{centered_rect, render_filter_input};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn render_issue_filter<C: LinearApi>(frame: &mut Frame, app: &App<C>) {
    let area = centered_rect(50, 60, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Filter Issues ")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Filter input
            Constraint::Min(0),    // List
        ])
        .split(inner);

    render_filter_input(frame, &app.issue_filter, inner_chunks[0]);

    let items: Vec<ListItem> = app
        .filtered_issues
        .iter()
        .enumerate()
        .map(|(i, filtered)| {
            let issue = &filtered.item;
            let style = if i == app.selected_issue_index {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            let prefix = if i == app.selected_issue_index {
                "> "
            } else {
                "  "
            };
            ListItem::new(format!("{}{} {}", prefix, issue.identifier, issue.title)).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner_chunks[1]);
}

pub fn render_team_picker<C: LinearApi>(frame: &mut Frame, app: &App<C>) {
    let area = centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Select Team ")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Filter input
            Constraint::Min(0),    // List
        ])
        .split(inner);

    render_filter_input(frame, &app.team_filter, chunks[0]);

    let items: Vec<ListItem> = app
        .filtered_teams
        .iter()
        .enumerate()
        .map(|(i, filtered)| {
            let team = &filtered.item;
            let style = if i == app.selected_team_index {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            let prefix = if i == app.selected_team_index {
                "> "
            } else {
                "  "
            };
            ListItem::new(format!("{}{} ({})", prefix, team.name, team.key)).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);
}

pub fn render_cycle_picker<C: LinearApi>(frame: &mut Frame, app: &App<C>) {
    let area = centered_rect(60, 50, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Select Cycle ")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Filter input
            Constraint::Min(0),    // List
        ])
        .split(inner);

    render_filter_input(frame, &app.cycle_filter, chunks[0]);

    // Determine which cycle is current
    let current_cycle_id = find_current_cycle(&app.cycles).map(|c| c.id.clone());

    let items: Vec<ListItem> = app
        .filtered_cycles
        .iter()
        .enumerate()
        .map(|(i, filtered)| {
            let cycle = &filtered.item;
            let is_selected = i == app.selected_cycle_index;
            let is_current = current_cycle_id
                .as_ref()
                .map(|id| id == &cycle.id)
                .unwrap_or(false);

            let style = if is_selected {
                Style::default().fg(Color::Yellow).bold()
            } else if is_current {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            let prefix = if is_selected { "> " } else { "  " };

            let suffix = if is_current { " [current]" } else { "" };

            ListItem::new(format!(
                "{}{}{}",
                prefix,
                cycle.display_with_dates(),
                suffix
            ))
            .style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);
}

pub fn render_status_picker<C: LinearApi>(frame: &mut Frame, app: &App<C>) {
    let area = centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Set Status ")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Filter input
            Constraint::Min(0),    // List
        ])
        .split(inner);

    render_filter_input(frame, &app.status_filter, chunks[0]);

    let items: Vec<ListItem> = app
        .filtered_states
        .iter()
        .enumerate()
        .map(|(i, filtered)| {
            let state = &filtered.item;
            let style = if i == app.selected_status_index {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            let prefix = if i == app.selected_status_index {
                "> "
            } else {
                "  "
            };
            ListItem::new(format!("{}{}", prefix, state.name)).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);
}

pub fn render_search_input<C: LinearApi>(frame: &mut Frame, app: &App<C>) {
    let area = centered_rect(50, 20, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Search Issues ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::raw(&app.search_query),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ]));

    frame.render_widget(input, inner);
}
