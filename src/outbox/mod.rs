use chrono::{DateTime, Utc};
use sqlx::{database::HasArguments, Encode, Executor, IntoArguments, Type};

use crate::event::{Event, Headers};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    DatabaseError(#[from] sqlx::Error),
    #[error("can't encode headers")]
    HeadersEncodingError(serde_json::Error),
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EventRow {
    pub id: i64,
    pub key: Option<String>,
    pub payload: Vec<u8>,
    pub headers: String,
    pub created_at: DateTime<Utc>,
    pub is_delivered: bool,
}

/// Stores outgoing event to outbox table.
///
/// # Errors
///
/// Will return an `Error` if there is any error occurs when storing an event to the database.
pub async fn store<'a, E, DB>(executor: E, event: Event) -> Result<(), Error>
where
    E: Executor<'a, Database = DB>,
    DB: sqlx::database::Database,
    <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
    DateTime<Utc>: Type<DB> + Encode<'a, DB>,
    String: Type<DB> + Encode<'a, DB>,
    Vec<u8>: Type<DB> + Encode<'a, DB>,
{
    write_to_db(executor, event.key, event.payload, event.headers).await
}

// TODO: Make it work for multiple events
async fn write_to_db<'a, E, DB, K, P>(
    executor: E,
    key: Option<K>,
    payload: P,
    headers: Headers,
) -> Result<(), Error>
where
    E: Executor<'a, Database = DB>,
    DB: sqlx::database::Database,
    <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
    K: Type<DB> + Encode<'a, DB> + ToString + Default + std::marker::Send + 'a,
    P: Type<DB> + Encode<'a, DB> + std::marker::Sync + std::marker::Send + 'a,
    DateTime<Utc>: Type<DB> + Encode<'a, DB>,
    String: Type<DB> + Encode<'a, DB>,
{
    let headers = serde_json::to_string(&headers).map_err(Error::HeadersEncodingError)?;

    sqlx::query(
        r#"
            INSERT INTO panacea_outbox (
                key, payload, headers, created_at
            ) VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(key.unwrap_or_default())
    .bind(payload)
    .bind(headers)
    .bind(chrono::offset::Utc::now())
    .execute(executor)
    .await?;

    Ok(())
}
