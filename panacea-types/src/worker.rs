use async_trait::async_trait;

use crate::event::Event;

/// Represents source of events for feeding the event processing loop.
#[async_trait]
pub trait EventSource {
    async fn next(&mut self) -> Option<&Event>;
    fn failed(&mut self, event: &Event);
    fn succeeded(&mut self, event: &Event);
    fn skipped(&mut self, event: &Event);
}
