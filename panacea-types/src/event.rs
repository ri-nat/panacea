use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Types that can be represented as a reference to a bytes array.
pub trait AsBytesRef {
    fn as_bytes_ref(&self) -> &[u8];
}

impl AsBytesRef for String {
    fn as_bytes_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsBytesRef for &str {
    fn as_bytes_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsBytesRef for &[u8] {
    fn as_bytes_ref(&self) -> &[u8] {
        self
    }
}

impl AsBytesRef for Vec<u8> {
    fn as_bytes_ref(&self) -> &[u8] {
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("malformed headers")]
    MalformedHeaders(serde_json::Error),
}

pub type Headers = HashMap<String, String>;

#[derive(Debug, Default, Clone)]
pub struct Event {
    pub topic: String,
    pub key: Option<String>,
    pub payload: Vec<u8>,
    pub headers: Headers,
    pub created_at: DateTime<Utc>,
}

pub fn new<K, P, T>(topic: &T, key: Option<K>, payload: &P, headers: Option<Headers>) -> Event
where
    K: ToString + Default,
    P: AsBytesRef,
    T: ToString,
{
    Event {
        topic: topic.to_string(),
        key: Some(key.unwrap_or_default().to_string()),
        payload: payload.as_bytes_ref().to_vec(),
        headers: headers.unwrap_or_default(),

        ..Default::default()
    }
}
