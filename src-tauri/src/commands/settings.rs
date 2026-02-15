use crate::errors::AppError;
use crate::errors::JiraError;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const SERVICE_NAME: &str = "ticket-dashboard";
const KEYRING_USER: &str = "jira-api-token";

#[derive(Serialize, Deserialize, Clone)]
pub struct JiraSettings {
    pub jira_url: String,
    pub email: String,
}

#[tauri::command]
pub async fn store_jira_token(token: String) -> Result<(), AppError> {
    let entry =
        Entry::new(SERVICE_NAME, KEYRING_USER).map_err(|e| AppError::Keyring(e.to_string()))?;
    entry
        .set_password(&token)
        .map_err(|e| AppError::Keyring(e.to_string()))?;
    Ok(())
}

pub(crate) async fn get_jira_token_internal() -> Result<String, AppError> {
    let entry =
        Entry::new(SERVICE_NAME, KEYRING_USER).map_err(|e| AppError::Keyring(e.to_string()))?;
    match entry.get_password() {
        Ok(token) => Ok(token),
        Err(keyring::Error::NoEntry) => Err(JiraError::NotConfigured.into()),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_jira_token() -> Result<(), AppError> {
    let entry =
        Entry::new(SERVICE_NAME, KEYRING_USER).map_err(|e| AppError::Keyring(e.to_string()))?;
    entry
        .delete_credential()
        .map_err(|e| AppError::Keyring(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn verify_jira_connection(
    jira_url: String,
    email: String,
) -> Result<serde_json::Value, AppError> {
    let token = get_jira_token_internal().await?;
    let client = crate::jira::JiraClient::new(&jira_url, &email, &token)?;

    // Simple verification: try to fetch 1 ticket
    let _ = client.fetch_tickets(None).await?;

    let mut response = serde_json::Map::new();
    response.insert("email".to_string(), serde_json::Value::String(email));
    response.insert("connected".to_string(), serde_json::Value::Bool(true));

    Ok(serde_json::Value::Object(response))
}

#[tauri::command]
pub async fn save_jira_settings(
    app_handle: AppHandle,
    jira_url: String,
    email: String,
) -> Result<(), AppError> {
    let settings = JiraSettings { jira_url, email };

    let store = app_handle
        .store("settings.json")
        .map_err(|e| AppError::Config(format!("Failed to access store: {}", e)))?;

    let settings_value = serde_json::to_value(&settings)
        .map_err(|e| AppError::Config(format!("Failed to serialize settings: {}", e)))?;

    store.set("jira", settings_value);

    store
        .save()
        .map_err(|e| AppError::Config(format!("Failed to save settings: {}", e)))?;

    Ok(())
}

#[tauri::command]
pub async fn load_jira_settings(app_handle: AppHandle) -> Result<Option<JiraSettings>, AppError> {
    let store = app_handle
        .store("settings.json")
        .map_err(|e| AppError::Config(format!("Failed to access store: {}", e)))?;

    let settings_value = store.get("jira");

    if let Some(value) = settings_value {
        let settings: JiraSettings = serde_json::from_value(value.clone())
            .map_err(|e| AppError::Config(format!("Failed to parse settings: {}", e)))?;
        Ok(Some(settings))
    } else {
        Ok(None)
    }
}
