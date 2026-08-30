use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::chat::{ChatMessage, ChatRole, Conversation};

#[derive(Debug)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(err: rusqlite::Error) -> Self {
        Self(err.to_string())
    }
}

pub struct ChatStore {
    conn: Connection,
}

impl ChatStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| StoreError(err.to_string()))?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                repo_id TEXT NOT NULL,
                filename TEXT NOT NULL,
                title TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, id);
            ",
        )?;
        Ok(Self { conn })
    }

    pub fn open_or_create(
        &self,
        repo_id: &str,
        filename: &str,
    ) -> Result<Conversation, StoreError> {
        if let Some(existing) = self.latest_for(repo_id, filename)? {
            return Ok(existing);
        }
        let now = now_secs();
        let id = format!("cnv-{now:x}-{}", repo_id.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect::<String>());
        let title = repo_id.rsplit('/').next().unwrap_or(repo_id).to_string();
        self.conn.execute(
            "INSERT INTO conversations (id, repo_id, filename, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, repo_id, filename, title, now],
        )?;
        Ok(Conversation {
            id,
            repo_id: repo_id.into(),
            filename: filename.into(),
            title,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn latest_for(
        &self,
        repo_id: &str,
        filename: &str,
    ) -> Result<Option<Conversation>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, repo_id, filename, title, created_at, updated_at
             FROM conversations
             WHERE repo_id = ?1 AND filename = ?2
             ORDER BY updated_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![repo_id, filename])?;
        if let Some(row) = rows.next()? {
            Ok(Some(read_conversation(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn messages(&self, conversation_id: &str) -> Result<Vec<ChatMessage>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT role, content FROM messages WHERE conversation_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![conversation_id], |row| {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok((role, content))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (role, content) = row?;
            if let Some(role) = ChatRole::parse(&role) {
                out.push(ChatMessage { role, content });
            }
        }
        Ok(out)
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<Conversation>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, repo_id, filename, title, created_at, updated_at
             FROM conversations
             ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let mut rows = stmt.query(params![limit as i64])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(read_conversation(row)?);
        }
        Ok(out)
    }

    pub fn new_conversation(
        &self,
        repo_id: &str,
        filename: &str,
    ) -> Result<Conversation, StoreError> {
        let now = now_secs();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(now as u128);
        let id = format!(
            "cnv-{stamp:x}-{}",
            repo_id
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .take(8)
                .collect::<String>()
        );
        let title = repo_id.rsplit('/').next().unwrap_or(repo_id).to_string();
        self.conn.execute(
            "INSERT INTO conversations (id, repo_id, filename, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, repo_id, filename, title, now],
        )?;
        Ok(Conversation {
            id,
            repo_id: repo_id.into(),
            filename: filename.into(),
            title,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn delete_conversation(&self, conversation_id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?1",
            params![conversation_id],
        )?;
        self.conn.execute(
            "DELETE FROM conversations WHERE id = ?1",
            params![conversation_id],
        )?;
        Ok(())
    }

    pub fn clear_messages(&self, conversation_id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?1",
            params![conversation_id],
        )?;
        Ok(())
    }

    pub fn append(
        &self,
        conversation_id: &str,
        role: ChatRole,
        content: &str,
    ) -> Result<(), StoreError> {
        let now = now_secs();
        self.conn.execute(
            "INSERT INTO messages (conversation_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![conversation_id, role.as_str(), content, now],
        )?;
        self.conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            params![now, conversation_id],
        )?;
        Ok(())
    }
}

fn read_conversation(row: &rusqlite::Row<'_>) -> Result<Conversation, rusqlite::Error> {
    Ok(Conversation {
        id: row.get(0)?,
        repo_id: row.get(1)?,
        filename: row.get(2)?,
        title: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persist_and_reload_roundtrip() {
        let dir = std::env::temp_dir().join(format!("reco-store-{}", now_secs()));
        let _ = std::fs::create_dir_all(&dir);
        let store = ChatStore::open(&dir.join("reco.db")).unwrap();
        let conv = store
            .open_or_create("Qwen/Qwen2.5-7B-Instruct-GGUF", "q4.gguf")
            .unwrap();
        store
            .append(&conv.id, ChatRole::User, "hola")
            .unwrap();
        store
            .append(&conv.id, ChatRole::Assistant, "qué tal")
            .unwrap();
        let again = ChatStore::open(&dir.join("reco.db")).unwrap();
        let loaded = again
            .latest_for("Qwen/Qwen2.5-7B-Instruct-GGUF", "q4.gguf")
            .unwrap()
            .unwrap();
        let msgs = again.messages(&loaded.id).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "hola");
        assert_eq!(msgs[1].role, ChatRole::Assistant);
        let second = store
            .new_conversation("Qwen/Qwen2.5-7B-Instruct-GGUF", "q4.gguf")
            .unwrap();
        assert_ne!(second.id, conv.id);
        assert_eq!(store.list_recent(10).unwrap().len(), 2);
        store.delete_conversation(&second.id).unwrap();
        assert_eq!(store.list_recent(10).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
