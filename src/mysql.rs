use diesel::associations::HasTable;
use diesel::connection::{AnsiTransactionManager, Connection, SimpleConnection};
use diesel::deserialize::{Queryable, QueryableByName};
use diesel::dsl::{Find, Update};
use diesel::mysql::{Mysql, MysqlConnection};
use diesel::query_builder::{AsChangeset, AsQuery, IntoUpdateTarget, QueryFragment, QueryId};
use diesel::query_dsl::methods::{ExecuteDsl, FindDsl};
use diesel::query_dsl::{LoadQuery, UpdateAndFetchResults};
use diesel::result::{ConnectionResult, QueryResult};
use diesel::sql_types::HasSqlType;
use diesel::Identifiable;
use tracing::instrument;

pub struct InstrumentedMysqlConnection {
    inner: MysqlConnection,
}

impl SimpleConnection for InstrumentedMysqlConnection {
    #[instrument(fields(db.system="mysql", otel.kind="client"), skip(self, query), err)]
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        self.inner.batch_execute(query)?;

        Ok(())
    }
}

impl Connection for InstrumentedMysqlConnection {
    type Backend = Mysql;
    type TransactionManager = AnsiTransactionManager;

    #[instrument(fields(db.system="mysql", otel.kind="client"), skip(database_url), err)]
    fn establish(database_url: &str) -> ConnectionResult<InstrumentedMysqlConnection> {
        Ok(InstrumentedMysqlConnection {
            inner: MysqlConnection::establish(database_url)?,
        })
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="mysql", otel.kind="client"), skip(self, query), err)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.inner.execute(query)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="mysql", otel.kind="client"), skip(self, source), err)]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Mysql> + QueryId,
        Mysql: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Mysql>,
    {
        self.inner.query_by_index(source)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="mysql", otel.kind="client"), skip(self, source), err)]
    fn query_by_name<T, U>(&self, source: &T) -> QueryResult<Vec<U>>
    where
        T: QueryFragment<Mysql> + QueryId,
        U: QueryableByName<Mysql>,
    {
        self.inner.query_by_name(source)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="mysql", otel.kind="client"), skip(self, source), err)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Mysql> + QueryId,
    {
        self.inner.execute_returning_count(source)
    }

    #[doc(hidden)]
    #[instrument(fields(db.system="mysql", otel.kind="client"), skip(self))]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.inner.transaction_manager()
    }
}

impl<Changes, Output> UpdateAndFetchResults<Changes, Output> for InstrumentedMysqlConnection
where
    Changes: Copy + Identifiable,
    Changes: AsChangeset<Target = <Changes as HasTable>::Table> + IntoUpdateTarget,
    Changes::Table: FindDsl<Changes::Id>,
    Update<Changes, Changes>: ExecuteDsl<MysqlConnection>,
    Find<Changes::Table, Changes::Id>: LoadQuery<MysqlConnection, Output>,
{
    fn update_and_fetch(&self, changeset: Changes) -> QueryResult<Output> {
        self.inner.update_and_fetch(changeset)
    }
}
