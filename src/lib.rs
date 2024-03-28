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
