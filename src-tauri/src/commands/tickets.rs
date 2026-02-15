use crate::db::{get_aggregations, get_tickets, DbPool};
use crate::errors::{AppError, DbError};
use crate::models::{AggregationResult, Ticket};

#[tauri::command]
pub async fn get_dashboard_data(
    db: tauri::State<'_, DbPool>,
) -> Result<AggregationResult, AppError> {
    let db_clone = db.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db_clone.lock().map_err(|_| DbError::LockFailed)?;
        get_aggregations(&conn)
    })
    .await
    .map_err(|_| AppError::Internal("Task join failed".to_string()))?
}

#[tauri::command]
pub async fn get_all_tickets(db: tauri::State<'_, DbPool>) -> Result<Vec<Ticket>, AppError> {
    let db_clone = db.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db_clone.lock().map_err(|_| DbError::LockFailed)?;
        get_tickets(&conn)
    })
    .await
    .map_err(|_| AppError::Internal("Task join failed".to_string()))?
}
