use super::SqliteStore;
use crate::domain::ChatSession;
use rusqlite::{params, Result as SqlResult};

impl SqliteStore {
    pub fn list_chat_sessions(&self) -> SqlResult<Vec<ChatSession>> {
        let mut statement = self.connection.prepare(
            "SELECT id,title,model_id,gateway_profile_id,created_at,updated_at
             FROM chat_sessions ORDER BY updated_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ChatSession {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    model_id: row.get(2)?,
                    gateway_profile_id: row.get(3)?,
                    messages: Vec::new(),
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                },
            ))
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            let (id, mut session) = row?;
            let mut messages = self.connection.prepare(
                "SELECT id,role,content,status,created_at FROM chat_messages WHERE session_id=?1 ORDER BY created_at ASC",
            )?;
            let rows = messages.query_map(params![id], |message| {
                Ok(crate::domain::ChatMessage {
                    id: message.get(0)?,
                    role: message.get(1)?,
                    content: message.get(2)?,
                    status: message.get(3)?,
                    created_at: message.get(4)?,
                })
            })?;
            session.messages = rows.collect::<SqlResult<Vec<_>>>()?;
            sessions.push(session);
        }
        Ok(sessions)
    }

    pub fn save_chat_session(&self, session: &ChatSession) -> SqlResult<()> {
        self.connection.execute(
            "INSERT INTO chat_sessions(id,title,model_id,gateway_profile_id,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title,model_id=excluded.model_id,
             gateway_profile_id=excluded.gateway_profile_id,updated_at=excluded.updated_at",
            params![
                session.id,
                session.title,
                session.model_id,
                session.gateway_profile_id,
                session.created_at,
                session.updated_at,
            ],
        )?;
        self.connection.execute(
            "DELETE FROM chat_messages WHERE session_id=?1",
            params![session.id],
        )?;
        for message in &session.messages {
            self.connection.execute(
                "INSERT INTO chat_messages(id,session_id,role,content,status,created_at)
                 VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    message.id,
                    session.id,
                    message.role,
                    message.content,
                    message.status,
                    message.created_at,
                ],
            )?;
        }
        Ok(())
    }

    pub fn delete_chat_session(&self, id: &str) -> SqlResult<()> {
        self.connection
            .execute("DELETE FROM chat_messages WHERE session_id=?1", params![id])?;
        self.connection
            .execute("DELETE FROM chat_sessions WHERE id=?1", params![id])?;
        Ok(())
    }
}
