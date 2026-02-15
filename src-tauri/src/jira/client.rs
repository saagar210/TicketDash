use crate::errors::{AppError, JiraError};
use crate::jira::types::JiraSearchResponse;
use crate::models::Ticket;
use base64::Engine;
use chrono::DateTime;

pub struct JiraClient {
    base_url: String,
    auth_header: String,
    client: reqwest::Client,
}

impl JiraClient {
    pub fn new(jira_url: &str, email: &str, token: &str) -> Result<Self, AppError> {
        let base_url = format!("{}/rest/api/3", jira_url.trim_end_matches('/'));
        let auth_header = Self::create_auth_header(email, token);
        let client = reqwest::Client::new();

        Ok(JiraClient {
            base_url,
            auth_header,
            client,
        })
    }

    fn create_auth_header(email: &str, token: &str) -> String {
        let credentials = format!("{}:{}", email, token);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
        format!("Basic {}", encoded)
    }

    pub async fn fetch_tickets(&self, last_sync_ts: Option<&str>) -> Result<Vec<Ticket>, AppError> {
        let mut all_tickets = Vec::new();
        let mut next_page_token: Option<String> = None;
        let jql = Self::build_jql(last_sync_ts);

        loop {
            let response = self.search_jql(&jql, next_page_token.as_deref()).await?;

            for issue in response.issues {
                let ticket = Self::convert_issue_to_ticket(issue);
                all_tickets.push(ticket);
            }

            if response.next_page_token.is_none() {
                break;
            }
            next_page_token = response.next_page_token;
        }

        Ok(all_tickets)
    }

    fn build_jql(last_sync_ts: Option<&str>) -> String {
        if let Some(ts) = last_sync_ts {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(ts) {
                let normalized = parsed.to_rfc3339();
                return format!(
                    "assignee = currentUser() AND updated >= \"{}\" ORDER BY updated ASC",
                    normalized
                );
            }

            log::warn!(
                "Invalid last_sync_at value '{}'; falling back to full sync query",
                ts
            );
        }

        "assignee = currentUser() ORDER BY created DESC".to_string()
    }

    async fn search_jql(
        &self,
        jql: &str,
        next_page_token: Option<&str>,
    ) -> Result<JiraSearchResponse, AppError> {
        let mut body = serde_json::Map::new();
        body.insert(
            "jql".to_string(),
            serde_json::Value::String(jql.to_string()),
        );
        body.insert("maxResults".to_string(), serde_json::Value::from(100_u64));
        body.insert(
            "fields".to_string(),
            serde_json::Value::Array(vec![
                serde_json::Value::String("summary".to_string()),
                serde_json::Value::String("status".to_string()),
                serde_json::Value::String("priority".to_string()),
                serde_json::Value::String("issuetype".to_string()),
                serde_json::Value::String("assignee".to_string()),
                serde_json::Value::String("reporter".to_string()),
                serde_json::Value::String("created".to_string()),
                serde_json::Value::String("updated".to_string()),
                serde_json::Value::String("resolutiondate".to_string()),
                serde_json::Value::String("labels".to_string()),
                serde_json::Value::String("project".to_string()),
            ]),
        );

        if let Some(token) = next_page_token {
            body.insert(
                "nextPageToken".to_string(),
                serde_json::Value::String(token.to_string()),
            );
        }

        let url = format!("{}/search/jql", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&serde_json::Value::Object(body))
            .send()
            .await
            .map_err(JiraError::from)?;

        let status = response.status();

        if status.is_success() {
            let search_response: JiraSearchResponse = response
                .json()
                .await
                .map_err(|e| JiraError::ParseError(e.to_string()))?;
            Ok(search_response)
        } else if status.as_u16() == 401 {
            Err(JiraError::Unauthorized.into())
        } else if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or_else(|| {
                    log::warn!("Rate limited but Retry-After header missing or invalid, defaulting to 60 seconds");
                    60
                });

            // Cap retry at 5 minutes to prevent unreasonable waits
            let retry_after = retry_after.min(300);

            Err(JiraError::RateLimited {
                retry_after_secs: retry_after,
            }
            .into())
        } else {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            Err(JiraError::ApiError {
                status: status.as_u16(),
                body,
            }
            .into())
        }
    }

    fn convert_issue_to_ticket(issue: crate::jira::types::JiraIssue) -> Ticket {
        Ticket {
            id: 0, // Will be set by database
            jira_key: issue.key,
            summary: issue.fields.summary,
            status: issue.fields.status.name,
            priority: issue.fields.priority.name,
            issue_type: issue.fields.issuetype.name,
            assignee: issue.fields.assignee.map(|a| a.display_name),
            reporter: issue.fields.reporter.map(|r| r.display_name),
            created_at: issue.fields.created,
            updated_at: issue.fields.updated,
            resolved_at: issue.fields.resolutiondate,
            labels: issue.fields.labels.join(","),
            project_key: issue.fields.project.key,
            category: None, // Will be set by categorizer
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JiraClient;

    #[test]
    fn build_jql_uses_incremental_query_for_valid_rfc3339() {
        let jql = JiraClient::build_jql(Some("2025-01-01T00:00:00Z"));
        assert_eq!(
            jql,
            "assignee = currentUser() AND updated >= \"2025-01-01T00:00:00+00:00\" ORDER BY updated ASC"
        );
    }

    #[test]
    fn build_jql_falls_back_to_full_query_for_invalid_timestamp() {
        let jql = JiraClient::build_jql(Some("not-a-timestamp"));
        assert_eq!(jql, "assignee = currentUser() ORDER BY created DESC");
    }
}
