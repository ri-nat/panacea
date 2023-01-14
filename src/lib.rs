#[cfg(not(feature = "sqlx"))]
compile_error!("you should enable one of the `runtime-*` features of `panacea`");

#[cfg(feature = "sqlx")]
pub mod event;

#[cfg(feature = "sqlx")]
pub mod outbox;
