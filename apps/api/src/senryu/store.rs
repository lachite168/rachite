use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use sqlx::{FromRow, MySqlPool, QueryBuilder};
use uuid::Uuid;

#[derive(Clone)]
pub struct SenryuStore {
    db: MySqlPool,
}

#[derive(FromRow)]
pub struct Senryu {
    pub message_id: String,
    pub channel_id: String,
    pub verse: String,
    pub detected_at: NaiveDateTime,
}

pub struct SenryuCursor {
    detected_at: NaiveDateTime,
    message_id: String,
}

impl SenryuCursor {
    pub fn decode(value: &str) -> Option<Self> {
        let (timestamp, id) = value.split_once('.')?;
        let detected_at =
            DateTime::<Utc>::from_timestamp_micros(timestamp.parse().ok()?)?.naive_utc();
        if !(1000..=9999).contains(&detected_at.year()) {
            return None;
        }
        Some(Self {
            detected_at,
            message_id: id.parse::<Uuid>().ok()?.to_string(),
        })
    }

    pub fn encode(entry: &Senryu) -> String {
        format!(
            "{}.{}",
            entry.detected_at.and_utc().timestamp_micros(),
            entry.message_id
        )
    }
}

impl SenryuStore {
    pub fn new(db: MySqlPool) -> Self {
        Self { db }
    }

    pub async fn check_connection(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.db).await?;
        Ok(())
    }

    pub async fn is_enabled(&self, channel_id: Uuid) -> Result<bool, sqlx::Error> {
        let enabled =
            sqlx::query_scalar("SELECT senryu_enabled FROM channel_settings WHERE channel_id = ?")
                .bind(channel_id.to_string())
                .fetch_optional(&self.db)
                .await?;
        Ok(enabled.unwrap_or(true))
    }

    pub async fn set_enabled(&self, channel_id: Uuid, enabled: bool) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO channel_settings (channel_id, senryu_enabled, updated_at)
             VALUES (?, ?, UTC_TIMESTAMP(6))
             ON DUPLICATE KEY UPDATE senryu_enabled = VALUES(senryu_enabled), updated_at = VALUES(updated_at)",
        )
        .bind(channel_id.to_string())
        .bind(enabled)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub(crate) async fn is_mogamigawa_enabled(
        &self,
        channel_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let enabled = sqlx::query_scalar(
            "SELECT mogamigawa_enabled FROM channel_settings WHERE channel_id = ?",
        )
        .bind(channel_id.to_string())
        .fetch_optional(&self.db)
        .await?;
        Ok(enabled.unwrap_or(true))
    }

    pub(crate) async fn set_mogamigawa_enabled(
        &self,
        channel_id: Uuid,
        enabled: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO channel_settings (channel_id, senryu_enabled, mogamigawa_enabled, updated_at)
             VALUES (?, TRUE, ?, UTC_TIMESTAMP(6))
             ON DUPLICATE KEY UPDATE mogamigawa_enabled = VALUES(mogamigawa_enabled), updated_at = VALUES(updated_at)",
        )
        .bind(channel_id.to_string())
        .bind(enabled)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub(crate) async fn record_mogamigawa(&self, message_id: Uuid) -> Result<bool, sqlx::Error> {
        let inserted = sqlx::query("INSERT INTO mogamigawa_responses (message_id) VALUES (?)")
            .bind(message_id.to_string())
            .execute(&self.db)
            .await;
        match inserted {
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(false),
            result => result.map(|_| true),
        }
    }

    /// Returns whether this event added a new entry, so duplicates don't send another reply.
    pub async fn record(
        &self,
        message_id: Uuid,
        channel_id: Uuid,
        verse: &str,
    ) -> Result<bool, sqlx::Error> {
        let inserted = sqlx::query(
            "INSERT INTO senryu (message_id, channel_id, verse, detected_at)
             VALUES (?, ?, ?, UTC_TIMESTAMP(6))",
        )
        .bind(message_id.to_string())
        .bind(channel_id.to_string())
        .bind(verse)
        .execute(&self.db)
        .await;
        match inserted {
            Ok(_) => Ok(true),
            Err(error)
                if error
                    .as_database_error()
                    .is_some_and(|e| e.is_unique_violation()) =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    pub async fn revise(
        &self,
        message_id: Uuid,
        channel_id: Uuid,
        verse: Option<&str>,
        allow_new: bool,
    ) -> Result<(), sqlx::Error> {
        let Some(verse) = verse else {
            return self.remove(message_id).await;
        };

        if allow_new {
            sqlx::query(
                "INSERT INTO senryu (message_id, channel_id, verse, detected_at)
                 VALUES (?, ?, ?, UTC_TIMESTAMP(6))
                 ON DUPLICATE KEY UPDATE verse = VALUES(verse)",
            )
            .bind(message_id.to_string())
            .bind(channel_id.to_string())
            .bind(verse)
            .execute(&self.db)
            .await?;
        } else {
            sqlx::query("UPDATE senryu SET verse = ? WHERE message_id = ?")
                .bind(verse)
                .bind(message_id.to_string())
                .execute(&self.db)
                .await?;
        }

        Ok(())
    }

    pub async fn remove(&self, message_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM senryu WHERE message_id = ?")
            .bind(message_id.to_string())
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn list(
        &self,
        before: Option<SenryuCursor>,
        limit: u32,
    ) -> Result<Vec<Senryu>, sqlx::Error> {
        let mut query =
            QueryBuilder::new("SELECT message_id, channel_id, verse, detected_at FROM senryu");
        if let Some(cursor) = before {
            query
                .push(" WHERE detected_at < ")
                .push_bind(cursor.detected_at)
                .push(" OR (detected_at = ")
                .push_bind(cursor.detected_at)
                .push(" AND message_id < ")
                .push_bind(cursor.message_id)
                .push(")");
        }
        query
            .push(" ORDER BY detected_at DESC, message_id DESC LIMIT ")
            .push_bind(limit);
        query.build_query_as().fetch_all(&self.db).await
    }
}
