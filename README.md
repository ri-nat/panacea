# Panacea

> WARNING: This project is still in development and is not ready for production use. But it will be soon.

Panacea is an event streaming and processing framework, based around the transactional outbox pattern. It is ideal for building consistent distributed event-driven systems.

## Features

- Transactions outbox pattern (for PostgreSQL, MySQL, and SQLite - via `sqlx`)
- `Worker` abstraction for processing events stream
- Pluggable event sources (`panacea` will provide at least `Kafka` source, but you can implement your own)
- Extensive logging and metrics
- Easy to use API

## Getting Started

> TODO

You can refer to the [examples](./examples) directory for now.

### License

Panacea is open-sourced under the [MIT license](./LICENSE)
