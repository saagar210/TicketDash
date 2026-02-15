use crate::errors::AppError;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::{interval, Duration};

pub struct SyncScheduler {
    interval_minutes: u64,
    is_running: Arc<TokioMutex<bool>>,
}

impl SyncScheduler {
    pub fn new(interval_minutes: u64) -> Self {
        SyncScheduler {
            interval_minutes,
            is_running: Arc::new(TokioMutex::new(false)),
        }
    }

    pub async fn start(
        &self,
        db_pool: Arc<std::sync::Mutex<rusqlite::Connection>>,
        jira_url: String,
        email: String,
        category_rules_json: String,
        app_handle: tauri::AppHandle,
    ) {
        if self.interval_minutes == 0 {
            log::info!("Background sync disabled (interval = 0)");
            return;
        }

        let mut is_running = self.is_running.lock().await;
        if *is_running {
            log::warn!("Background sync already running");
            return;
        }
        *is_running = true;
        drop(is_running);

        let interval_minutes = self.interval_minutes;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_minutes * 60));

            loop {
                ticker.tick().await;
                log::info!("Background sync triggered");

                // Emit event
                app_handle.emit("background-sync-started", ()).ok();

                // Perform sync (call the sync logic without the lock check)
                match perform_background_sync(&db_pool, &jira_url, &email, &category_rules_json)
                    .await
                {
                    Ok(count) => {
                        log::info!("Background sync completed: {} tickets", count);
                        app_handle.emit("background-sync-complete", count).ok();
                    }
                    Err(e) => {
                        log::error!("Background sync failed: {}", e);
                        app_handle.emit("background-sync-error", e.to_string()).ok();
                    }
                }
            }
        });
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.lock().await;
        *is_running = false;
        log::info!("Background sync stopped");
    }
}

async fn perform_background_sync(
    db_pool: &Arc<std::sync::Mutex<rusqlite::Connection>>,
    jira_url: &str,
    email: &str,
    category_rules_json: &str,
) -> Result<usize, AppError> {
    // Get token
    let token = crate::commands::settings::get_jira_token_internal().await?;

    // Parse category rules
    let rules_wrapper: crate::commands::sync::CategoryRulesWrapper =
        serde_json::from_str(category_rules_json)
            .map_err(|e| AppError::Config(format!("Failed to parse category rules: {}", e)))?;
    let category_rules = rules_wrapper.category_rules;

    // Create Jira client
    let client = crate::jira::JiraClient::new(jira_url, email, &token)?;

    // Get last sync timestamp
    let db_clone = db_pool.clone();
    let last_sync_ts = tauri::async_runtime::spawn_blocking(move || {
        let conn = db_clone
            .lock()
            .map_err(|_| AppError::Internal("Lock failed".to_string()))?;
        crate::db::get_sync_metadata(&conn, "last_sync_at")
    })
    .await
    .map_err(|_| AppError::Internal("Task join failed".to_string()))??;

    // Fetch tickets from Jira
    let mut tickets = client.fetch_tickets(last_sync_ts.as_deref()).await?;

    // Categorize tickets
    for ticket in &mut tickets {
        ticket.category = crate::services::categorize_ticket(ticket, &category_rules);
    }

    let synced_count = tickets.len();

    // Store in database
    let db_clone = db_pool.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db_clone
            .lock()
            .map_err(|_| AppError::Internal("Lock failed".to_string()))?;

        for ticket in &tickets {
            crate::db::upsert_ticket(&conn, ticket)?;
        }

        // Update last sync timestamp
        let now = chrono::Utc::now().to_rfc3339();
        crate::db::set_sync_metadata(&conn, "last_sync_at", &now)?;

        Ok::<(), AppError>(())
    })
    .await
    .map_err(|_| AppError::Internal("Task join failed".to_string()))??;

    Ok(synced_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_calculation() {
        let scheduler = SyncScheduler::new(30);
        assert_eq!(scheduler.interval_minutes, 30);
        // 30 minutes * 60 seconds = 1800 seconds
        assert_eq!(scheduler.interval_minutes * 60, 1800);
    }

    #[test]
    fn test_disabled_scheduler() {
        let scheduler = SyncScheduler::new(0);
        assert_eq!(scheduler.interval_minutes, 0);
    }
}
