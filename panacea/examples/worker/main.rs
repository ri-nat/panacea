use std::collections::VecDeque;

use async_trait::async_trait;
use panacea::worker::Worker;
use panacea_types::{Event, EventSource, MaybeHandlers};

#[derive(Default)]
struct TestEventSource {
    pub events: VecDeque<Event>,
    current_event: Option<Event>,
}

#[async_trait]
impl EventSource for TestEventSource {
    async fn next(&mut self) -> Option<&Event> {
        if self.current_event.is_none() {
            self.current_event = self.events.pop_front();
        }

        self.current_event.as_ref()
    }

    fn succeeded(&mut self, _event: &Event) {}
    fn failed(&mut self, _event: &Event) {}
    fn skipped(&mut self, _event: &Event) {}
}

#[async_std::main]
async fn main() {
    Worker::new(TestEventSource::default())
        .with_ctrlc_handling()
        .with_handlers_resolver(resolver)
        .run()
        .await;
}

fn resolver(_event: &Event) -> MaybeHandlers {
    None
}
