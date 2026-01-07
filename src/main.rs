mod api;
mod app;
mod config;
mod error;
mod fuzzy;
mod markdown;
mod ui;

use api::{LinearApi, LinearClient};
use app::{App, Mode};
use config::Config;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use error::AppError;
use ratatui::prelude::*;
use std::io::{self, stdout};
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error:\n{}", e);
            std::process::exit(1);
        }
    };

    let client = LinearClient::new(config.api_key.clone());
    let mut app = App::new(client, config);

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = run(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    result
}

fn get_editor() -> Option<String> {
    std::env::var("EDITOR")
        .ok()
        .or_else(|| std::env::var("VISUAL").ok())
        .or_else(|| {
            // Check if common editors exist
            for editor in ["vim", "nvim", "nano", "vi"] {
                if Command::new("which")
                    .arg(editor)
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                {
                    return Some(editor.to_string());
                }
            }
            None
        })
}

async fn edit_description<C: LinearApi>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App<C>,
) -> Result<(), AppError> {
    let Some(issue) = app.selected_issue() else {
        return Ok(());
    };

    let issue_id = issue.identifier.clone();
    let original_description = app.get_description_for_edit();

    // Get editor
    let Some(editor) = get_editor() else {
        app.error = Some("No editor found. Set $EDITOR environment variable.".to_string());
        return Ok(());
    };

    // Create temp file
    let temp_path = std::env::temp_dir().join(format!("ortholinear-{}.md", issue_id));
    std::fs::write(&temp_path, &original_description)?;

    // Suspend TUI
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    // Run editor
    let status = Command::new(&editor).arg(&temp_path).status();

    // Resume TUI
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    terminal.clear()?;

    // Check editor exit status
    match status {
        Ok(exit_status) if exit_status.success() => {
            // Read edited content
            let new_description = std::fs::read_to_string(&temp_path).unwrap_or_default();

            // Clean up temp file
            let _ = std::fs::remove_file(&temp_path);

            // Only update if content changed
            if new_description != original_description {
                app.update_selected_issue_description(&new_description)
                    .await?;
            } else {
                // Clear any pending edit since user didn't change anything
                app.clear_pending_description_edit();
            }
        }
        Ok(_) => {
            // Editor exited with non-zero, treat as cancelled
            let _ = std::fs::remove_file(&temp_path);
        }
        Err(e) => {
            let _ = std::fs::remove_file(&temp_path);
            app.error = Some(format!("Failed to run editor '{}': {}", editor, e));
        }
    }

    Ok(())
}

async fn run<C: LinearApi>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App<C>,
) -> Result<(), AppError> {
    app.init().await?;

    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.mode {
                    Mode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('j') | KeyCode::Down => app.next_issue(),
                        KeyCode::Char('k') | KeyCode::Up => app.previous_issue(),
                        KeyCode::Char('g') => app.first_issue(),
                        KeyCode::Char('G') => app.last_issue(),
                        KeyCode::Enter => {
                            app.enter_detail_view().await?;
                        }
                        KeyCode::Char('/') => app.enter_issue_filter(),
                        KeyCode::Char('t') => app.enter_team_select(),
                        KeyCode::Char('c') => app.enter_cycle_select(),
                        KeyCode::Char('C') => {
                            app.jump_to_current_cycle().await?;
                        }
                        KeyCode::Char('B') => {
                            app.backlog_mode = !app.backlog_mode;
                            if app.backlog_mode {
                                app.load_backlog_issues().await?;
                            } else {
                                app.load_issues().await?;
                            }
                        }
                        KeyCode::Char('s') => app.enter_status_select(),
                        KeyCode::Char('m') => {
                            app.toggle_my_issues();
                            app.load_issues().await?;
                        }
                        KeyCode::Char('r') => {
                            app.refresh().await?;
                        }
                        KeyCode::Char('x') => app.clear_issue_filter(),
                        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.enter_search();
                        }
                        KeyCode::Char('v') => app.enter_view_select(),
                        KeyCode::Esc => {
                            if app.in_search_context {
                                app.exit_search_results();
                            } else if app.in_view_context {
                                app.exit_view_context();
                            } else {
                                app.clear_error();
                            }
                        }
                        _ => {}
                    },
                    Mode::DetailView => match key.code {
                        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                            app.exit_detail_view()
                        }
                        KeyCode::Char('o') => {
                            if let Err(e) = app.open_selected_issue() {
                                app.error = Some(format!("Failed to open URL: {}", e));
                            }
                        }
                        KeyCode::Char('e') => {
                            edit_description(terminal, app).await?;
                        }
                        KeyCode::Char('j') | KeyCode::Down => app.scroll_detail_down(),
                        KeyCode::Char('k') | KeyCode::Up => app.scroll_detail_up(),
                        KeyCode::Char('g') => app.scroll_detail_top(),
                        KeyCode::Char('G') => app.scroll_detail_bottom(),
                        _ => {}
                    },
                    Mode::IssueFilter => match key.code {
                        KeyCode::Enter => app.confirm_issue_filter(),
                        KeyCode::Esc => app.cancel_picker(),
                        KeyCode::Backspace => app.filter_backspace(),
                        KeyCode::Down | KeyCode::Tab => app.next_picker_item(),
                        KeyCode::Up | KeyCode::BackTab => app.previous_picker_item(),
                        KeyCode::Char(c) => app.filter_input(c),
                        _ => {}
                    },
                    Mode::TeamSelect => match key.code {
                        KeyCode::Enter => {
                            app.select_team_from_filter().await?;
                        }
                        KeyCode::Esc => app.cancel_picker(),
                        KeyCode::Backspace => app.filter_backspace(),
                        KeyCode::Down | KeyCode::Tab => app.next_picker_item(),
                        KeyCode::Up | KeyCode::BackTab => app.previous_picker_item(),
                        KeyCode::Char(c) => app.filter_input(c),
                        _ => {}
                    },
                    Mode::CycleSelect => match key.code {
                        KeyCode::Enter => {
                            app.select_cycle_from_filter().await?;
                        }
                        KeyCode::Esc => app.cancel_picker(),
                        KeyCode::Backspace => app.filter_backspace(),
                        KeyCode::Down | KeyCode::Tab => app.next_picker_item(),
                        KeyCode::Up | KeyCode::BackTab => app.previous_picker_item(),
                        KeyCode::Char(c) => app.filter_input(c),
                        _ => {}
                    },
                    Mode::StatusSelect => match key.code {
                        KeyCode::Enter => {
                            if let Some(state) = app
                                .filtered_states
                                .get(app.selected_status_index)
                                .map(|f| f.item.clone())
                            {
                                app.update_selected_issue_status(&state).await?;
                            }
                        }
                        KeyCode::Esc => app.cancel_picker(),
                        KeyCode::Backspace => app.filter_backspace(),
                        KeyCode::Down | KeyCode::Tab => app.next_picker_item(),
                        KeyCode::Up | KeyCode::BackTab => app.previous_picker_item(),
                        KeyCode::Char(c) => app.filter_input(c),
                        _ => {}
                    },
                    Mode::Search => match key.code {
                        KeyCode::Enter => {
                            app.execute_search().await?;
                        }
                        KeyCode::Esc => app.cancel_search(),
                        KeyCode::Backspace => app.search_backspace(),
                        KeyCode::Char(c) => app.search_input(c),
                        _ => {}
                    },
                    Mode::SearchResults => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.next_search_result(),
                        KeyCode::Char('k') | KeyCode::Up => app.previous_search_result(),
                        KeyCode::Enter => {
                            app.enter_detail_view().await?;
                        }
                        KeyCode::Char('r') => {
                            app.refresh().await?;
                        }
                        KeyCode::Esc => app.exit_search_results(),
                        KeyCode::Char('q') => break,
                        _ => {}
                    },
                    Mode::ViewSelect => match key.code {
                        KeyCode::Enter => {
                            app.select_view_from_filter().await?;
                        }
                        KeyCode::Esc => app.cancel_picker(),
                        KeyCode::Backspace => app.filter_backspace(),
                        KeyCode::Down | KeyCode::Tab => app.next_picker_item(),
                        KeyCode::Up | KeyCode::BackTab => app.previous_picker_item(),
                        KeyCode::Char(c) => app.filter_input(c),
                        _ => {}
                    },
                }
            }
        }
    }

    Ok(())
}
