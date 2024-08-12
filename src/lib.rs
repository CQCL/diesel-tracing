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
        sync::{Arc, Mutex},
    };

    use diesel::{connection::set_default_instrumentation, sqlite, Connection, RunQueryDsl};
    use tracing::{span, Subscriber};

    use crate::TracingInstrumentation;

    // A subscriber that just copies and records events.
    struct EventRecorder {
        // Debug formatted events in the order they are recorded.
        event_debug: Arc<Mutex<Vec<String>>>,
    }

    impl Subscriber for EventRecorder {
        fn event(&self, event: &tracing::Event<'_>) {
            let mut events = self.event_debug.lock().unwrap();
            events.push(format!("{:?}", event))
        }

        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
            unimplemented!()
        }

        fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {
            unimplemented!()
        }

        fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
            unimplemented!()
        }

        fn enter(&self, _span: &span::Id) {
            unimplemented!()
        }

        fn exit(&self, _span: &span::Id) {
            unimplemented!()
        }
    }

    #[test]
    fn handle_connection_events() -> Result<(), Box<dyn Error>> {
        let event_debug = Arc::new(Mutex::new(Vec::new()));
        let subscriber = EventRecorder {
            event_debug: Arc::clone(&event_debug),
        };

        set_default_instrumentation(|| Some(Box::new(TracingInstrumentation::new(true)))).unwrap();
        tracing::subscriber::with_default(subscriber, || {
            let _conn = sqlite::SqliteConnection::establish(":memory:")?;

            Ok::<(), Box<dyn Error>>(())
        })?;
        set_default_instrumentation(|| None).unwrap();

        let events = event_debug.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(events[0].contains("message: Started establishing connection with url: `:memory:`"));
        assert!(events[0].contains("module_path: \"diesel_tracing\""));

        assert!(events[1].contains("message: Established connected to `:memory:`"));
        assert!(events[1].contains("module_path: \"diesel_tracing\""));

        Ok(())
    }

    #[test]
    fn handle_simple_queries_events() -> Result<(), Box<dyn Error>> {
        let event_debug = Arc::new(Mutex::new(Vec::new()));
        let subscriber = EventRecorder {
            event_debug: Arc::clone(&event_debug),
        };
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
        assert!(events[0].contains("message: Started query: `SELECT 1 -- binds: []`"));
        assert!(events[0].contains("module_path: \"diesel_tracing\""));

        assert!(events[1].contains("message: Finished query: `SELECT 1 -- binds: []`"));
        assert!(events[1].contains("module_path: \"diesel_tracing\""));

        Ok(())
    }

    #[test]
    fn handle_transactions() -> Result<(), Box<dyn Error>> {
        let event_debug = Arc::new(Mutex::new(Vec::new()));
        let subscriber = EventRecorder {
            event_debug: Arc::clone(&event_debug),
        };
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
        assert!(events[0].contains("message: Started transaction with depth: 1"));
        assert!(events[0].contains("module_path: \"diesel_tracing\""));

        assert!(events[1].contains("message: Started query: `BEGIN`"));
        assert!(events[1].contains("module_path: \"diesel_tracing\""));

        assert!(events[2].contains("message: Finished query: `BEGIN`"));
        assert!(events[2].contains("module_path: \"diesel_tracing\""));

        assert!(events[3].contains("message: Started query: `SELECT 1 -- binds: []`"));
        assert!(events[3].contains("module_path: \"diesel_tracing\""));

        assert!(events[4].contains("message: Finished query: `SELECT 1 -- binds: []`"));
        assert!(events[4].contains("module_path: \"diesel_tracing\""));

        assert!(events[5].contains("message: Commiting transaction with depth: 1"));
        assert!(events[5].contains("module_path: \"diesel_tracing\""));

        assert!(events[6].contains("message: Started query: `COMMIT`"));
        assert!(events[6].contains("module_path: \"diesel_tracing\""));

        assert!(events[7].contains("message: Finished query: `COMMIT`"));
        assert!(events[7].contains("module_path: \"diesel_tracing\""));
        Ok(())
    }
}
