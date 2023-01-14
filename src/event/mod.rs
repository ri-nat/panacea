use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::outbox::EventRow;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("can't decode headers")]
    HeadersDecodingError(serde_json::Error),
}

pub type Headers = HashMap<String, String>;

#[derive(Default)]
pub struct Event {
    pub key: Option<String>,
    pub payload: Vec<u8>,
    pub headers: Headers,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<EventRow> for Event {
    type Error = Error;

    fn try_from(value: EventRow) -> Result<Self, Self::Error> {
        let headers: Headers =
            serde_json::from_str(&value.headers).map_err(Self::Error::HeadersDecodingError)?;

        Ok(Self {
            key: value.key,
            payload: value.payload,
            headers,
            created_at: value.created_at,
        })
    }
}
