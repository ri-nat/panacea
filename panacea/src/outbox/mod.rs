#[cfg(not(any(
    feature = "sqlx-runtime-async-std-native-tls",
    feature = "sqlx-runtime-async-std-rustls",
    feature = "sqlx-runtime-tokio-native-tls",
    feature = "sqlx-runtime-tokio-rustls",
)))]
compile_error!("you should enable one of the `sqlx-runtime-*` features of `panacea`");

#[cfg(not(any(feature = "mysql", feature = "postgres", feature = "sqlite")))]
compile_error!(
    "you should enable one of the `mysql`, `postgres` or `sqlite` features of `panacea`"
);

// Fail if multiple of `mysql`, `postgres` or `sqlite` features are enabled.
#[cfg(all(feature = "mysql", feature = "postgres"))]
compile_error!("you can't enable both `mysql` and `postgres` features of `panacea`");
#[cfg(all(feature = "mysql", feature = "sqlite"))]
compile_error!("you can't enable both `mysql` and `sqlite` features of `panacea`");
#[cfg(all(feature = "postgres", feature = "sqlite"))]
compile_error!("you can't enable both `postgres` and `sqlite` features of `panacea`");

use chrono::{DateTime, Utc};
use sqlx::{database::HasArguments, Encode, Executor, IntoArguments, Type};

use panacea_types::event::{self, AsBytesRef, Event, Headers};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("can't encode headers")]
    HeadersEncoding(serde_json::Error),
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EventRow {
    pub topic: String,
    pub key: Option<String>,
    pub payload: Vec<u8>,
    pub headers: String,
    pub created_at: DateTime<Utc>,
}

/// Constructs [`Event`] and stores it to outbox table.
///
/// Shorthand for [`event::new()`] + [`store_event()`].
///
/// # Errors
///
/// Will return an `Error` if there is any error occurs when storing an event to the database.
pub async fn store<'a, E, DB, K, T, P>(
    executor: E,
    topic: T,
    key: Option<K>,
    payload: P,
    headers: Option<Headers>,
) -> Result<(), Error>
where
    E: Executor<'a, Database = DB>,
    DB: sqlx::database::Database,
    <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
    K: ToString + Default,
    P: AsBytesRef,
    T: ToString,
    DateTime<Utc>: Type<DB> + Encode<'a, DB>,
    String: Type<DB> + Encode<'a, DB>,
    Vec<u8>: Type<DB> + Encode<'a, DB>,
{
    store_event(executor, event::new(&topic, key, &payload, headers)).await
}

/// Stores multiple outgoing events to outbox table.
///
/// # Errors
///
/// Will return an [`Error`] if there is any error occurs when storing an event to the database.
pub fn store_events<'a, E, DB>(_executor: &E, _events: &[Event]) -> Result<(), Error>
where
    E: Executor<'a, Database = DB>,
    DB: sqlx::database::Database,
    <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
    DateTime<Utc>: Type<DB> + Encode<'a, DB>,
    String: Type<DB> + Encode<'a, DB>,
    Vec<u8>: Type<DB> + Encode<'a, DB>,
{
    // TODO: Make it work

    Ok(())
}

/// Stores outgoing event to outbox table.
///
/// # Errors
///
/// Will return an [`Error`] if there is any error occurs when storing an event to the database.
pub async fn store_event<'a, E, DB>(executor: E, event: Event) -> Result<(), Error>
where
    E: Executor<'a, Database = DB>,
    DB: sqlx::database::Database,
    <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
    DateTime<Utc>: Type<DB> + Encode<'a, DB>,
    String: Type<DB> + Encode<'a, DB>,
    Vec<u8>: Type<DB> + Encode<'a, DB>,
{
    let headers = serde_json::to_string(&event.headers).map_err(Error::HeadersEncoding)?;

    #[cfg(feature = "mysql")]
    let query = r#"
        INSERT INTO panacea_outbox (
            topic, key, payload, headers, created_at
        ) VALUES (?, ?, ?, ?, NOW())
    "#;

    #[cfg(feature = "postgresql")]
    let query = r#"
        INSERT INTO panacea_outbox (
            topic, key, payload, headers, created_at
        ) VALUES ($1, $2, $3, $4, NOW())
    "#;

    #[cfg(feature = "sqlite")]
    let query = r#"
        INSERT INTO panacea_outbox (
            topic, key, payload, headers, created_at
        ) VALUES ($1, $2, $3, $4, datetime('now'))
    "#;

    sqlx::query(query)
        .bind(event.topic)
        .bind(event.key.unwrap_or_default())
        .bind(event.payload)
        .bind(headers)
        .execute(executor)
        .await?;

    Ok(())
}

impl TryFrom<EventRow> for Event {
    type Error = event::Error;

    fn try_from(value: EventRow) -> Result<Self, Self::Error> {
        let headers: Headers =
            serde_json::from_str(&value.headers).map_err(Self::Error::MalformedHeaders)?;

        Ok(Self {
            topic: value.topic,
            key: value.key,
            payload: value.payload,
            headers,
            created_at: value.created_at,
        })
    }
}
