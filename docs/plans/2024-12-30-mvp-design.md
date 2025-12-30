# Ortholinear MVP Design

A terminal UI for Linear, built with Rust and Ratatui.

## Goals

**MVP scope:**
- Issue triage: quickly review issues, update status
- Cycle management: view and manage issues within a cycle
- View-only + status updates (no issue creation or comments yet)

**Target users:** Power users who live in the terminal, especially Neovim users.

## Core Dependencies

- `ratatui` - TUI framework
- `crossterm` - Terminal backend (cross-platform)
- `tokio` - Async runtime for API calls
- `reqwest` - HTTP client for Linear's GraphQL API
- `serde` / `serde_json` - JSON serialization
- `dirs` - Cross-platform config directory detection
- `anyhow` - Error handling

## Project Structure

```
src/
├── main.rs           # Entry point, terminal setup/teardown
├── app.rs            # App state and update logic
├── ui/
│   ├── mod.rs
│   ├── layout.rs     # Main split-view layout
│   ├── issue_list.rs # Left panel rendering
│   └── issue_detail.rs # Right panel rendering
├── api/
│   ├── mod.rs
│   ├── client.rs     # Linear GraphQL client
│   └── types.rs      # Issue, Cycle, Team structs
├── config.rs         # Auth config loading
└── event.rs          # Input event handling
```

## Architecture

Using the Elm Architecture pattern:
- Single `App` struct holds all state
- Events flow through one `update()` function
- One `view()` function renders everything

```
Event → update(state, event) → new state → view(state) → UI
```

This can be refactored to actor/message-passing later as complexity grows.

## UI Layout

```
┌─────────────────────────────────────────────────────────────┐
│ [Team: Eng] [Cycle: Sprint 24]              ortholinear v0.1│
├────────────────────────┬────────────────────────────────────┤
│ Issues                 │ Issue Detail                       │
│ ──────────────────     │ ──────────────────────────────     │
│ > BUG-123 Fix login    │ Title: Fix login redirect bug      │
│   BUG-124 Update deps  │ Status: In Progress                │
│   BUG-125 Add tests    │ Assignee: Joey McKenzie            │
│   BUG-126 Refactor...  │ Priority: High                     │
│                        │ Project: Auth Improvements         │
│                        │ ──────────────────────────────     │
│                        │ Description:                       │
│                        │ When users log in with SSO, the    │
│                        │ redirect fails if...               │
│                        │                                    │
├────────────────────────┴────────────────────────────────────┤
│ j/k: navigate  s: set status  t: team  c: cycle  ?: help    │
└─────────────────────────────────────────────────────────────┘
```

## Keybindings

**Normal mode:**
- `j/k` or arrows - Move selection up/down
- `g` / `G` - Jump to top/bottom
- `Enter` or `l` - Focus detail panel
- `h` - Back to list focus
- `s` - Open status picker
- `t` - Open team picker
- `c` - Open cycle picker
- `r` - Refresh data
- `q` - Quit
- `?` - Show help

**StatusSelect mode:**
- `j/k` - Select status
- `Enter` - Confirm
- `Esc` - Cancel

## Authentication

1. Check `LINEAR_API_KEY` environment variable
2. Fall back to `~/.config/ortholinear/config.toml`:
   ```toml
   api_key = "lin_api_..."
   ```
3. Error with setup instructions if neither found

## Linear API Integration

Linear uses GraphQL. Key queries:

**Fetch teams:**
```graphql
query Teams {
  teams { nodes { id name key } }
}
```

**Fetch cycles:**
```graphql
query Cycles($teamId: String!) {
  team(id: $teamId) {
    cycles { nodes { id name number startsAt endsAt } }
  }
}
```

**Fetch issues:**
```graphql
query Issues($teamId: String, $cycleId: String) {
  issues(filter: { team: { id: { eq: $teamId } }, cycle: { id: { eq: $cycleId } } }) {
    nodes {
      id identifier title description
      state { id name color }
      assignee { id name }
      priority
      project { id name }
    }
  }
}
```

**Fetch workflow states:**
```graphql
query WorkflowStates($teamId: String!) {
  workflowStates(filter: { team: { id: { eq: $teamId } } }) {
    nodes { id name color type }
  }
}
```

**Update status:**
```graphql
mutation UpdateIssueState($issueId: String!, $stateId: String!) {
  issueUpdate(id: $issueId, input: { stateId: $stateId }) {
    issue { id state { id name } }
  }
}
```

## Application State

```rust
struct App {
    // Data
    teams: Vec<Team>,
    cycles: Vec<Cycle>,
    issues: Vec<Issue>,
    workflow_states: Vec<WorkflowState>,

    // UI state
    mode: Mode,
    selected_issue_index: usize,
    selected_status_index: usize,

    // Filters
    current_team: Option<Team>,
    current_cycle: Option<Cycle>,

    // Status
    loading: bool,
    error: Option<String>,
}

enum Mode {
    Normal,
    StatusSelect,
    TeamSelect,
    CycleSelect,
    Help,
}
```

## Event Loop

```rust
loop {
    terminal.draw(|frame| ui::render(frame, &app))?;

    if let Event::Key(key) = event::read()? {
        match app.mode {
            Mode::Normal => handle_normal_mode(&mut app, key),
            Mode::StatusSelect => handle_status_select(&mut app, key),
            // ...
        }
    }

    if app.should_quit { break; }
}
```

## Future Considerations

- Refactor to actor/message-passing for non-blocking API calls
- Add issue creation and comments
- Project views and cross-team visibility
- Caching for offline browsing
