use async_std::task;
use core::time::Duration;
use sqlx::sqlite::SqlitePool;
use std::thread;

#[macro_use]
pub extern crate panacea;

use panacea::outbox::{store, EventRow};
use panacea_types::{event::Headers, handler::HandlingResult, state::State};

#[handler]
fn handle_some_stuff(_name: &State<String>) -> HandlingResult {
    eprintln!("some_state: 123");
    eprintln!("another_state: {:?}", "234");

    Ok(None)
}

#[async_std::main]
async fn main() {
    let db_conn = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Can't connect to SQLite");

    create_events_outbox_table(&db_conn).await;

    // Every second stores new event to the outbox table
    start_producing_events(db_conn.clone(), Duration::from_secs(1)).await;
    // Every 5 seconds prints number of events stored in the outbox table
    start_counting_events(db_conn, Duration::from_secs(5)).await;
}

async fn start_producing_events(db_conn: SqlitePool, duration: Duration) -> task::JoinHandle<()> {
    task::spawn(async move {
        loop {
            println!("-----> Storing event...");

            let mut headers = Headers::new();
            headers.insert("answer".to_string(), 42.to_string());

            store(
                &db_conn,
                "panacea.test",
                Some(420),
                "Hello, Panacea!",
                Some(headers),
            )
            .await
            .expect("Can't store event");

            thread::sleep(duration);
        }
    })
}

fn start_counting_events(db_conn: SqlitePool, duration: Duration) -> task::JoinHandle<()> {
    task::spawn(async move {
        loop {
            thread::sleep(duration);
            print_events_count(&db_conn).await;
        }
    })
}

async fn print_events_count(db_conn: &SqlitePool) {
    let row: (i32,) = sqlx::query_as("SELECT COUNT(*) FROM panacea_outbox")
        .fetch_one(db_conn)
        .await
        .expect("Can't get events count");

    let maybe_last_event: Option<EventRow> = sqlx::query_as::<_, EventRow>(
        "SELECT * FROM panacea_outbox ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(db_conn)
    .await
    .expect("Can't get last event from DB");

    println!("<----- Events count: {}", row.0);

    if let Some(event) = maybe_last_event {
        println!("<----- Last event: {event:?}");
    }
}

async fn create_events_outbox_table(db_conn: &SqlitePool) {
    sqlx::query(
        r#"
            CREATE TABLE panacea_outbox (
                topic TEXT,
                key TEXT,
                payload BLOB,
                headers TEXT,
                created_at TEXT
            )
        "#,
    )
    .execute(db_conn)
    .await
    .expect("Can't create `events` table");
}
