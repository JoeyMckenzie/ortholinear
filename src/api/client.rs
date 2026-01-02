use crate::api::types::*;
use crate::api::LinearApi;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::json;

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

pub struct LinearClient {
    client: Client,
    api_key: String,
}

impl LinearClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    async fn query<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: Option<serde_json::Value>,
    ) -> Result<T> {
        let body = json!({
            "query": query,
            "variables": variables.unwrap_or(json!({}))
        });

        let response = self
            .client
            .post(LINEAR_API_URL)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Linear API")?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            bail!("Linear API error ({}): {}", status, text);
        }

        let result: GraphQLResponse<T> = response
            .json()
            .await
            .context("Failed to parse Linear API response")?;

        if let Some(errors) = result.errors {
            let messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
            bail!("GraphQL errors: {}", messages.join(", "));
        }

        result.data.context("No data in response")
    }
}

impl LinearApi for LinearClient {
    async fn fetch_teams(&self) -> Result<Vec<Team>> {
        let query = r#"
            query Teams {
                teams {
                    nodes {
                        id
                        name
                        key
                    }
                }
            }
        "#;

        let response: TeamsResponse = self.query(query, None).await?;
        Ok(response.teams.nodes)
    }

    async fn fetch_cycles(&self, team_id: &str) -> Result<Vec<Cycle>> {
        let query = r#"
            query Cycles($teamId: String!) {
                team(id: $teamId) {
                    cycles(first: 20, orderBy: createdAt) {
                        nodes {
                            id
                            name
                            number
                            startsAt
                            endsAt
                        }
                    }
                }
            }
        "#;

        let variables = json!({ "teamId": team_id });
        let response: TeamWithCyclesResponse = self.query(query, Some(variables)).await?;
        Ok(response.team.cycles.nodes)
    }

    async fn fetch_issues(
        &self,
        team_id: Option<&str>,
        cycle_id: Option<&str>,
        assignee_id: Option<&str>,
    ) -> Result<Vec<Issue>> {
        let query = r#"
            query Issues($filter: IssueFilter) {
                issues(first: 50, filter: $filter, orderBy: updatedAt) {
                    nodes {
                        id
                        identifier
                        title
                        description
                        url
                        state {
                            id
                            name
                            color
                            type
                        }
                        assignee {
                            id
                            name
                        }
                        priority
                        project {
                            id
                            name
                        }
                    }
                }
            }
        "#;

        let mut filter = json!({});
        if let Some(tid) = team_id {
            filter["team"] = json!({ "id": { "eq": tid } });
        }
        if let Some(cid) = cycle_id {
            filter["cycle"] = json!({ "id": { "eq": cid } });
        }
        if let Some(aid) = assignee_id {
            filter["assignee"] = json!({ "id": { "eq": aid } });
        }

        let variables = json!({ "filter": filter });
        let response: IssuesResponse = self.query(query, Some(variables)).await?;
        Ok(response.issues.nodes)
    }

    async fn fetch_backlog_issues(
        &self,
        team_id: &str,
        assignee_id: Option<&str>,
    ) -> Result<Vec<Issue>> {
        let query = r#"
            query BacklogIssues($filter: IssueFilter) {
                issues(first: 50, filter: $filter, orderBy: updatedAt) {
                    nodes {
                        id
                        identifier
                        title
                        description
                        url
                        state {
                            id
                            name
                            color
                            type
                        }
                        assignee {
                            id
                            name
                        }
                        priority
                        project {
                            id
                            name
                        }
                    }
                }
            }
        "#;

        let mut filter = json!({ "team": { "id": { "eq": team_id } } });
        if let Some(aid) = assignee_id {
            filter["assignee"] = json!({ "id": { "eq": aid } });
        }
        filter["state"] = json!({ "type": { "eq": "backlog" } });
        filter["cycle"] = json!({ "id": { "isNull": true } });

        let variables = json!({ "filter": filter });
        let response: IssuesResponse = self.query(query, Some(variables)).await?;
        Ok(response.issues.nodes)
    }

    async fn fetch_workflow_states(&self, team_id: &str) -> Result<Vec<WorkflowState>> {
        let query = r#"
            query WorkflowStates($teamId: String!) {
                workflowStates(filter: { team: { id: { eq: $teamId } } }) {
                    nodes {
                        id
                        name
                        color
                        type
                    }
                }
            }
        "#;

        let variables = json!({ "teamId": team_id });
        let response: WorkflowStatesResponse = self.query(query, Some(variables)).await?;
        Ok(response.workflow_states.nodes)
    }

    async fn update_issue_status(&self, issue_id: &str, state_id: &str) -> Result<Issue> {
        let query = r#"
            mutation UpdateIssueState($issueId: String!, $stateId: String!) {
                issueUpdate(id: $issueId, input: { stateId: $stateId }) {
                    success
                    issue {
                        id
                        identifier
                        title
                        description
                        url
                        state {
                            id
                            name
                            color
                            type
                        }
                        assignee {
                            id
                            name
                        }
                        priority
                        project {
                            id
                            name
                        }
                    }
                }
            }
        "#;

        let variables = json!({
            "issueId": issue_id,
            "stateId": state_id
        });

        let response: IssueUpdateResponse = self.query(query, Some(variables)).await?;

        if !response.issue_update.success {
            bail!("Failed to update issue status");
        }

        response.issue_update.issue.context("No issue in response")
    }

    async fn fetch_viewer(&self) -> Result<User> {
        let query = r#"
            query Viewer {
                viewer {
                    id
                    name
                }
            }
        "#;

        let response: ViewerResponse = self.query(query, None).await?;
        Ok(response.viewer)
    }
}
