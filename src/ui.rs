use crate::api::{Issue, LinearApi, TimelineEvent};
use crate::app::{App, Mode};
use crate::markdown::render_markdown;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap},
};

pub fn render<C: LinearApi>(frame: &mut Frame, app: &mut App<C>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);

    match app.mode {
        Mode::TeamSelect => render_team_picker(frame, app),
        Mode::CycleSelect => render_cycle_picker(frame, app),
        Mode::StatusSelect => render_status_picker(frame, app),
        Mode::IssueFilter => render_issue_filter(frame, app),
        Mode::Search => render_search_input(frame, app),
        Mode::Normal | Mode::DetailView | Mode::SearchResults => {}
    }

    if let Some(error) = &app.error {
        render_error_popup(frame, error);
    }

    if app.loading {
        render_loading(frame);
    }
}

fn render_header<C: LinearApi>(frame: &mut Frame, app: &mut App<C>, area: Rect) {
    let header_content = if app.in_search_context {
        let result_count = app.search_results.len();
        Line::from(vec![
            Span::styled(" [Search: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.search_query, Style::default().fg(Color::Cyan)),
            Span::styled("] ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{} result{}",
                    result_count,
                    if result_count == 1 { "" } else { "s" }
                ),
                Style::default().fg(Color::Yellow),
            ),
        ])
    } else {
        let team_name = app
            .current_team
            .as_ref()
            .map(|t| t.name.as_str())
            .unwrap_or("No team");

        let view_label = if app.backlog_mode {
            "Backlog".to_string()
        } else {
            app.current_cycle
                .as_ref()
                .map(|c| c.display_name())
                .unwrap_or_else(|| "No cycle".to_string())
        };

        let view_label_text = if app.backlog_mode {
            "View: "
        } else {
            "Cycle: "
        };

        let filter_indicator = if !app.issue_filter.is_empty() {
            format!(" [filter: {}]", app.issue_filter)
        } else {
            String::new()
        };

        Line::from(vec![
            Span::styled(" [Team: ", Style::default().fg(Color::DarkGray)),
            Span::styled(team_name, Style::default().fg(Color::Cyan)),
            Span::styled("] ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("[{}", view_label_text),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(view_label, Style::default().fg(Color::Cyan)),
            Span::styled("]", Style::default().fg(Color::DarkGray)),
            Span::styled(filter_indicator, Style::default().fg(Color::Magenta)),
        ])
    };

    let header = Paragraph::new(header_content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" ortholinear ")
            .title_alignment(Alignment::Right)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(header, area);
}

fn render_main<C: LinearApi>(frame: &mut Frame, app: &mut App<C>, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Issue list
            Constraint::Percentage(65), // Issue detail
        ])
        .split(area);

    render_issue_list(frame, app, chunks[0]);
    render_issue_detail(frame, app, chunks[1]);
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

fn render_issue_list<C: LinearApi>(frame: &mut Frame, app: &mut App<C>, area: Rect) {
    let (issues, selected_index): (Vec<&crate::api::Issue>, usize) = if app.in_search_context {
        (
            app.search_results.iter().collect(),
            app.selected_search_index,
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

            let team_prefix = if app.in_search_context {
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

fn render_issue_detail<C: LinearApi>(frame: &mut Frame, app: &mut App<C>, area: Rect) {
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
        let message = if app.mode == crate::app::Mode::DetailView {
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

fn render_footer<C: LinearApi>(frame: &mut Frame, app: &mut App<C>, area: Rect) {
    let help_text = match app.mode {
        Mode::Normal => {
            let mut spans = vec![
                Span::styled(" j/k", Style::default().fg(Color::Yellow)),
                Span::styled(": nav  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(": focus  ", Style::default().fg(Color::DarkGray)),
                Span::styled("/", Style::default().fg(Color::Yellow)),
                Span::styled(": filter  ", Style::default().fg(Color::DarkGray)),
                Span::styled("^f", Style::default().fg(Color::Yellow)),
                Span::styled(": search  ", Style::default().fg(Color::DarkGray)),
                Span::styled("s", Style::default().fg(Color::Yellow)),
                Span::styled(": status  ", Style::default().fg(Color::DarkGray)),
                Span::styled("t", Style::default().fg(Color::Yellow)),
                Span::styled(": team  ", Style::default().fg(Color::DarkGray)),
                Span::styled("c", Style::default().fg(Color::Yellow)),
                Span::styled(": cycle  ", Style::default().fg(Color::DarkGray)),
                Span::styled("B", Style::default().fg(Color::Yellow)),
                Span::styled(": backlog  ", Style::default().fg(Color::DarkGray)),
                Span::styled("C", Style::default().fg(Color::Yellow)),
                Span::styled(": current  ", Style::default().fg(Color::DarkGray)),
                Span::styled("m", Style::default().fg(Color::Yellow)),
                Span::styled(
                    if app.filter_my_issues {
                        ": all  "
                    } else {
                        ": mine  "
                    },
                    Style::default().fg(Color::DarkGray),
                ),
            ];
            if !app.issue_filter.is_empty() {
                spans.push(Span::styled("x", Style::default().fg(Color::Yellow)));
                spans.push(Span::styled(
                    ": clear  ",
                    Style::default().fg(Color::DarkGray),
                ));
            }
            spans.push(Span::styled("q", Style::default().fg(Color::Yellow)));
            spans.push(Span::styled(": quit", Style::default().fg(Color::DarkGray)));
            Line::from(spans)
        }
        Mode::DetailView => Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
            Span::styled(": scroll  ", Style::default().fg(Color::DarkGray)),
            Span::styled("g/G", Style::default().fg(Color::Yellow)),
            Span::styled(": top/bottom  ", Style::default().fg(Color::DarkGray)),
            Span::styled("e", Style::default().fg(Color::Yellow)),
            Span::styled(": edit  ", Style::default().fg(Color::DarkGray)),
            Span::styled("o", Style::default().fg(Color::Yellow)),
            Span::styled(": open  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc/Enter/q", Style::default().fg(Color::Yellow)),
            Span::styled(": back", Style::default().fg(Color::DarkGray)),
        ]),
        Mode::IssueFilter => Line::from(vec![
            Span::styled(" Type to filter  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab/↓↑", Style::default().fg(Color::Yellow)),
            Span::styled(": navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(": confirm  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(": cancel", Style::default().fg(Color::DarkGray)),
        ]),
        Mode::TeamSelect | Mode::CycleSelect | Mode::StatusSelect => Line::from(vec![
            Span::styled(" Type to filter  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab/↓↑", Style::default().fg(Color::Yellow)),
            Span::styled(": navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(": select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(": cancel", Style::default().fg(Color::DarkGray)),
        ]),
        Mode::Search => Line::from(vec![
            Span::styled(" Type to search  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(": search  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(": cancel", Style::default().fg(Color::DarkGray)),
        ]),
        Mode::SearchResults => Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
            Span::styled(": navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(": view  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(": exit search  ", Style::default().fg(Color::DarkGray)),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::styled(": quit", Style::default().fg(Color::DarkGray)),
        ]),
    };

    let footer = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_filter_input(frame: &mut Frame, filter: &str, area: Rect) {
    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::raw(filter),
        Span::styled("█", Style::default().fg(Color::Cyan)), // Cursor
    ]))
    .block(Block::default().borders(Borders::TOP));

    frame.render_widget(input, area);
}

fn render_issue_filter<C: LinearApi>(frame: &mut Frame, app: &mut App<C>) {
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

fn render_team_picker<C: LinearApi>(frame: &mut Frame, app: &mut App<C>) {
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

fn render_cycle_picker<C: LinearApi>(frame: &mut Frame, app: &mut App<C>) {
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
    let current_cycle_id = crate::app::find_current_cycle(&app.cycles).map(|c| c.id.clone());

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

fn render_status_picker<C: LinearApi>(frame: &mut Frame, app: &mut App<C>) {
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

fn render_search_input<C: LinearApi>(frame: &mut Frame, app: &mut App<C>) {
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

fn render_error_popup(frame: &mut Frame, error: &str) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let paragraph = Paragraph::new(vec![
        Line::from(error),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc to dismiss",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .wrap(Wrap { trim: false })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Error ")
            .border_style(Style::default().fg(Color::Red)),
    );

    frame.render_widget(paragraph, area);
}

fn render_loading(frame: &mut Frame) {
    let area = centered_rect(20, 10, frame.area());
    frame.render_widget(Clear, area);

    let paragraph = Paragraph::new("Loading...")
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(paragraph, area);
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
