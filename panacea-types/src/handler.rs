use crate::event::Event;
use state::Container;

#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type HandlingResult = Result<Option<Vec<Event>>, Error>;
pub type MaybeHandlers = Option<Vec<Box<dyn Handler + Send + Sync>>>;

#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
#[async_trait]
pub trait Handler: Send {
    #[cfg(feature = "postgres")]
    /// Accepts sqlx postresql transaction and an event.
    /// Returns a list of events to be published.
    ///
    /// # Errors
    ///
    /// Will return an [`Error`] if there is any error occurs when handling an event.
    async fn handle<'a>(
        &self,
        state: &mut Container![Send + Sync],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
        event: &Event,
    ) -> HandlingResult;

    #[cfg(feature = "mysql")]
    /// Accepts sqlx mysql transaction and an event.
    /// Returns a list of events to be published.
    ///
    /// # Errors
    ///
    /// Will return an [`Error`] if there is any error occurs when handling an event.
    async fn handle<'a>(
        &self,
        state: &mut Container![Send + Sync],
        tx: &mut sqlx::Transaction<'a, sqlx::MySql>,
        event: &Event,
    ) -> HandlingResult;

    #[cfg(feature = "sqlite")]
    /// Accepts sqlx sqlite transaction and an event.
    /// Returns a list of events to be published.
    ///
    /// # Errors
    ///
    /// Will return an [`Error`] if there is any error occurs when handling an event.
    async fn handle(
        &self,
        state: &mut Container![Send + Sync],
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        event: &Event,
    ) -> HandlingResult;
}
