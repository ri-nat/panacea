#![deny(clippy::unwrap_used, unsafe_code)]

pub mod event;
pub mod handler;
pub mod state;
pub mod worker;

pub use event::Event;
pub use worker::EventSource;

#[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
pub use handler::Handler;

pub use handler::{HandlingResult, MaybeHandlers};
// pub use handler::HandlingResult;
