use diesel::connection::{AnsiTransactionManager, Connection, SimpleConnection};
use diesel::deserialize::{Queryable, QueryableByName};
use diesel::query_builder::*;
use diesel::result::Error;
use diesel::result::{ConnectionResult, QueryResult};
use diesel::serialize::ToSql;
use diesel::sql_types::HasSqlType;
use diesel::sqlite::{Sqlite, SqliteConnection};
use tracing::instrument;

pub struct InstrumentedSqliteConnection {
    inner: SqliteConnection,
}

impl SimpleConnection for InstrumentedSqliteConnection {
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, query), err)]
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        self.inner.batch_execute(query)
    }
}

impl Connection for InstrumentedSqliteConnection {
    type Backend = Sqlite;
    type TransactionManager = AnsiTransactionManager;

    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(database_url), err)]
    fn establish(database_url: &str) -> ConnectionResult<InstrumentedSqliteConnection> {
        Ok(InstrumentedSqliteConnection {
            inner: SqliteConnection::establish(database_url)?,
        })
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, query), err)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.inner.execute(query)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, source), err)]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Sqlite> + QueryId,
        Sqlite: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Sqlite>,
    {
        self.inner.query_by_index(source)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, source), err)]
    fn query_by_name<T, U>(&self, source: &T) -> QueryResult<Vec<U>>
    where
        T: QueryFragment<Sqlite> + QueryId,
        U: QueryableByName<Sqlite>,
    {
        self.inner.query_by_name(source)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, source), err)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Sqlite> + QueryId,
    {
        self.inner.execute_returning_count(source)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self))]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.inner.transaction_manager()
    }
}

impl InstrumentedSqliteConnection {
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, f))]
    pub fn immediate_transaction<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<Error>,
    {
        self.inner.immediate_transaction(f)
    }

    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, f))]
    pub fn exclusive_transaction<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<Error>,
    {
        self.inner.exclusive_transaction(f)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="sqlite", otel.kind="client"), skip(self, f))]
    pub fn register_sql_function<ArgsSqlType, RetSqlType, Args, Ret, F>(
        &self,
        fn_name: &str,
        deterministic: bool,
        f: F,
    ) -> QueryResult<()>
    where
        F: FnMut(Args) -> Ret + Send + 'static,
        Args: Queryable<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        self.inner.register_sql_function(fn_name, deterministic, f)
    }
}
