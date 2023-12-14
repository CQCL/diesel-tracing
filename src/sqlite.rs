use diesel::associations::HasTable;
use diesel::connection::{
    AnsiTransactionManager, Connection, ConnectionSealed, DefaultLoadingMode, LoadConnection,
    SimpleConnection, TransactionManager,
};
use diesel::deserialize::{FromSqlRow, StaticallySizedRow};
use diesel::dsl::{Find, Update};
use diesel::expression::{is_aggregate, MixedAggregates, QueryMetadata, ValidGrouping};
use diesel::migration::{MigrationConnection, CREATE_MIGRATIONS_TABLE};
use diesel::query_builder::{AsChangeset, IntoUpdateTarget, Query, QueryFragment, QueryId};
use diesel::query_dsl::methods::{ExecuteDsl, FindDsl};
use diesel::query_dsl::{LoadQuery, UpdateAndFetchResults};
use diesel::result::{ConnectionResult, QueryResult};
use diesel::serialize::ToSql;
use diesel::sql_types::HasSqlType;
use diesel::sqlite::{Sqlite, SqliteConnection};
use diesel::RunQueryDsl;
use diesel::{sql_query, Identifiable, Table};
use tracing::{debug, instrument};

pub struct InstrumentedSqliteConnection {
    inner: SqliteConnection,
}

impl SimpleConnection for InstrumentedSqliteConnection {
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, query), err)]
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        self.inner.batch_execute(query)?;

        Ok(())
    }
}

impl ConnectionSealed for InstrumentedSqliteConnection {}

impl Connection for InstrumentedSqliteConnection {
    type Backend = Sqlite;
    type TransactionManager = AnsiTransactionManager;

    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(database_url), err)]
    fn establish(database_url: &str) -> ConnectionResult<InstrumentedSqliteConnection> {
        Ok(InstrumentedSqliteConnection {
            inner: SqliteConnection::establish(database_url)?,
        })
    }

    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, f))]
    fn transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
        E: From<diesel::result::Error>,
    {
        Self::TransactionManager::transaction(self, f)
    }

    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, source), err)]
    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Sqlite> + QueryId,
    {
        self.inner.execute_returning_count(source)
    }

    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self))]
    fn transaction_state(&mut self) -> &mut Self::TransactionManager {
        self.inner.transaction_state()
    }
}

impl LoadConnection<DefaultLoadingMode> for InstrumentedSqliteConnection {
    type Cursor<'conn, 'query> = <SqliteConnection as LoadConnection<DefaultLoadingMode>>::Cursor<'conn, 'query>
        where
            Self: 'conn;
    type Row<'conn, 'query> = <SqliteConnection as LoadConnection<DefaultLoadingMode>>::Row<'conn, 'query>
        where
            Self: 'conn;

    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, source), err)]
    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<Self::Cursor<'conn, 'query>>
    where
        T: Query + QueryFragment<Self::Backend> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        self.inner.load(source)
    }
}

impl MigrationConnection for InstrumentedSqliteConnection {
    fn setup(&mut self) -> QueryResult<usize> {
        sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}

impl InstrumentedSqliteConnection {
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, f))]
    pub fn immediate_transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut SqliteConnection) -> Result<T, E>,
        E: From<diesel::result::Error>,
    {
        self.inner.immediate_transaction(f)
    }

    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, f))]
    pub fn exclusive_transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut SqliteConnection) -> Result<T, E>,
        E: From<diesel::result::Error>,
    {
        self.inner.exclusive_transaction(f)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, f))]
    pub fn register_sql_function<ArgsSqlType, RetSqlType, Args, Ret, F>(
        &mut self,
        fn_name: &str,
        deterministic: bool,
        f: F,
    ) -> QueryResult<()>
    where
        F: FnMut(Args) -> Ret + std::panic::UnwindSafe + Send + 'static,
        Args: FromSqlRow<ArgsSqlType, Sqlite> + StaticallySizedRow<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        self.inner.register_sql_function(fn_name, deterministic, f)
    }
}

impl<'b, Changes, Output> UpdateAndFetchResults<Changes, Output> for InstrumentedSqliteConnection
where
    Changes: Copy + Identifiable,
    Changes: AsChangeset<Target = <Changes as HasTable>::Table> + IntoUpdateTarget,
    Changes::Table: FindDsl<Changes::Id>,
    Update<Changes, Changes>: ExecuteDsl<SqliteConnection>,
    Find<Changes::Table, Changes::Id>: LoadQuery<'b, SqliteConnection, Output>,
    <Changes::Table as Table>::AllColumns: ValidGrouping<()>,
    <<Changes::Table as Table>::AllColumns as ValidGrouping<()>>::IsAggregate:
        MixedAggregates<is_aggregate::No, Output = is_aggregate::No>,
{
    fn update_and_fetch(&mut self, changeset: Changes) -> QueryResult<Output> {
        debug!("updating and fetching changeset");
        self.inner.update_and_fetch(changeset)
    }
}
