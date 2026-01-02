use crate::api::error::ApiError;
use crate::api::types::*;
use crate::api::LinearApi;
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
    ) -> Result<T, ApiError> {
        let body = json!({
            "query": query,
            "variables": variables.unwrap_or(json!({}))
        });

        let response = self
            .client
            .post(LINEAR_API_URL)
            .header("User-Agent", "ortholinear/0.1.0")
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Http { status, body });
        }

        let result: GraphQLResponse<T> = response.json().await?;

        if let Some(errors) = result.errors {
            let messages = errors.into_iter().map(|e| e.message).collect();
            return Err(ApiError::GraphQL { messages });
        }

        result.data.ok_or(ApiError::MissingData {
            context: "response",
        })
    }
}

impl LinearApi for LinearClient {
    async fn fetch_teams(&self) -> Result<Vec<Team>, ApiError> {
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

    async fn fetch_cycles(&self, team_id: &str) -> Result<Vec<Cycle>, ApiError> {
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
    ) -> Result<Vec<Issue>, ApiError> {
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
    ) -> Result<Vec<Issue>, ApiError> {
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
        filter["cycle"] = json!({ "null": true });

        let variables = json!({ "filter": filter });
        let response: IssuesResponse = self.query(query, Some(variables)).await?;
        Ok(response.issues.nodes)
    }

    async fn fetch_workflow_states(&self, team_id: &str) -> Result<Vec<WorkflowState>, ApiError> {
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

    async fn update_issue_status(&self, issue_id: &str, state_id: &str) -> Result<Issue, ApiError> {
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
            return Err(ApiError::UpdateFailed);
        }

        response
            .issue_update
            .issue
            .ok_or(ApiError::MissingData { context: "issue" })
    }

    async fn update_issue_description(
        &self,
        issue_id: &str,
        description: &str,
    ) -> Result<Issue, ApiError> {
        let query = r#"
            mutation UpdateIssueDescription($issueId: String!, $description: String!) {
                issueUpdate(id: $issueId, input: { description: $description }) {
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
            "description": description
        });

        let response: IssueUpdateResponse = self.query(query, Some(variables)).await?;

        if !response.issue_update.success {
            return Err(ApiError::UpdateFailed);
        }

        response
            .issue_update
            .issue
            .ok_or(ApiError::MissingData { context: "issue" })
    }

    async fn fetch_viewer(&self) -> Result<User, ApiError> {
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

    async fn fetch_issue_activity(
        &self,
        issue_id: &str,
    ) -> Result<(Vec<Comment>, Vec<IssueHistory>), ApiError> {
        let query = r#"
            query IssueActivity($issueId: String!) {
                issue(id: $issueId) {
                    comments(first: 50) {
                        nodes {
                            id
                            body
                            createdAt
                            user {
                                id
                                name
                            }
                        }
                    }
                    history(first: 50) {
                        nodes {
                            id
                            createdAt
                            actor {
                                id
                                name
                            }
                            fromState {
                                id
                                name
                                color
                                type
                            }
                            toState {
                                id
                                name
                                color
                                type
                            }
                            fromAssignee {
                                id
                                name
                            }
                            toAssignee {
                                id
                                name
                            }
                        }
                    }
                }
            }
        "#;

        let variables = json!({ "issueId": issue_id });
        let response: IssueActivityResponse = self.query(query, Some(variables)).await?;

        let issue = response
            .issue
            .ok_or(ApiError::MissingData { context: "issue" })?;

        Ok((issue.comments.nodes, issue.history.nodes))
    }
}
