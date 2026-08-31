use crate::db::{Database, HistoryEntry};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_history(
    limit: Option<i64>,
    offset: Option<i64>,
    search: Option<String>,
    db: State<'_, Arc<Database>>,
) -> Result<Vec<HistoryEntry>, String> {
    let limit = limit.unwrap_or(100).clamp(1, 1000);
    let offset = offset.unwrap_or(0).max(0);
    let search_ref = search.as_deref();
    db.list_history(limit, offset, search_ref)
        .map_err(|e| format!("读取历史记录失败: {}", e))
}

#[tauri::command]
pub async fn delete_history(id: i64, db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.delete_history(id).map_err(|e| format!("删除历史记录失败: {}", e))
}

#[tauri::command]
pub async fn clear_history(db: State<'_, Arc<Database>>) -> Result<usize, String> {
    let n = db
        .clear_history()
        .map_err(|e| format!("清空历史记录失败: {}", e))?;
    tracing::info!("cleared all history ({} entries)", n);
    Ok(n)
}

#[tauri::command]
pub async fn get_history_count(
    search: Option<String>,
    db: State<'_, Arc<Database>>,
) -> Result<i64, String> {
    db.get_history_count(search.as_deref())
        .map_err(|e| format!("读取历史数量失败: {}", e))
}
