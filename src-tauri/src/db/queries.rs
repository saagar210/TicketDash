use crate::errors::{AppError, DbError};
use crate::models::{
    AggregationResult, AvgEntry, CountEntry, SummaryStats, Ticket, TimeSeriesEntry,
};
use crate::services::time_calc::business_hours_between;
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;

pub fn upsert_ticket(conn: &Connection, ticket: &Ticket) -> Result<(), AppError> {
    conn.execute(
        r#"
        INSERT INTO tickets (
            jira_key, summary, status, priority, issue_type, assignee, reporter,
            created_at, updated_at, resolved_at, labels, project_key, category
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(jira_key) DO UPDATE SET
            summary = excluded.summary,
            status = excluded.status,
            priority = excluded.priority,
            issue_type = excluded.issue_type,
            assignee = excluded.assignee,
            reporter = excluded.reporter,
            updated_at = excluded.updated_at,
            resolved_at = excluded.resolved_at,
            labels = excluded.labels,
            category = excluded.category
        "#,
        params![
            ticket.jira_key,
            ticket.summary,
            ticket.status,
            ticket.priority,
            ticket.issue_type,
            ticket.assignee,
            ticket.reporter,
            ticket.created_at,
            ticket.updated_at,
            ticket.resolved_at,
            ticket.labels,
            ticket.project_key,
            ticket.category,
        ],
    )
    .map_err(DbError::from)?;

    Ok(())
}

pub fn get_tickets(conn: &Connection) -> Result<Vec<Ticket>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, jira_key, summary, status, priority, issue_type, assignee, reporter, \
             created_at, updated_at, resolved_at, labels, project_key, category \
             FROM tickets ORDER BY created_at DESC",
        )
        .map_err(DbError::from)?;

    let tickets = stmt
        .query_map([], |row| {
            Ok(Ticket {
                id: row.get(0)?,
                jira_key: row.get(1)?,
                summary: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                issue_type: row.get(5)?,
                assignee: row.get(6)?,
                reporter: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                resolved_at: row.get(10)?,
                labels: row.get(11)?,
                project_key: row.get(12)?,
                category: row.get(13)?,
            })
        })
        .map_err(DbError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::from)?;

    Ok(tickets)
}

pub fn get_aggregations(conn: &Connection) -> Result<AggregationResult, AppError> {
    let tickets_by_status = get_count_by_field(conn, "status")?;
    let tickets_by_priority = get_count_by_field(conn, "priority")?;
    let tickets_by_category = get_count_by_field(conn, "category")?;
    let tickets_over_time = get_tickets_over_time(conn)?;
    let resolution_time_by_priority = get_resolution_time_by_priority(conn)?;
    let summary = get_summary_stats(conn)?;

    Ok(AggregationResult {
        tickets_by_status,
        tickets_by_priority,
        tickets_by_category,
        tickets_over_time,
        resolution_time_by_priority,
        summary,
    })
}

fn get_count_by_field(conn: &Connection, field: &str) -> Result<Vec<CountEntry>, AppError> {
    // Whitelist of allowed field names to prevent SQL injection
    let allowed_fields = ["status", "priority", "category"];
    if !allowed_fields.contains(&field) {
        return Err(AppError::Internal(format!("Invalid field name: {}", field)));
    }

    // Safe to use now that field is validated
    let query = format!(
        "SELECT COALESCE({}, 'Uncategorized') as name, COUNT(*) as count FROM tickets GROUP BY {} ORDER BY count DESC",
        field, field
    );

    let mut stmt = conn.prepare(&query).map_err(DbError::from)?;
    let entries = stmt
        .query_map([], |row| {
            Ok(CountEntry {
                name: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .map_err(DbError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::from)?;

    Ok(entries)
}

fn get_tickets_over_time(conn: &Connection) -> Result<Vec<TimeSeriesEntry>, AppError> {
    // Group created/resolved independently by month, then merge.
    // This avoids undercounting resolved issues that were created in a different month.
    let mut stmt = conn
        .prepare(
            r#"
        WITH created AS (
            SELECT strftime('%Y-%m', created_at) AS month, COUNT(*) AS created_count
            FROM tickets
            WHERE created_at IS NOT NULL
            GROUP BY month
        ),
        resolved AS (
            SELECT strftime('%Y-%m', resolved_at) AS month, COUNT(*) AS resolved_count
            FROM tickets
            WHERE resolved_at IS NOT NULL
            GROUP BY month
        ),
        months AS (
            SELECT month FROM created
            UNION
            SELECT month FROM resolved
        ),
        combined AS (
            SELECT
                months.month AS month,
                COALESCE(created.created_count, 0) AS created_count,
                COALESCE(resolved.resolved_count, 0) AS resolved_count
            FROM months
            LEFT JOIN created ON created.month = months.month
            LEFT JOIN resolved ON resolved.month = months.month
            ORDER BY months.month DESC
            LIMIT 12
        )
        SELECT month, created_count, resolved_count
        FROM combined
        ORDER BY month ASC
        "#,
        )
        .map_err(DbError::from)?;

    let entries = stmt
        .query_map([], |row| {
            Ok(TimeSeriesEntry {
                date: row.get(0)?,
                created: row.get(1)?,
                resolved: row.get(2)?,
            })
        })
        .map_err(DbError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::from)?;

    Ok(entries)
}

fn get_resolution_time_by_priority(conn: &Connection) -> Result<Vec<AvgEntry>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT priority, created_at, resolved_at FROM tickets WHERE resolved_at IS NOT NULL",
        )
        .map_err(DbError::from)?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(DbError::from)?;

    let mut durations_by_priority: HashMap<String, Vec<f64>> = HashMap::new();

    for row in rows {
        let (priority, created_at, resolved_at) = row.map_err(DbError::from)?;
        if let Some(hours) = calculate_business_resolution_hours(&created_at, &resolved_at) {
            durations_by_priority
                .entry(priority)
                .or_default()
                .push(hours);
        }
    }

    let mut entries = durations_by_priority
        .into_iter()
        .map(|(priority, mut durations)| {
            durations.sort_by(|a, b| a.total_cmp(b));
            AvgEntry {
                name: priority,
                avg_hours: average(&durations),
                median_hours: median(&durations),
                count: durations.len() as u32,
            }
        })
        .collect::<Vec<_>>();

    // Sort by priority order
    entries.sort_by_key(|e| match e.name.as_str() {
        "Critical" => 1,
        "High" => 2,
        "Medium" => 3,
        "Low" => 4,
        _ => 5,
    });

    Ok(entries)
}

fn get_summary_stats(conn: &Connection) -> Result<SummaryStats, AppError> {
    let total_tickets: u32 = conn
        .query_row("SELECT COUNT(*) FROM tickets", [], |row| row.get(0))
        .map_err(DbError::from)?;

    let open_tickets: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM tickets WHERE resolved_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(DbError::from)?;

    let resolved_tickets = total_tickets - open_tickets;

    let mut stmt = conn
        .prepare("SELECT created_at, resolved_at FROM tickets WHERE resolved_at IS NOT NULL")
        .map_err(DbError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(DbError::from)?;

    let mut resolution_hours = Vec::new();
    for row in rows {
        let (created_at, resolved_at) = row.map_err(DbError::from)?;
        if let Some(hours) = calculate_business_resolution_hours(&created_at, &resolved_at) {
            resolution_hours.push(hours);
        }
    }
    resolution_hours.sort_by(|a, b| a.total_cmp(b));

    let avg_resolution_hours = average(&resolution_hours);
    let median_resolution_hours = median(&resolution_hours);

    Ok(SummaryStats {
        total_tickets,
        open_tickets,
        resolved_tickets,
        avg_resolution_hours,
        median_resolution_hours,
    })
}

pub fn get_sync_metadata(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    let result: Option<String> = conn
        .query_row(
            "SELECT value FROM sync_metadata WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(DbError::from)?;
    Ok(result)
}

pub fn set_sync_metadata(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR REPLACE INTO sync_metadata (key, value) VALUES (?1, ?2)",
        params![key, value],
    )
    .map_err(DbError::from)?;
    Ok(())
}

fn calculate_business_resolution_hours(created_at: &str, resolved_at: &str) -> Option<f64> {
    let created = DateTime::parse_from_rfc3339(created_at).ok()?.naive_utc();
    let resolved = DateTime::parse_from_rfc3339(resolved_at).ok()?.naive_utc();
    business_hours_between(created, resolved, 9, 17).ok()
}

fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn median(sorted_values: &[f64]) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    let mid = sorted_values.len() / 2;
    if sorted_values.len().is_multiple_of(2) {
        (sorted_values[mid - 1] + sorted_values[mid]) / 2.0
    } else {
        sorted_values[mid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::initialize_database;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        initialize_database(&conn).expect("schema initialized");
        conn
    }

    fn sample_ticket(
        key: &str,
        priority: &str,
        created_at: &str,
        resolved_at: Option<&str>,
    ) -> Ticket {
        Ticket {
            id: 0,
            jira_key: key.to_string(),
            summary: format!("Summary {}", key),
            status: "Done".to_string(),
            priority: priority.to_string(),
            issue_type: "Task".to_string(),
            assignee: None,
            reporter: None,
            created_at: created_at.to_string(),
            updated_at: created_at.to_string(),
            resolved_at: resolved_at.map(|value| value.to_string()),
            labels: String::new(),
            project_key: "TEST".to_string(),
            category: None,
        }
    }

    #[test]
    fn tickets_over_time_counts_resolution_month_independently() {
        let conn = setup_db();

        upsert_ticket(
            &conn,
            &sample_ticket(
                "TEST-1",
                "High",
                "2025-01-15T09:00:00Z",
                Some("2025-02-03T10:00:00Z"),
            ),
        )
        .expect("insert TEST-1");
        upsert_ticket(
            &conn,
            &sample_ticket("TEST-2", "High", "2025-02-10T09:00:00Z", None),
        )
        .expect("insert TEST-2");
        upsert_ticket(
            &conn,
            &sample_ticket(
                "TEST-3",
                "Medium",
                "2025-02-11T09:00:00Z",
                Some("2025-02-12T11:00:00Z"),
            ),
        )
        .expect("insert TEST-3");

        let entries = get_tickets_over_time(&conn).expect("timeline aggregations");
        let by_month = entries
            .into_iter()
            .map(|entry| (entry.date, (entry.created, entry.resolved)))
            .collect::<HashMap<_, _>>();

        assert_eq!(by_month.get("2025-01"), Some(&(1, 0)));
        assert_eq!(by_month.get("2025-02"), Some(&(2, 2)));
    }

    #[test]
    fn resolution_stats_use_business_hours_and_even_median() {
        let conn = setup_db();

        upsert_ticket(
            &conn,
            &sample_ticket(
                "TEST-10",
                "High",
                "2025-01-06T09:00:00Z",
                Some("2025-01-06T17:00:00Z"),
            ),
        )
        .expect("insert TEST-10");
        upsert_ticket(
            &conn,
            &sample_ticket(
                "TEST-11",
                "High",
                "2025-01-07T09:00:00Z",
                Some("2025-01-07T13:00:00Z"),
            ),
        )
        .expect("insert TEST-11");
        upsert_ticket(
            &conn,
            &sample_ticket(
                "TEST-12",
                "Medium",
                "2025-01-10T16:00:00Z",
                Some("2025-01-13T10:00:00Z"),
            ),
        )
        .expect("insert TEST-12");

        let by_priority = get_resolution_time_by_priority(&conn).expect("priority stats");
        let high = by_priority
            .iter()
            .find(|entry| entry.name == "High")
            .expect("high priority entry");

        assert!((high.avg_hours - 6.0).abs() < 1e-9);
        assert!((high.median_hours - 6.0).abs() < 1e-9);
        assert_eq!(high.count, 2);

        let summary = get_summary_stats(&conn).expect("summary stats");
        assert_eq!(summary.total_tickets, 3);
        assert_eq!(summary.open_tickets, 0);
        assert_eq!(summary.resolved_tickets, 3);
        assert!((summary.avg_resolution_hours - (14.0 / 3.0)).abs() < 1e-9);
        assert!((summary.median_resolution_hours - 4.0).abs() < 1e-9);
    }
}
