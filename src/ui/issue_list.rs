use crate::api::LinearApi;
use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};

pub fn render_issue_list<C: LinearApi>(frame: &mut Frame, app: &App<C>, area: Rect) {
    let (issues, selected_index): (Vec<&crate::api::Issue>, usize) = if app.in_search_context {
        (
            app.search_results.iter().collect(),
            app.selected_search_index,
        )
    } else if app.in_view_context {
        (
            app.view_issues.iter().collect(),
            app.selected_issue_index,
        )
    } else {
        (
            app.filtered_issues.iter().map(|f| &f.item).collect(),
            app.selected_issue_index,
        )
    };

    let items: Vec<ListItem> = issues
        .iter()
        .enumerate()
        .map(|(i, issue)| {
            let style = if i == selected_index {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };

            let prefix = if i == selected_index { "> " } else { "  " };

            let team_prefix = if app.in_search_context || app.in_view_context {
                issue
                    .team
                    .as_ref()
                    .map(|t| format!("[{}] ", t.key))
                    .unwrap_or_default()
            } else {
                String::new()
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(team_prefix, Style::default().fg(Color::Magenta)),
                Span::styled(&issue.identifier, Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(&issue.title, style),
            ]))
        })
        .collect();

    let title = if app.in_search_context {
        " Search Results ".to_string()
    } else if app.in_view_context {
        " View Issues ".to_string()
    } else {
        build_issues_title(app)
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(list, area);
}

fn build_issues_title<C: LinearApi>(app: &App<C>) -> String {
    let mut parts = vec!["Issues".to_string()];

    if let Some(team) = &app.current_team {
        parts.push(team.name.clone());
    }

    if let Some(cycle) = &app.current_cycle {
        parts.push(cycle.display_name());
    }

    if app.filter_my_issues {
        parts.push("My Issues".to_string());
    }

    let filtered_count = app.filtered_issues.len();
    let total_count = app.issues.len();
    let count_str = if filtered_count == total_count {
        format!("{}", total_count)
    } else {
        format!("{}/{}", filtered_count, total_count)
    };

    if parts.len() > 1 {
        format!(
            " {} ({}) [{}] ",
            parts[0],
            parts[1..].join(" · "),
            count_str
        )
    } else {
        format!(" {} ({}) ", parts[0], count_str)
    }
}
