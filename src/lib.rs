/*!
`diesel-tracing` provides connection structures that can be used as drop in
replacements for diesel connections with extra tracing and logging.

# Usage

## Feature flags

Just like diesel this crate relies on some feature flags to specify which
database driver to support. Just as in diesel configure this in your
`Cargo.toml`

```toml
[dependencies]
diesel-tracing = { version = "<version>", features = ["<postgres|mysql|sqlite>"] }
```

# Establishing a connection

`diesel-tracing` has several instrumented connection structs that wrap the underlying
`diesel` implementations of the connection. As these structs also implement the
`diesel::Connection` trait, establishing a connection is done in the same way as
the `diesel` crate. For example, with the `postgres` feature flag:

```
#[cfg(feature = "postgres")]
{
    use diesel_tracing::pg::InstrumentedPgConnection;

    let conn = InstrumentedPgConnection::establish("postgresql://example");
}
```

This connection can then be used with diesel dsl methods such as
`diesel::prelude::RunQueryDsl::execute` or `diesel::prelude::RunQueryDsl::get_results`.

# Code reuse

In some applications it may be desirable to be able to use both instrumented and
uninstrumented connections. For example, in the tests for a library. To achieve this
you can use the `diesel::Connection` trait.

```
fn use_connection(
    conn: &impl diesel::Connection<Backend = diesel::pg::Pg>,
) -> () {}
```

Will accept both `diesel::PgConnection` and the `InstrumentedPgConnection`
provided by this crate and this works similarly for other implementations
of `Connection` if you change the parametized Backend marker in the
function signature.

Unfortunately there are some methods specific to backends which are not
encapsulated by the `diesel::Connection` trait, so in those places it is
likely that you will just need to replace your connection type with the
Instrumented version.

## Connection Pooling

`diesel-tracing` supports the `r2d2` connection pool, through the `r2d2`
feature flag. See `diesel::r2d2` for details of usage.

# Notes

## Fields

Currently the few fields that are recorded are a subset of the `OpenTelemetry`
semantic conventions for [databases](https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/trace/semantic_conventions/database.md).
This was chosen for compatibility with the `tracing-opentelemetry` crate, but
if it makes sense for other standards to be available this could be set by
feature flag later.

Database statements may optionally be recorded by enabling the
`statement-fields` feature. This uses [`diesel::debug_query`](https://docs.rs/diesel/latest/diesel/fn.debug_query.html)
to convert the query into a string. As this may expose sensitive information,
the feature is not enabled by default.

It would be quite useful to be able to parse connection strings to be able
to provide more information, but this may be difficult if it requires use of
diesel feature flags by default to access the underlying C bindings.

## Levels

All logged traces are currently set to DEBUG level, potentially this could be
changed to a different default or set to be configured by feature flags. At
them moment this crate is quite new and it's unclear what a sensible default
would be.

## Errors

Errors in Result objects returned by methods on the connection should be
automatically logged through the `err` directive in the `instrument` macro.

## Sensitive Information

As statements may contain sensitive information they are currently not recorded
explicitly, unless you opt in by enabling the `statement-fields` feature.
Finding a way to filter statements intelligently to solve this problem is a
TODO.

Similarly connection strings are not recorded in spans as they may contain
passwords

## TODO

- [ ] Record and log connection information (filtering out sensitive fields)
- [ ] Provide a way of filtering statements, maybe based on regex?

*/
#![warn(clippy::all, clippy::pedantic)]

#[cfg(feature = "mysql")]
pub mod mysql;
#[cfg(feature = "postgres")]
pub mod pg;
#[cfg(feature = "sqlite")]
pub mod sqlite;

use diesel::connection::{Instrumentation, InstrumentationEvent};
use tracing::{event, Level};

pub struct TracingInstrumentation {
    include_url: bool,
}

impl TracingInstrumentation {
    #[must_use]
    pub fn new(include_url: bool) -> Self {
        Self { include_url }
    }
}

impl Instrumentation for TracingInstrumentation {
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        match event {
            InstrumentationEvent::StartEstablishConnection { url, .. } => {
                if self.include_url {
                    event!(name: "StartEstablishConnection", Level::DEBUG, "Started establishing connection with url: `{url}`", url = url);
                } else {
                    event!(name: "StartEstablishConnection", Level::DEBUG, "Started establishing connection");
                }
            }
            InstrumentationEvent::FinishEstablishConnection { url, error, .. } => {
                match (self.include_url, error) {
                    (true, Some(error)) => {
                        event!(name: "FinishEstablishConnection", Level::ERROR, "Failed to establish connection for `{url}`, error: {error}", url = url);
                    }
                    (true, None) => {
                        event!(name: "FinishEstablishConnection", Level::DEBUG, "Established connected to `{url}`", url = url);
                    }
                    (false, Some(error)) => {
                        event!(name: "FinishEstablishConnection", Level::ERROR, "Failed to establish connection, error: {error}");
                    }
                    (false, None) => {
                        event!(name: "FinishEstablishConnection", Level::DEBUG, "Established connection");
                    }
                }
            }
            InstrumentationEvent::StartQuery { query, .. } => {
                event!(
                    name: "StartedQuery",
                    Level::DEBUG,
                    "Started query: `{query}`",
                    query = query.to_string()
                );
            }
            InstrumentationEvent::CacheQuery { sql, .. } => {
                event!(name: "CacheQuery", Level::DEBUG, "Caching query: `{sql}`", sql = sql);
            }
            InstrumentationEvent::FinishQuery { query, error, .. } => {
                if let Some(error) = error {
                    event!(name: "FinishQuery", Level::ERROR, "Failed to execute query: `{query}`, error: {error}", query = query.to_string());
                } else {
                    event!(name: "FinishQuery", Level::DEBUG, "Finished query: `{query}`", query = query.to_string());
                }
            }
            InstrumentationEvent::BeginTransaction { depth, .. } => {
                event!(name: "BeginTransaction", Level::DEBUG, "Started transaction with depth: {depth}");
            }
            InstrumentationEvent::CommitTransaction { depth, .. } => {
                event!(name: "CommitTransaction", Level::DEBUG, "Commiting transaction with depth: {depth}");
            }
            InstrumentationEvent::RollbackTransaction { depth, .. } => {
                event!(name: "RollbackTransaction", Level::DEBUG, "Rolling back transaction with depth: {depth}");
            }
            _ => {
                event!(name: "<UnknownEvent>", Level::WARN, "Unknown event: {:?}", event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        num::NonZeroU64,
        sync::{atomic::AtomicU64, Arc, Mutex},
    };

    use diesel::{connection::set_default_instrumentation, sqlite, Connection, RunQueryDsl};
    use tracing::{span, Subscriber};

    use crate::TracingInstrumentation;

    // A subscriber that just copies and records events.
    #[derive(Default)]
    struct EventRecorder {
        span_id: AtomicU64,
        current_span: AtomicU64,
        // Debug formatted events in the order they are recorded.
        event_debug: Arc<Mutex<Vec<(Option<span::Id>, String)>>>,
    }

    impl EventRecorder {
        fn event_debug(&self) -> Arc<Mutex<Vec<(Option<span::Id>, String)>>> {
            Arc::clone(&self.event_debug)
        }
    }

    impl Subscriber for EventRecorder {
        fn event(&self, event: &tracing::Event<'_>) {
            let mut events = self.event_debug.lock().unwrap();
            let span_id =
                NonZeroU64::new(self.current_span.load(std::sync::atomic::Ordering::Relaxed))
                    .map(span::Id::from_non_zero_u64);
            events.push((span_id, format!("{:?}", event)))
        }

        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(
                self.span_id
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    + 1,
            )
        }

        fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {
            unimplemented!()
        }

        fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
            unimplemented!()
        }

        fn enter(&self, span: &span::Id) {
            let mut spans = self.event_debug.lock().unwrap();
            spans.push((Some(span.clone()), "enter span".to_string()));
            self.current_span
                .store(span.into_u64(), std::sync::atomic::Ordering::Relaxed);
        }

        fn exit(&self, span: &span::Id) {
            let mut events = self.event_debug.lock().unwrap();
            events.push((Some(span.clone()), "exit span".to_string()));
            self.current_span
                .store(0, std::sync::atomic::Ordering::Relaxed);
        }
    }

    #[test]
    fn handle_connection_events() -> Result<(), Box<dyn Error>> {
        let subscriber = EventRecorder::default();
        let event_debug = subscriber.event_debug();

        set_default_instrumentation(|| Some(Box::new(TracingInstrumentation::new(true)))).unwrap();
        tracing::subscriber::with_default(subscriber, || {
            let _conn = sqlite::SqliteConnection::establish(":memory:")?;

            Ok::<(), Box<dyn Error>>(())
        })?;
        set_default_instrumentation(|| None).unwrap();

        let events = event_debug.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(events[0]
            .1
            .contains("message: Started establishing connection with url: `:memory:`"));
        assert!(events[0].1.contains("module_path: \"diesel_tracing\""));

        assert!(events[1]
            .1
            .contains("message: Established connected to `:memory:`"));
        assert!(events[1].1.contains("module_path: \"diesel_tracing\""));

        Ok(())
    }

    #[test]
    fn handle_simple_queries_events() -> Result<(), Box<dyn Error>> {
        let subscriber = EventRecorder::default();
        let event_debug = subscriber.event_debug();
        let mut conn = sqlite::SqliteConnection::establish(":memory:")?;
        conn.set_instrumentation(TracingInstrumentation::new(true));

        tracing::subscriber::with_default(subscriber, || {
            let query = diesel::sql_query("SELECT 1");
            query.execute(&mut conn)?;

            Ok::<(), Box<dyn Error>>(())
        })?;

        let events = event_debug.lock().unwrap();
        assert_eq!(events.len(), 2);
        dbg!(&events);
        assert!(events[0]
            .1
            .contains("message: Started query: `SELECT 1 -- binds: []`"));
        assert!(events[0].1.contains("module_path: \"diesel_tracing\""));

        assert!(events[1]
            .1
            .contains("message: Finished query: `SELECT 1 -- binds: []`"));
        assert!(events[1].1.contains("module_path: \"diesel_tracing\""));

        Ok(())
    }

    #[test]
    fn handle_error_events() -> Result<(), Box<dyn Error>> {
        let subscriber = EventRecorder::default();
        let event_debug = subscriber.event_debug();
        let mut conn = sqlite::SqliteConnection::establish(":memory:")?;
        conn.set_instrumentation(TracingInstrumentation::new(true));

        tracing::subscriber::with_default(subscriber, || {
            // query with a syntax error
            let query = diesel::sql_query("SELECT DELETE");
            // Intentionally ignoring the error here
            let _res = query.execute(&mut conn);

            Ok::<(), Box<dyn Error>>(())
        })?;

        let events = event_debug.lock().unwrap();
        assert_eq!(events.len(), 2);
        dbg!(&events);
        assert!(events[0]
            .1
            .contains("message: Started query: `SELECT DELETE -- binds: []`"));
        assert!(events[0].1.contains("module_path: \"diesel_tracing\""));

        assert!(events[1].1.contains("message: Failed to execute query: `SELECT DELETE -- binds: []`, error: near \"DELETE\": syntax error"));
        assert!(events[1].1.contains("module_path: \"diesel_tracing\""));
        assert!(events[1].1.contains("level: Level(Error)"));

        Ok(())
    }

    #[test]
    fn handle_transactions() -> Result<(), Box<dyn Error>> {
        let subscriber = EventRecorder::default();
        let event_debug = subscriber.event_debug();
        let mut conn = sqlite::SqliteConnection::establish(":memory:")?;
        conn.set_instrumentation(TracingInstrumentation::new(true));

        tracing::subscriber::with_default(subscriber, || {
            conn.transaction(|conn| {
                let query = diesel::sql_query("SELECT 1");
                query.execute(conn)?;

                Ok::<(), Box<dyn Error>>(())
            })
        })?;

        let events = event_debug.lock().unwrap();
        assert_eq!(events.len(), 8);
        dbg!(&events);
        assert!(events[0]
            .1
            .contains("message: Started transaction with depth: 1"));
        assert!(events[0].1.contains("module_path: \"diesel_tracing\""));

        assert!(events[1].1.contains("message: Started query: `BEGIN`"));
        assert!(events[1].1.contains("module_path: \"diesel_tracing\""));

        assert!(events[2].1.contains("message: Finished query: `BEGIN`"));
        assert!(events[2].1.contains("module_path: \"diesel_tracing\""));

        assert!(events[3]
            .1
            .contains("message: Started query: `SELECT 1 -- binds: []`"));
        assert!(events[3].1.contains("module_path: \"diesel_tracing\""));

        assert!(events[4]
            .1
            .contains("message: Finished query: `SELECT 1 -- binds: []`"));
        assert!(events[4].1.contains("module_path: \"diesel_tracing\""));

        assert!(events[5]
            .1
            .contains("message: Commiting transaction with depth: 1"));
        assert!(events[5].1.contains("module_path: \"diesel_tracing\""));

        assert!(events[6].1.contains("message: Started query: `COMMIT`"));
        assert!(events[6].1.contains("module_path: \"diesel_tracing\""));

        assert!(events[7].1.contains("message: Finished query: `COMMIT`"));
        assert!(events[7].1.contains("module_path: \"diesel_tracing\""));
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn with_instrumented_connection() -> Result<(), Box<dyn Error>> {
        let subscriber = EventRecorder::default();
        let event_debug = subscriber.event_debug();
        let mut conn = crate::sqlite::InstrumentedSqliteConnection::establish(":memory:")?;
        conn.set_instrumentation(TracingInstrumentation::new(true));

        tracing::subscriber::with_default(subscriber, || {
            let query = diesel::sql_query("SELECT 1");
            query.execute(&mut conn)?;

            Ok::<(), Box<dyn Error>>(())
        })?;

        let events = event_debug.lock().unwrap();
        assert_eq!(events.len(), 4);
        dbg!(&events);

        assert_eq!(events[0].0.as_ref().unwrap().into_u64(), 1);
        assert!(events[0].1.contains("enter span"));

        assert_eq!(events[1].0.as_ref().unwrap().into_u64(), 1);
        assert!(events[1]
            .1
            .contains("message: Started query: `SELECT 1 -- binds: []`"));
        assert!(events[1].1.contains("module_path: \"diesel_tracing\""));

        assert_eq!(events[2].0.as_ref().unwrap().into_u64(), 1);
        assert!(events[2]
            .1
            .contains("message: Finished query: `SELECT 1 -- binds: []`"));
        assert!(events[2].1.contains("module_path: \"diesel_tracing\""));

        assert_eq!(events[3].0.as_ref().unwrap().into_u64(), 1);
        assert!(events[3].1.contains("exit span"));

        Ok(())
    }
}
