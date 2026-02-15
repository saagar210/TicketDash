use crate::db::{get_sync_metadata, set_sync_metadata, upsert_ticket, DbPool};
use crate::errors::{AppError, DbError};
use crate::jira::JiraClient;
use crate::services::categorize_ticket;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

pub struct SyncLock(pub tokio::sync::Mutex<bool>);

#[derive(Deserialize)]
pub struct CategoryRulesWrapper {
    #[serde(rename = "categoryRules")]
    pub category_rules: Vec<crate::services::categorizer::CategoryRule>,
}

#[derive(Serialize, Clone)]
struct SyncProgress {
    phase: String,
    current: usize,
    total: Option<usize>,
}

#[tauri::command]
pub async fn trigger_sync(
    lock: tauri::State<'_, SyncLock>,
    db: tauri::State<'_, DbPool>,
    jira_url: String,
    email: String,
    category_rules_json: String,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, AppError> {
    let mut is_syncing = lock.0.lock().await;
    if *is_syncing {
        return Err(AppError::SyncAlreadyInProgress);
    }
    *is_syncing = true;
    drop(is_syncing);

    // Emit sync started
    app_handle.emit("sync-started", ()).ok();

    let result = perform_sync(db, jira_url, email, category_rules_json, app_handle.clone()).await;

    let mut is_syncing = lock.0.lock().await;
    *is_syncing = false;

    // Emit sync completed or error
    match &result {
        Ok(data) => {
            app_handle.emit("sync-complete", data.clone()).ok();
        }
        Err(e) => {
            app_handle.emit("sync-error", e.to_string()).ok();
        }
    }

    result
}

async fn perform_sync(
    db: tauri::State<'_, DbPool>,
    jira_url: String,
    email: String,
    category_rules_json: String,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, AppError> {
    // Get token
    let token = super::settings::get_jira_token_internal().await?;

    // Parse category rules
    let rules_wrapper: CategoryRulesWrapper = serde_json::from_str(&category_rules_json)
        .map_err(|e| AppError::Config(format!("Failed to parse category rules: {}", e)))?;
    let category_rules = rules_wrapper.category_rules;

    // Create Jira client
    let client = JiraClient::new(&jira_url, &email, &token)?;

    // Get last sync timestamp
    let db_clone = db.0.clone();
    let last_sync_ts = tauri::async_runtime::spawn_blocking(move || {
        let conn = db_clone.lock().map_err(|_| DbError::LockFailed)?;
        get_sync_metadata(&conn, "last_sync_at")
    })
    .await
    .map_err(|_| AppError::Internal("Task join failed".to_string()))??;

    // Emit fetching phase
    app_handle
        .emit(
            "sync-progress",
            SyncProgress {
                phase: "fetching".to_string(),
                current: 0,
                total: None,
            },
        )
        .ok();

    // Fetch tickets from Jira
    let mut tickets = client.fetch_tickets(last_sync_ts.as_deref()).await?;

    let total_count = tickets.len();

    // Emit categorizing phase
    app_handle
        .emit(
            "sync-progress",
            SyncProgress {
                phase: "categorizing".to_string(),
                current: 0,
                total: Some(total_count),
            },
        )
        .ok();

    // Categorize tickets
    for (idx, ticket) in tickets.iter_mut().enumerate() {
        ticket.category = categorize_ticket(ticket, &category_rules);

        // Emit progress every 10 tickets
        if idx % 10 == 0 {
            app_handle
                .emit(
                    "sync-progress",
                    SyncProgress {
                        phase: "categorizing".to_string(),
                        current: idx,
                        total: Some(total_count),
                    },
                )
                .ok();
        }
    }

    let synced_count = tickets.len();

    // Emit saving phase
    app_handle
        .emit(
            "sync-progress",
            SyncProgress {
                phase: "saving".to_string(),
                current: 0,
                total: Some(synced_count),
            },
        )
        .ok();

    // Store in database
    let db_clone = db.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db_clone.lock().map_err(|_| DbError::LockFailed)?;

        for ticket in &tickets {
            upsert_ticket(&conn, ticket)?;
        }

        // Update last sync timestamp
        let now = chrono::Utc::now().to_rfc3339();
        set_sync_metadata(&conn, "last_sync_at", &now)?;

        Ok::<(), AppError>(())
    })
    .await
    .map_err(|_| AppError::Internal("Task join failed".to_string()))??;

    let mut response = serde_json::Map::new();
    response.insert(
        "synced".to_string(),
        serde_json::Value::from(synced_count as u64),
    );
    response.insert("errors".to_string(), serde_json::Value::from(0_u64));
    response.insert(
        "last_sync".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );

    Ok(serde_json::Value::Object(response))
}

#[tauri::command]
pub async fn get_sync_status(db: tauri::State<'_, DbPool>) -> Result<serde_json::Value, AppError> {
    let db_clone = db.0.clone();
    let last_sync_at = tauri::async_runtime::spawn_blocking(move || {
        let conn = db_clone.lock().map_err(|_| DbError::LockFailed)?;
        get_sync_metadata(&conn, "last_sync_at")
    })
    .await
    .map_err(|_| AppError::Internal("Task join failed".to_string()))??;

    let mut response = serde_json::Map::new();
    response.insert("is_syncing".to_string(), serde_json::Value::Bool(false));
    response.insert(
        "last_sync_at".to_string(),
        last_sync_at
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    response.insert("last_error".to_string(), serde_json::Value::Null);

    Ok(serde_json::Value::Object(response))
}
