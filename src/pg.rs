use diesel::associations::HasTable;
use diesel::connection::{AnsiTransactionManager, Connection, SimpleConnection};
use diesel::deserialize::{Queryable, QueryableByName};
use diesel::dsl::Update;
use diesel::pg::{Pg, PgConnection, TransactionBuilder};
use diesel::query_builder::{AsChangeset, AsQuery, IntoUpdateTarget, QueryFragment, QueryId};
use diesel::query_dsl::{LoadQuery, UpdateAndFetchResults};
use diesel::result::{ConnectionError, ConnectionResult, QueryResult};
use diesel::sql_types::HasSqlType;
use diesel::RunQueryDsl;
use diesel::{no_arg_sql_function, select};
use tracing::{debug, field, instrument};

// https://www.postgresql.org/docs/12/functions-info.html
// db.name
no_arg_sql_function!(current_database, diesel::sql_types::Text);
// net.peer.ip
no_arg_sql_function!(inet_server_addr, diesel::sql_types::Inet);
// net.peer.port
no_arg_sql_function!(inet_server_port, diesel::sql_types::Integer);
// db.version
no_arg_sql_function!(version, diesel::sql_types::Text);

#[derive(Queryable, Clone, Debug, PartialEq)]
struct PgConnectionInfo {
    current_database: String,
    inet_server_addr: ipnetwork::IpNetwork,
    inet_server_port: i32,
    version: String,
}

pub struct InstrumentedPgConnection {
    inner: PgConnection,
    info: PgConnectionInfo,
}

impl SimpleConnection for InstrumentedPgConnection {
    #[instrument(
        fields(
            db.name=%self.info.current_database,
            db.system="postgresql",
            db.version=%self.info.version,
            otel.kind="client",
            net.peer.ip=%self.info.inet_server_addr,
            net.peer.port=%self.info.inet_server_port,
        ),
        skip(self, query),
        err,
    )]
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        debug!("executing batch query");
        self.inner.batch_execute(query)?;

        Ok(())
    }
}

impl Connection for InstrumentedPgConnection {
    type Backend = Pg;
    type TransactionManager = AnsiTransactionManager;

    #[instrument(
        fields(
            db.name=field::Empty,
            db.system="postgresql",
            db.version=field::Empty,
            otel.kind="client",
            net.peer.ip=field::Empty,
            net.peer.port=field::Empty,
        ),
        skip(database_url),
        err,
    )]
    fn establish(database_url: &str) -> ConnectionResult<InstrumentedPgConnection> {
        debug!("establishing postgresql connection");
        let conn = PgConnection::establish(database_url)?;

        debug!("querying postgresql connection information");
        let info: PgConnectionInfo = select((
            current_database,
            inet_server_addr,
            inet_server_port,
            version,
        ))
        .get_result(&conn)
        .map_err(ConnectionError::CouldntSetupConfiguration)?;

        let span = tracing::Span::current();
        span.record("db.name", &info.current_database.as_str());
        span.record("db.version", &info.version.as_str());
        span.record(
            "net.peer.ip",
            &format!("{}", info.inet_server_addr).as_str(),
        );
        span.record("net.peer.port", &info.inet_server_port);

        Ok(InstrumentedPgConnection { inner: conn, info })
    }

    #[doc(hidden)]
    #[instrument(
        fields(
            db.name=%self.info.current_database,
            db.system="postgresql",
            db.version=%self.info.version,
            otel.kind="client",
            net.peer.ip=%self.info.inet_server_addr,
            net.peer.port=%self.info.inet_server_port,
        ),
        skip(self, query),
        err,
    )]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        debug!("executing query");
        self.inner.execute(query)
    }

    #[doc(hidden)]
    #[instrument(
        fields(
            db.name=%self.info.current_database,
            db.system="postgresql",
            db.version=%self.info.version,
            otel.kind="client",
            net.peer.ip=%self.info.inet_server_addr,
            net.peer.port=%self.info.inet_server_port,
        ),
        skip(self, source),
        err,
    )]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Pg> + QueryId,
        Pg: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Pg>,
    {
        debug!("querying by index");
        self.inner.query_by_index(source)
    }

    #[doc(hidden)]
    #[instrument(
        fields(
            db.name=%self.info.current_database,
            db.system="postgresql",
            db.version=%self.info.version,
            otel.kind="client",
            net.peer.ip=%self.info.inet_server_addr,
            net.peer.port=%self.info.inet_server_port,
        ),
        skip(self, source),
        err,
    )]
    fn query_by_name<T, U>(&self, source: &T) -> QueryResult<Vec<U>>
    where
        T: QueryFragment<Pg> + QueryId,
        U: QueryableByName<Pg>,
    {
        debug!("querying by name");
        self.inner.query_by_name(source)
    }

    #[doc(hidden)]
    #[instrument(
        fields(
            db.name=%self.info.current_database,
            db.system="postgresql",
            db.version=%self.info.version,
            otel.kind="client",
            net.peer.ip=%self.info.inet_server_addr,
            net.peer.port=%self.info.inet_server_port,
        ),
        skip(self, source),
        err,
    )]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Pg> + QueryId,
    {
        debug!("executing returning count");
        self.inner.execute_returning_count(source)
    }

    #[doc(hidden)]
    #[instrument(
        fields(
            db.name=%self.info.current_database,
            db.system="postgresql",
            db.version=%self.info.version,
            otel.kind="client",
            net.peer.ip=%self.info.inet_server_addr,
            net.peer.port=%self.info.inet_server_port,
        ),
        skip(self),
    )]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        debug!("retrieving transaction manager");
        &self.inner.transaction_manager()
    }
}

impl InstrumentedPgConnection {
    #[instrument(
        fields(
            db.name=%self.info.current_database,
            db.system="postgresql",
            db.version=%self.info.version,
            otel.kind="client",
            net.peer.ip=%self.info.inet_server_addr,
            net.peer.port=%self.info.inet_server_port,
        ),
        skip(self),
    )]
    pub fn build_transaction(&self) -> TransactionBuilder {
        debug!("starting transaction builder");
        self.inner.build_transaction()
    }
}

impl<Changes, Output> UpdateAndFetchResults<Changes, Output> for InstrumentedPgConnection
where
    Changes: Copy + AsChangeset<Target = <Changes as HasTable>::Table> + IntoUpdateTarget,
    Update<Changes, Changes>: LoadQuery<PgConnection, Output>,
{
    fn update_and_fetch(&self, changeset: Changes) -> QueryResult<Output> {
        debug!("updating and fetching changeset");
        self.inner.update_and_fetch(changeset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_info_on_establish() {
        InstrumentedPgConnection::establish(
            &std::env::var("POSTGRESQL_URL").expect("no postgresql env var specified"),
        )
        .expect("failed to establish connection or collect info");
    }
}
