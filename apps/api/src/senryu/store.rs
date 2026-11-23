use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use sqlx::{FromRow, MySql, MySqlPool, QueryBuilder};
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

#[derive(FromRow)]
pub(crate) struct Mogamigawa {
    pub message_id: String,
    pub channel_id: Option<String>,
    pub verse: Option<String>,
    pub detected_at: Option<NaiveDateTime>,
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
        encode_dated_cursor(entry.detected_at, &entry.message_id)
    }

    fn push_older_than(self, query: &mut QueryBuilder<MySql>) {
        query
            .push("(detected_at < ")
            .push_bind(self.detected_at)
            .push(" OR (detected_at = ")
            .push_bind(self.detected_at)
            .push(" AND message_id < ")
            .push_bind(self.message_id)
            .push("))");
    }
}

fn encode_dated_cursor(detected_at: NaiveDateTime, message_id: &str) -> String {
    format!("{}.{message_id}", detected_at.and_utc().timestamp_micros())
}

pub(crate) enum MogamigawaCursor {
    Dated(SenryuCursor),
    Legacy(Uuid),
}

impl MogamigawaCursor {
    pub(crate) fn decode(value: &str) -> Option<Self> {
        match value.strip_prefix("legacy.") {
            Some(id) => id.parse().ok().map(Self::Legacy),
            None => SenryuCursor::decode(value).map(Self::Dated),
        }
    }

    pub(crate) fn encode(entry: &Mogamigawa) -> String {
        match entry.detected_at {
            Some(date) => encode_dated_cursor(date, &entry.message_id),
            None => format!("legacy.{}", entry.message_id),
        }
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

    pub(crate) async fn record_mogamigawa(
        &self,
        message_id: Uuid,
        channel_id: Uuid,
        verse: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO mogamigawa_responses (message_id, channel_id, verse, detected_at, response_started)
             VALUES (?, ?, ?, UTC_TIMESTAMP(6), FALSE)
             ON DUPLICATE KEY UPDATE message_id = VALUES(message_id)",
        )
        .bind(message_id.to_string())
        .bind(channel_id.to_string())
        .bind(verse)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub(crate) async fn claim_mogamigawa_response(
        &self,
        message_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE mogamigawa_responses SET response_started = TRUE
             WHERE message_id = ? AND listed AND NOT response_started",
        )
        .bind(message_id.to_string())
        .execute(&self.db)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn revise_mogamigawa(
        &self,
        message_id: Uuid,
        verse: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE mogamigawa_responses SET verse = ?, listed = ?
             WHERE message_id = ? AND listed",
        )
        .bind(verse)
        .bind(verse.is_some())
        .bind(message_id.to_string())
        .execute(&self.db)
        .await?;
        Ok(())
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
            query.push(" WHERE ");
            cursor.push_older_than(&mut query);
        }
        query
            .push(" ORDER BY detected_at DESC, message_id DESC LIMIT ")
            .push_bind(limit);
        query.build_query_as().fetch_all(&self.db).await
    }

    pub(crate) async fn list_mogamigawa(
        &self,
        before: Option<MogamigawaCursor>,
        limit: u32,
    ) -> Result<Vec<Mogamigawa>, sqlx::Error> {
        let mut query = QueryBuilder::new(
            "SELECT message_id, channel_id, verse, detected_at FROM mogamigawa_responses WHERE listed",
        );
        match before {
            Some(MogamigawaCursor::Dated(cursor)) => {
                query.push(" AND (detected_at IS NULL OR ");
                cursor.push_older_than(&mut query);
                query.push(")");
            }
            Some(MogamigawaCursor::Legacy(id)) => {
                query
                    .push(" AND detected_at IS NULL AND message_id < ")
                    .push_bind(id.to_string());
            }
            None => {}
        }
        query
            .push(" ORDER BY detected_at DESC, message_id DESC LIMIT ")
            .push_bind(limit);
        query.build_query_as().fetch_all(&self.db).await
    }
}
