use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: i64,
    pub jira_key: String,
    pub summary: String,
    pub status: String,
    pub priority: String,
    pub issue_type: String,
    pub assignee: Option<String>,
    pub reporter: Option<String>,
    pub created_at: String,          // ISO 8601
    pub updated_at: String,          // ISO 8601
    pub resolved_at: Option<String>, // ISO 8601
    pub labels: String,              // comma-separated
    pub project_key: String,
    pub category: Option<String>, // computed locally
}
