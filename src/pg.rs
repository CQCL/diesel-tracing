use diesel::associations::HasTable;
use diesel::connection::{
    AnsiTransactionManager, Connection, ConnectionSealed, DefaultLoadingMode, SimpleConnection,
};
use diesel::connection::{LoadConnection, TransactionManager};
use diesel::deserialize::Queryable;
use diesel::dsl::Update;
use diesel::expression::{is_aggregate, MixedAggregates, QueryMetadata, ValidGrouping};
use diesel::migration::{MigrationConnection, CREATE_MIGRATIONS_TABLE};
use diesel::pg::{GetPgMetadataCache, Pg, PgConnection, PgRowByRowLoadingMode, TransactionBuilder};
use diesel::query_builder::{AsChangeset, IntoUpdateTarget, Query, QueryFragment, QueryId};
use diesel::query_dsl::{LoadQuery, UpdateAndFetchResults};
use diesel::result::{ConnectionError, ConnectionResult, QueryResult};
use diesel::{select, Table};
use diesel::{sql_query, RunQueryDsl};
use tracing::{debug, field, instrument};

// https://www.postgresql.org/docs/12/functions-info.html
// db.name
sql_function!(fn current_database() -> diesel::sql_types::Text);
// net.peer.ip
sql_function!(fn inet_server_addr() -> diesel::sql_types::Inet);
// net.peer.port
sql_function!(fn inet_server_port() -> diesel::sql_types::Integer);
// db.version
sql_function!(fn version() -> diesel::sql_types::Text);

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
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        debug!("executing batch query");
        self.inner.batch_execute(query)?;

        Ok(())
    }
}

impl ConnectionSealed for InstrumentedPgConnection {}

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
        let mut conn = PgConnection::establish(database_url)?;

        debug!("querying postgresql connection information");
        let info: PgConnectionInfo = select((
            current_database(),
            inet_server_addr(),
            inet_server_port(),
            version(),
        ))
        .get_result(&mut conn)
        .map_err(ConnectionError::CouldntSetupConfiguration)?;

        let span = tracing::Span::current();
        span.record("db.name", info.current_database.as_str());
        span.record("db.version", info.version.as_str());
        span.record("net.peer.ip", format!("{}", info.inet_server_addr).as_str());
        span.record("net.peer.port", info.inet_server_port);

        Ok(InstrumentedPgConnection { inner: conn, info })
    }

    #[instrument(
        fields(
            db.name=%self.info.current_database,
            db.system="postgresql",
            db.version=%self.info.version,
            otel.kind="client",
            net.peer.ip=%self.info.inet_server_addr,
            net.peer.port=%self.info.inet_server_port,
        ),
        skip(self, f),
    )]
    fn transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
        E: From<diesel::result::Error>,
    {
        Self::TransactionManager::transaction(self, f)
    }

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
    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Pg> + QueryId,
    {
        self.inner.execute_returning_count(source)
    }

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
    fn transaction_state(&mut self) -> &mut Self::TransactionManager {
        self.inner.transaction_state()
    }
}

impl LoadConnection<DefaultLoadingMode> for InstrumentedPgConnection {
    type Cursor<'conn, 'query> =
        <PgConnection as LoadConnection<DefaultLoadingMode>>::Cursor<'conn, 'query>
            where
                Self: 'conn;
    type Row<'conn, 'query> =
        <PgConnection as LoadConnection<DefaultLoadingMode>>::Row<'conn, 'query>
            where
                Self: 'conn;

    #[cfg_attr(
        feature = "statement-fields",
        instrument(
            fields(
                db.name=%self.info.current_database,
                db.system="postgresql",
                db.version=%self.info.version,
                otel.kind="client",
                net.peer.ip=%self.info.inet_server_addr,
                net.peer.port=%self.info.inet_server_port,
                db.statement=%diesel::debug_query(&source),
            ),
            skip(self, source),
            err,
        )
    )]
    #[cfg_attr(
        not(feature = "statement-fields"),
        instrument(
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
        )
    )]
    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<Self::Cursor<'conn, 'query>>
    where
        T: Query + QueryFragment<Pg> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        <PgConnection as LoadConnection<DefaultLoadingMode>>::load(&mut self.inner, source)
    }
}

impl LoadConnection<PgRowByRowLoadingMode> for InstrumentedPgConnection {
    type Cursor<'conn, 'query> =
        <PgConnection as LoadConnection<PgRowByRowLoadingMode>>::Cursor<'conn, 'query>
    where
        Self: 'conn;
    type Row<'conn, 'query> =
        <PgConnection as LoadConnection<PgRowByRowLoadingMode>>::Row<'conn, 'query>
    where
        Self: 'conn;

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
    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<Self::Cursor<'conn, 'query>>
    where
        T: Query + QueryFragment<Pg> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        <PgConnection as LoadConnection<PgRowByRowLoadingMode>>::load(&mut self.inner, source)
    }
}

impl MigrationConnection for InstrumentedPgConnection {
    fn setup(&mut self) -> QueryResult<usize> {
        sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}

impl GetPgMetadataCache for InstrumentedPgConnection {
    fn get_metadata_cache(&mut self) -> &mut diesel::pg::PgMetadataCache {
        self.inner.get_metadata_cache()
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
    pub fn build_transaction(&mut self) -> TransactionBuilder<'_, InstrumentedPgConnection> {
        TransactionBuilder::new(self)
    }
}

impl<'b, Changes, Output> UpdateAndFetchResults<Changes, Output> for InstrumentedPgConnection
where
    Changes: Copy + AsChangeset<Target = <Changes as HasTable>::Table> + IntoUpdateTarget,
    Update<Changes, Changes>: LoadQuery<'b, PgConnection, Output>,
    <Changes::Table as Table>::AllColumns: ValidGrouping<()>,
    <<Changes::Table as Table>::AllColumns as ValidGrouping<()>>::IsAggregate:
        MixedAggregates<is_aggregate::No, Output = is_aggregate::No>,
{
    fn update_and_fetch(&mut self, changeset: Changes) -> QueryResult<Output> {
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
            &std::env::var("POSTGRESQL_URL").expect("no POSTGRESQL_URL env var specified"),
        )
        .expect("failed to establish connection or collect info");
    }
}
