use crate::api::{Issue, LinearApi, TimelineEvent};
use crate::app::{App, Mode};
use crate::markdown::render_markdown;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};

pub fn render_issue_detail<C: LinearApi>(frame: &mut Frame, app: &mut App<C>, area: Rect) {
    let is_focused = app.mode == Mode::DetailView;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title = if is_focused {
        " Details (focused) "
    } else {
        " Details "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::horizontal(1));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if let Some(issue) = app.current_issue() {
        let content = build_detail_content(issue, app);
        let content_height = content.len() as u16;

        // Update app state for scroll calculations
        app.detail_content_height = content_height;
        app.detail_viewport_height = inner_area.height;

        let paragraph = Paragraph::new(content)
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll_offset, 0));
        frame.render_widget(paragraph, inner_area);
    } else {
        let empty = Paragraph::new("No issue selected").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, inner_area);
    }
}

fn build_detail_content<C: LinearApi>(issue: &Issue, app: &App<C>) -> Vec<Line<'static>> {
    let priority_label = match issue.priority {
        0 => ("No priority", Color::DarkGray),
        1 => ("Urgent", Color::Red),
        2 => ("High", Color::Yellow),
        3 => ("Medium", Color::Blue),
        _ => ("Low", Color::DarkGray),
    };

    let status_color = match issue.state.state_type.as_str() {
        "backlog" => Color::DarkGray,
        "unstarted" => Color::Gray,
        "started" => Color::Yellow,
        "completed" => Color::Green,
        "canceled" => Color::Red,
        _ => Color::White,
    };

    let assignee = issue
        .assignee
        .as_ref()
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "Unassigned".to_string());

    let project = issue
        .project
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "None".to_string());

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Title: ", Style::default().fg(Color::DarkGray)),
            Span::styled(issue.title.clone(), Style::default().bold()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
            Span::styled(issue.state.name.clone(), Style::default().fg(status_color)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Assignee: ", Style::default().fg(Color::DarkGray)),
            Span::raw(assignee),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Priority: ", Style::default().fg(Color::DarkGray)),
            Span::styled(priority_label.0, Style::default().fg(priority_label.1)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Project: ", Style::default().fg(Color::DarkGray)),
            Span::raw(project),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "───────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Description:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    if let Some(description) = &issue.description {
        let markdown_lines = render_markdown(description);
        lines.extend(markdown_lines);
    } else {
        lines.push(Line::from(Span::styled(
            "No description",
            Style::default().fg(Color::DarkGray).italic(),
        )));
    }

    // Add Activity section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "───────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Activity:",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    if app.timeline_loading {
        lines.push(Line::from(Span::styled(
            "Loading activity...",
            Style::default().fg(Color::DarkGray).italic(),
        )));
    } else if app.timeline_events.is_empty() {
        // Show different message based on whether we're focused
        let message = if app.mode == Mode::DetailView {
            "No activity"
        } else {
            "Press Enter to load activity"
        };
        lines.push(Line::from(Span::styled(
            message,
            Style::default().fg(Color::DarkGray).italic(),
        )));
    } else {
        for event in &app.timeline_events {
            lines.extend(render_timeline_event(event));
            lines.push(Line::from(""));
        }
    }

    lines
}

fn render_timeline_event(event: &TimelineEvent) -> Vec<Line<'static>> {
    match event {
        TimelineEvent::Comment {
            user,
            body,
            created_at,
        } => {
            let mut lines = vec![Line::from(vec![
                Span::styled(" ", Style::default().fg(Color::White)),
                Span::styled(user.clone(), Style::default().bold()),
                Span::styled(" · ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format_timeline_date(created_at),
                    Style::default().fg(Color::DarkGray),
                ),
            ])];
            // Render comment body as markdown
            let body_lines = render_markdown(body);
            for line in body_lines {
                // Indent comment body
                let mut spans = vec![Span::raw("  ")];
                spans.extend(line.spans);
                lines.push(Line::from(spans));
            }
            lines
        }
        TimelineEvent::StatusChange {
            actor,
            from,
            to,
            created_at,
        } => {
            vec![
                Line::from(vec![
                    Span::styled(" ", Style::default().fg(Color::Yellow)),
                    Span::styled(actor.clone(), Style::default().bold()),
                    Span::styled(" · ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format_timeline_date(created_at),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(from.clone(), Style::default().fg(Color::DarkGray)),
                    Span::styled(" → ", Style::default().fg(Color::Yellow)),
                    Span::styled(to.clone(), Style::default().fg(Color::White)),
                ]),
            ]
        }
        TimelineEvent::AssigneeChange {
            actor,
            from,
            to,
            created_at,
        } => {
            let from_name = from.clone().unwrap_or_else(|| "Unassigned".to_string());
            let to_name = to.clone().unwrap_or_else(|| "Unassigned".to_string());
            vec![
                Line::from(vec![
                    Span::styled(" ", Style::default().fg(Color::Cyan)),
                    Span::styled(actor.clone(), Style::default().bold()),
                    Span::styled(" · ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format_timeline_date(created_at),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(from_name, Style::default().fg(Color::DarkGray)),
                    Span::styled(" → ", Style::default().fg(Color::Cyan)),
                    Span::styled(to_name, Style::default().fg(Color::White)),
                ]),
            ]
        }
    }
}

fn format_timeline_date(date_str: &str) -> String {
    use chrono::NaiveDateTime;

    // Try parsing ISO 8601 format
    if let Ok(dt) =
        NaiveDateTime::parse_from_str(&date_str.replace('Z', ""), "%Y-%m-%dT%H:%M:%S%.f")
    {
        return dt.format("%m-%d-%Y %H:%M").to_string();
    }

    // Try simple date format
    if date_str.len() >= 10 {
        let date_part = &date_str[..10];
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
            return date.format("%m-%d-%Y").to_string();
        }
    }

    date_str.to_string()
}
