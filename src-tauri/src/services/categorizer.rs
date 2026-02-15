use crate::models::Ticket;
use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CategoryRule {
    pub name: String,
    pub conditions: Vec<RuleCondition>,
    #[serde(rename = "matchMode")]
    pub match_mode: MatchMode,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuleCondition {
    pub field: String,
    pub operator: String,
    pub value: String,
    #[serde(rename = "caseSensitive")]
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    All,
    Any,
}

pub fn categorize_ticket(ticket: &Ticket, rules: &[CategoryRule]) -> Option<String> {
    for rule in rules {
        let matched = match rule.match_mode {
            MatchMode::All => rule.conditions.iter().all(|c| eval_condition(ticket, c)),
            MatchMode::Any => rule.conditions.iter().any(|c| eval_condition(ticket, c)),
        };
        if matched {
            return Some(rule.name.clone());
        }
    }
    None
}

fn eval_condition(ticket: &Ticket, cond: &RuleCondition) -> bool {
    let field_value = match cond.field.as_str() {
        "summary" => &ticket.summary,
        "issue_type" => &ticket.issue_type,
        "project_key" => &ticket.project_key,
        "labels" => &ticket.labels,
        _ => return false,
    };

    match cond.operator.as_str() {
        "contains" => {
            if cond.case_sensitive {
                field_value.contains(&cond.value)
            } else {
                field_value
                    .to_lowercase()
                    .contains(&cond.value.to_lowercase())
            }
        }
        "equals" => {
            if cond.case_sensitive {
                field_value == &cond.value
            } else {
                field_value.to_lowercase() == cond.value.to_lowercase()
            }
        }
        "regex" => Regex::new(&cond.value)
            .map(|re| re.is_match(field_value))
            .unwrap_or(false),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_password_reset() {
        let ticket = Ticket {
            id: 1,
            jira_key: "TEST-1".to_string(),
            summary: "User forgot password, needs reset".to_string(),
            status: "Open".to_string(),
            priority: "Medium".to_string(),
            issue_type: "Task".to_string(),
            assignee: None,
            reporter: None,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            resolved_at: None,
            labels: String::new(),
            project_key: "TEST".to_string(),
            category: None,
        };

        let rules = vec![CategoryRule {
            name: "Password".to_string(),
            conditions: vec![RuleCondition {
                field: "summary".to_string(),
                operator: "contains".to_string(),
                value: "password".to_string(),
                case_sensitive: false,
            }],
            match_mode: MatchMode::Any,
        }];

        let result = categorize_ticket(&ticket, &rules);
        assert_eq!(result, Some("Password".to_string()));
    }

    #[test]
    fn test_categorize_no_match() {
        let ticket = Ticket {
            id: 1,
            jira_key: "TEST-1".to_string(),
            summary: "Weird thing".to_string(),
            status: "Open".to_string(),
            priority: "Medium".to_string(),
            issue_type: "Task".to_string(),
            assignee: None,
            reporter: None,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            resolved_at: None,
            labels: String::new(),
            project_key: "TEST".to_string(),
            category: None,
        };

        let rules = vec![CategoryRule {
            name: "VPN".to_string(),
            conditions: vec![RuleCondition {
                field: "summary".to_string(),
                operator: "contains".to_string(),
                value: "vpn".to_string(),
                case_sensitive: false,
            }],
            match_mode: MatchMode::Any,
        }];

        let result = categorize_ticket(&ticket, &rules);
        assert_eq!(result, None);
    }
}
