use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub save_path: String,
    pub size_bytes: Option<i64>,
    pub downloaded_at: String, // ISO8601
    pub status: String,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).context("打开 SQLite 失败")?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT NOT NULL,
                save_path TEXT NOT NULL,
                size_bytes INTEGER,
                downloaded_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'completed'
            );
            CREATE INDEX IF NOT EXISTS idx_history_downloaded_at ON history(downloaded_at DESC);

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )
        .context("建表失败")?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn add_history(
        &self,
        url: &str,
        title: &str,
        save_path: &str,
        size_bytes: Option<i64>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO history (url, title, save_path, size_bytes, downloaded_at, status) VALUES (?, ?, ?, ?, ?, 'completed')",
            params![url, title, save_path, size_bytes, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 分页 + 模糊搜索 (url/title)
    /// - `limit` / `offset` 控制分页
    /// - `search` 非空时,做 LIKE '%search%' 匹配 url 或 title
    pub fn list_history(
        &self,
        limit: i64,
        offset: i64,
        search: Option<&str>,
    ) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let limit = limit.max(1);
        let offset = offset.max(0);

        let (sql, search_pattern): (&str, String) = match search {
            Some(s) if !s.trim().is_empty() => {
                let pat = format!("%{}%", s.trim());
                (
                    "SELECT id, url, title, save_path, size_bytes, downloaded_at, status
                     FROM history
                     WHERE url LIKE ?1 OR title LIKE ?1
                     ORDER BY id DESC
                     LIMIT ?2 OFFSET ?3",
                    pat,
                )
            }
            _ => (
                "SELECT id, url, title, save_path, size_bytes, downloaded_at, status
                 FROM history
                 ORDER BY id DESC
                 LIMIT ?1 OFFSET ?2",
                String::new(),
            ),
        };

        let mut stmt = conn.prepare(sql)?;
        let entries = if search_pattern.is_empty() {
            stmt.query_map(params![limit, offset], row_to_entry)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![search_pattern, limit, offset], row_to_entry)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(entries)
    }

    pub fn get_history_count(&self, search: Option<&str>) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = match search {
            Some(s) if !s.trim().is_empty() => {
                let pat = format!("%{}%", s.trim());
                conn.query_row(
                    "SELECT COUNT(*) FROM history WHERE url LIKE ?1 OR title LIKE ?1",
                    params![pat],
                    |row| row.get(0),
                )?
            }
            _ => conn.query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))?,
        };
        Ok(count)
    }

    pub fn delete_history(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute("DELETE FROM history WHERE id = ?", params![id])?;
        if n == 0 {
            // 不算错误, 仅记录
            tracing::warn!("delete_history: id={} 不存在", id);
        }
        Ok(())
    }

    /// 清空所有历史
    pub fn clear_history(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute("DELETE FROM history", [])?;
        Ok(n)
    }

    #[allow(dead_code)]
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        url: row.get(1)?,
        title: row.get(2)?,
        save_path: row.get(3)?,
        size_bytes: row.get(4)?,
        downloaded_at: row.get(5)?,
        status: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn fresh_db() -> Database {
        let dir = env::temp_dir().join(format!("vt_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        Database::open(&path).expect("open db")
    }

    #[test]
    fn test_add_and_list_history() {
        let db = fresh_db();
        let id1 = db.add_history("https://a.com/v1", "Title1", "/tmp/a.mp4", Some(1024)).unwrap();
        let id2 = db.add_history("https://b.com/v2", "Title2", "/tmp/b.mp4", None).unwrap();
        assert!(id1 > 0 && id2 > id1);

        let all = db.list_history(10, 0, None).unwrap();
        assert_eq!(all.len(), 2);
        // 倒序: 最新的 (id2) 在前
        assert_eq!(all[0].id, id2);
        assert_eq!(all[0].url, "https://b.com/v2");
        assert_eq!(all[1].id, id1);
    }

    #[test]
    fn test_list_history_pagination() {
        let db = fresh_db();
        for i in 0..5 {
            db.add_history(&format!("https://x.com/{}", i), &format!("t{}", i), "/p", None).unwrap();
        }
        let page1 = db.list_history(2, 0, None).unwrap();
        let page2 = db.list_history(2, 2, None).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_ne!(page1[0].id, page2[0].id);
    }

    #[test]
    fn test_list_history_search() {
        let db = fresh_db();
        db.add_history("https://youtube.com/abc", "Cool video", "/p", None).unwrap();
        db.add_history("https://bilibili.com/xyz", "Some video", "/p", None).unwrap();
        db.add_history("https://example.org", "Unrelated", "/p", None).unwrap();

        let r1 = db.list_history(10, 0, Some("youtube")).unwrap();
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].url, "https://youtube.com/abc");

        let r2 = db.list_history(10, 0, Some("video")).unwrap(); // 匹配 title
        assert_eq!(r2.len(), 2);

        let r3 = db.list_history(10, 0, Some("nope")).unwrap();
        assert_eq!(r3.len(), 0);
    }

    #[test]
    fn test_delete_history() {
        let db = fresh_db();
        let id = db.add_history("https://a.com", "t", "/p", None).unwrap();
        assert_eq!(db.list_history(10, 0, None).unwrap().len(), 1);
        db.delete_history(id).unwrap();
        assert_eq!(db.list_history(10, 0, None).unwrap().len(), 0);
        // 删不存在的 id 不报错
        db.delete_history(9999).unwrap();
    }

    #[test]
    fn test_clear_history() {
        let db = fresh_db();
        for i in 0..3 {
            db.add_history(&format!("u{}", i), &format!("t{}", i), "/p", None).unwrap();
        }
        assert_eq!(db.get_history_count(None).unwrap(), 3);
        let n = db.clear_history().unwrap();
        assert_eq!(n, 3);
        assert_eq!(db.get_history_count(None).unwrap(), 0);
    }

    #[test]
    fn test_get_history_count_with_search() {
        let db = fresh_db();
        db.add_history("https://yt.com/1", "foo", "/p", None).unwrap();
        db.add_history("https://yt.com/2", "bar", "/p", None).unwrap();
        db.add_history("https://other.com", "baz", "/p", None).unwrap();
        assert_eq!(db.get_history_count(None).unwrap(), 3);
        assert_eq!(db.get_history_count(Some("yt")).unwrap(), 2);
        assert_eq!(db.get_history_count(Some("foo")).unwrap(), 1);
    }

    #[test]
    fn test_setting_roundtrip() {
        let db = fresh_db();
        assert_eq!(db.get_setting("k").unwrap(), None);
        db.set_setting("k", "v1").unwrap();
        assert_eq!(db.get_setting("k").unwrap().as_deref(), Some("v1"));
        db.set_setting("k", "v2").unwrap();
        assert_eq!(db.get_setting("k").unwrap().as_deref(), Some("v2"));
    }
}
