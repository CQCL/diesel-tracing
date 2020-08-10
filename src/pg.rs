use diesel::connection::{AnsiTransactionManager, Connection, SimpleConnection};
use diesel::deserialize::{Queryable, QueryableByName};
use diesel::pg::{Pg, PgConnection, TransactionBuilder};
use diesel::query_builder::*;
use diesel::result::{ConnectionResult, QueryResult};
use diesel::sql_types::HasSqlType;
use tracing::instrument;

pub struct InstrumentedPgConnection {
    inner: PgConnection,
}

impl SimpleConnection for InstrumentedPgConnection {
    #[instrument(fields(db.system="postgresql", otel.kind="client"), skip(self, query), err)]
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        self.inner.batch_execute(query)
    }
}

impl Connection for InstrumentedPgConnection {
    type Backend = Pg;
    type TransactionManager = AnsiTransactionManager;

    #[instrument(fields(db.system="postgresql", otel.kind="client"), skip(database_url), err)]
    fn establish(database_url: &str) -> ConnectionResult<InstrumentedPgConnection> {
        Ok(InstrumentedPgConnection {
            inner: PgConnection::establish(database_url)?,
        })
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="postgresql", otel.kind="client"), skip(self, query), err)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.inner.execute(query)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="postgresql", otel.kind="client"), skip(self, source), err)]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Pg> + QueryId,
        Pg: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Pg>,
    {
        self.inner.query_by_index(source)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="postgresql", otel.kind="client"), skip(self, source), err)]
    fn query_by_name<T, U>(&self, source: &T) -> QueryResult<Vec<U>>
    where
        T: QueryFragment<Pg> + QueryId,
        U: QueryableByName<Pg>,
    {
        self.inner.query_by_name(source)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="postgresql", otel.kind="client"), skip(self, source), err)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Pg> + QueryId,
    {
        self.inner.execute_returning_count(source)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="postgresql", otel.kind="client"), skip(self))]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.inner.transaction_manager()
    }
}

impl InstrumentedPgConnection {
    #[instrument(fields(db.system="postgresql", otel.kind="client"), skip(self))]
    pub fn build_transaction(&self) -> TransactionBuilder {
        self.inner.build_transaction()
    }
}
