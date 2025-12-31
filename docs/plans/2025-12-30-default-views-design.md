# Default Views & Assignee Filter Design

## Overview

Add configurable default views so the TUI opens with contextually relevant issues - specifically, the current user's issues in the active cycle.

## Config Structure

Extend `config.toml` to support default view preferences:

```toml
api_key = "lin_api_..."

[defaults]
team = "Engineering"     # team name/key, or omit for first team
cycle = "current"        # "current", "none", or cycle number
assignee = "me"          # "me", "none", or specific name
```

All `[defaults]` fields are optional. If omitted:
- `team` → uses first team returned (existing behavior)
- `cycle` → `"none"` (no cycle filter)
- `assignee` → `"none"` (show all assignees)

Rust types:

```rust
pub struct Config {
    pub api_key: String,
    pub defaults: DefaultsConfig,
}

pub struct DefaultsConfig {
    pub team: Option<String>,
    pub cycle: CycleDefault,      // enum: Current, None, Number(i32)
    pub assignee: AssigneeDefault, // enum: Me, None, Name(String)
}
```

## API Changes

### New: Fetch Current User

```rust
fn fetch_viewer(&self) -> impl Future<Output = Result<User>> + Send;
```

GraphQL query:
```graphql
query Viewer {
  viewer {
    id
    name
    email
  }
}
```

### Extended: Fetch Issues with Assignee Filter

```rust
fn fetch_issues(
    &self,
    team_id: Option<&str>,
    cycle_id: Option<&str>,
    assignee_id: Option<&str>,  // new parameter
) -> impl Future<Output = Result<Vec<Issue>>> + Send;
```

GraphQL filter addition:
```rust
if let Some(aid) = assignee_id {
    filter["assignee"] = json!({ "id": { "eq": aid } });
}
```

### Current Cycle Detection

Helper function using existing cycle data:

```rust
fn find_current_cycle(cycles: &[Cycle]) -> Option<&Cycle> {
    let today = chrono::Utc::now().date_naive();
    cycles.iter().find(|c| {
        // parse startsAt/endsAt, check if today is within range
    })
}
```

Requires `chrono` dependency for date handling.

## App State Changes

New fields in `App` struct:

```rust
pub struct App<C: LinearApi> {
    // ... existing fields ...

    pub viewer: Option<User>,        // current user from viewer query
    pub filter_my_issues: bool,      // assignee filter toggle state
    pub applied_cycle_default: bool, // track if cycle default was applied
}
```

### Initialization Flow

1. Fetch teams (existing)
2. Fetch viewer → store in `self.viewer`
3. Find default team (by name from config, or first)
4. Fetch cycles for team (existing)
5. If `config.defaults.cycle == "current"`, find current cycle from date range
6. Fetch workflow states (existing)
7. Fetch issues with filters:
   - `team_id` from selected team
   - `cycle_id` from current cycle (if found)
   - `assignee_id` from viewer (if `config.defaults.assignee == "me"`)

### Toggle Implementation

```rust
pub fn toggle_my_issues(&mut self) {
    self.filter_my_issues = !self.filter_my_issues;
    // triggers reload of issues
}
```

Keybinding: `m` in Normal mode.

## UI Changes

### Header Indicators

Show active filters in header:

```
┌─ Issues (Engineering · Cycle 5 · My Issues) ─────────────────┐
```

When no current cycle found:
```
┌─ Issues (Engineering · No active cycle · My Issues) ──────────┐
```

### Footer Updates

Normal mode footer adds `m` toggle:
```
 j/k: nav  Enter: focus  /: filter  t: team  c: cycle  m: my issues  ...
```

Hint text changes based on state:
- When showing all: `m: my issues`
- When filtered to self: `m: all issues`

## Error Handling

- `fetch_viewer` fails → Continue without "my issues" feature. `m` key disabled.
- Date parsing fails on cycles → Treat as "no current cycle", fall back to unfiltered.
- Default team not found by name → Fall back to first team, show warning.
- Config `[defaults]` section missing → Use defaults (all filters off).

## Testing

### Config Tests
- `parses_defaults_section`
- `missing_defaults_uses_none`
- `cycle_current_parses`
- `cycle_number_parses`
- `assignee_me_parses`

### API Tests
- `fetch_viewer_returns_user`
- `fetch_issues_filters_by_assignee`

### App Tests
- `init_applies_current_cycle_default`
- `init_applies_my_issues_default`
- `toggle_my_issues_flips_filter`
- `find_current_cycle_returns_matching`
- `find_current_cycle_returns_none_when_outside_range`

### Mock Client Updates
- Add `viewer` field and `fetch_viewer` implementation
- Update `fetch_issues` to accept `assignee_id` parameter

## Dependencies

- Add `chrono` for date parsing and comparison
