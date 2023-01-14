use async_std::task;
use core::time::Duration;
use sqlx::sqlite::SqlitePool;
use std::thread;

use panacea::{
    event::{Event, Headers},
    outbox::{store, EventRow},
};

fn main() {
    task::block_on(run());
}

async fn run() {
    let db_conn = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Can't connect to SQLite");

    create_events_outbox_table(&db_conn).await;

    start_event_storing(db_conn.clone()).await;
    start_event_counting(db_conn).await;
}

async fn start_event_storing(db_conn: SqlitePool) -> task::JoinHandle<()> {
    task::spawn(async move {
        loop {
            println!("-----> Storing event...");

            let mut headers = Headers::new();
            headers.insert("answer".to_string(), 42.to_string());

            store(
                &db_conn,
                Event {
                    key: Some(420.to_string()),
                    payload: "data".as_bytes().to_vec(),
                    headers,
                    ..Default::default()
                },
            )
            .await
            .expect("Can't store event");

            thread::sleep(Duration::from_secs(1));
        }
    })
}

fn start_event_counting(db_conn: SqlitePool) -> task::JoinHandle<()> {
    task::spawn(async move {
        loop {
            thread::sleep(Duration::from_secs(5));
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
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT,
                payload BLOB,
                headers TEXT,
                created_at TEXT,
                is_delivered BOOLEAN
            )
        "#,
    )
    .execute(db_conn)
    .await
    .expect("Can't create `events` table");
}
