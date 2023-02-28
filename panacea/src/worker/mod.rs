use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::outbox;
use async_std::sync::Mutex;
use panacea_types::{event::Event, handler::MaybeHandlers, state::State, worker::EventSource};
use state::Container;

pub struct Worker<S: EventSource> {
    /// [`EventSource`] instance.
    event_source: Arc<Mutex<S>>,
    /// Function, that resolves [`panacea_types::Handler`]'s from given [`Event`].
    handlers_resolver: Box<dyn Fn(&Event) -> MaybeHandlers + Send>,
    /// Holds sqlx PostgreSQL connection pool.
    #[cfg(feature = "postgres")]
    db: Option<sqlx::PgPool>,
    /// Holds sqlx MySQL connection pool.
    #[cfg(feature = "mysql")]
    db: Option<sqlx::MySqlPool>,
    /// Holds sqlx SQLite connection pool.
    #[cfg(feature = "sqlite")]
    db: Option<sqlx::SqlitePool>,
    /// Worker activeness flag.
    is_active: Arc<AtomicBool>,
    /// Managed state.
    pub state: Container![Sync + Send],
}

impl<S> Worker<S>
where
    S: EventSource + 'static,
{
    pub fn new(event_source: S) -> Self {
        Self {
            event_source: Arc::new(Mutex::new(event_source)),
            handlers_resolver: Box::new(|_| {
                println!("Warning: using default handler name resolver");
                None
            }),
            #[cfg(feature = "postgres")]
            db: None,
            #[cfg(feature = "mysql")]
            db: None,
            #[cfg(feature = "sqlite")]
            db: None,
            is_active: Arc::new(AtomicBool::new(true)),
            state: <Container![Send + Sync]>::new(),
        }
    }

    pub async fn run(mut self) {
        println!("Starting events consuming...");

        'outer: while self.is_active.load(Ordering::SeqCst) {
            let mut es_lock = self.event_source.lock().await;
            let Some(event) = es_lock.next().await else {
                continue
            };
            println!("{event:?}");

            // Resolve handlers
            let Some(handlers) = (self.handlers_resolver)(event) else {
                self.event_source.lock().await.skipped(event);
                continue;
            };

            // Handle event
            #[cfg(any(feature = "mysql", feature = "postgres", feature = "sqlite"))]
            if let Some(db) = &self.db {
                // `ALL` handlers processing (tries to handle event with all handlers, fails if any of them fails)
                //
                // TODO: implement `ANY` handlers processing (processes event with all handlers,
                // even if some of them fails).

                // Begin transaction
                let mut tx = db.begin().await.expect("Can't begin transaction");

                // Handle event
                for handler in handlers {
                    match handler.handle(&mut self.state, &mut tx, event).await {
                        // Everything is ok, got some events back
                        Ok(Some(events)) => {
                            for event in events {
                                outbox::store_event(&mut tx, event)
                                    .await
                                    .expect("Can't store event");
                            }
                        }
                        // Everything is ok, no events
                        Ok(None) => {}
                        // Something went wrong
                        Err(_) => {
                            self.event_source.lock().await.failed(event);
                            continue 'outer;
                        }
                    };
                }

                // Commit transaction
                tx.commit().await.expect("Can't commit transaction");
            }

            // Handle succeeded
            self.event_source.lock().await.succeeded(event);
        }
    }

    /// Sets [`panacea_types::Handler`] name resolver function.
    /// This function is used to get [`panacea_types::Handler`] name from given [`Event`].
    #[must_use]
    pub fn with_handlers_resolver<F>(mut self, resolver: F) -> Self
    where
        F: Fn(&Event) -> MaybeHandlers + Send + 'static,
    {
        self.handlers_resolver = Box::new(resolver);

        self
    }

    /// Sets [`sqlx::MySqlPool`] to the [`Worker`].
    #[cfg(feature = "mysql")]
    #[must_use]
    pub fn with_db(mut self, db: sqlx::MySqlPool) -> Self {
        self.db = Some(db);

        self
    }

    /// Sets [`sqlx::PgPool`] to the [`Worker`].
    #[cfg(feature = "postgres")]
    #[must_use]
    pub fn with_db(mut self, db: sqlx::PgPool) -> Self {
        self.db = Some(db);

        self
    }

    /// Sets [`sqlx::SqlitePool`] to the [`Worker`].
    #[cfg(feature = "sqlite")]
    #[must_use]
    pub fn with_db(mut self, db: sqlx::SqlitePool) -> Self {
        self.db = Some(db);

        self
    }

    #[must_use]
    pub fn with_activeness_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.is_active = flag;

        self
    }

    #[cfg(feature = "ctrlc")]
    #[must_use]
    pub fn with_ctrlc_handling(self) -> Self {
        let r = self.is_active.clone();

        ctrlc::set_handler(move || {
            println!("Shutting down worker...");
            r.store(false, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");

        self
    }

    #[inline]
    pub fn get_state<T: Send + Sync + 'static>(&self) -> Option<&State<T>> {
        self.state.try_get::<State<T>>()
    }
}

#[cfg(test)]
mod tests {
    use async_std::task;
    use async_trait::async_trait;
    use core::time;
    use std::collections::VecDeque;

    use super::*;
    use panacea_proc_macros::{handler, handlers};
    use panacea_types::handler::HandlingResult;

    struct TestEventSource {
        events: VecDeque<Event>,
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

    async fn run_worker<T>(es: T) -> Result<(), ()>
    where
        T: EventSource + std::marker::Sync + std::marker::Send + 'static,
    {
        let is_active = Arc::new(AtomicBool::new(true));
        let worker = Worker::new(es).with_activeness_flag(is_active.clone());
        task::spawn(async move {
            std::thread::sleep(time::Duration::from_millis(10));
            is_active.store(false, Ordering::SeqCst);
        })
        .await;

        task::spawn(async {
            worker.run().await;
            Ok(())
        })
        .await
    }

    #[async_std::test]
    async fn consumes_event_source() {
        let col = TestEventSource {
            events: VecDeque::from([Event::default()]),
            current_event: None,
        };

        // TODO: write something meaningful here
        let result = run_worker(col).await;
        assert!(result.is_ok());
    }

    // Test `with_handler` name generation correctness
    #[async_std::test]
    async fn with_handler_name() {
        #[handler]
        fn handle_some_stuff(_s: &State<String>) -> HandlingResult {
            Ok(None)
        }

        let col = TestEventSource {
            events: VecDeque::from([Event::default()]),
            current_event: None,
        };

        let _worker = Worker::new(col).with_handlers_resolver(|_| handlers![handle_some_stuff]);
    }
}
