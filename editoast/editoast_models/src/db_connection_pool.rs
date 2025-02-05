mod tracing_instrumentation;

use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use diesel::sql_query;
use diesel::ConnectionError;
use diesel::ConnectionResult;
use diesel_async::pooled_connection::deadpool::Object;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::ManagerConfig;
use diesel_async::scoped_futures::ScopedBoxFuture;
use diesel_async::AsyncConnection;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use futures::future::BoxFuture;
use futures::Future;
use futures_util::FutureExt as _;
use openssl::ssl::SslConnector;
use openssl::ssl::SslMethod;
use openssl::ssl::SslVerifyMode;
use tracing::trace;
use url::Url;

use tokio::sync::OwnedRwLockWriteGuard;
use tokio::sync::RwLock;

pub type DbConnectionConfig = AsyncDieselConnectionManager<AsyncPgConnection>;

#[derive(Clone)]
pub struct DbConnection {
    inner: Arc<RwLock<Object<AsyncPgConnection>>>,
}

pub struct WriteHandle {
    guard: OwnedRwLockWriteGuard<Object<AsyncPgConnection>>,
}

impl DbConnection {
    pub fn new(inner: Arc<RwLock<Object<AsyncPgConnection>>>) -> Self {
        Self { inner }
    }

    pub async fn write(&self) -> WriteHandle {
        WriteHandle {
            guard: self.inner.clone().write_owned().await,
        }
    }

    // Implementation of this function is taking a strong inspiration from
    // https://docs.rs/diesel/2.1.6/src/diesel/connection/transaction_manager.rs.html#50-71
    // Sadly, this function is private so we can't use it.
    //
    // :WARNING: If you ever need to modify this function, please take a look at the
    // original `diesel` function, they probably do it right more than us.
    pub async fn transaction<'a, R, E, F>(&self, callback: F) -> std::result::Result<R, E>
    where
        F: FnOnce(Self) -> ScopedBoxFuture<'a, 'a, std::result::Result<R, E>> + Send + 'a,
        E: From<diesel::result::Error> + Send + 'a,
        R: Send + 'a,
    {
        use diesel_async::TransactionManager as _;

        type TxManager = <AsyncPgConnection as AsyncConnection>::TransactionManager;

        {
            let mut handle = self.write().await;
            TxManager::begin_transaction(handle.deref_mut()).await?;
        }

        match callback(self.clone()).await {
            Ok(result) => {
                let mut handle = self.write().await;
                TxManager::commit_transaction(handle.deref_mut()).await?;
                Ok(result)
            }
            Err(callback_error) => {
                let mut handle = self.write().await;
                match TxManager::rollback_transaction(handle.deref_mut()).await {
                    Ok(()) | Err(diesel::result::Error::BrokenTransactionManager) => {
                        Err(callback_error)
                    }
                    Err(rollback_error) => Err(rollback_error.into()),
                }
            }
        }
    }
}

impl Deref for WriteHandle {
    type Target = AsyncPgConnection;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl DerefMut for WriteHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

/// Wrapper for connection pooling with support for test connections on `cfg(test)`
///
/// # Testing pool
///
/// In test mode, the [Pool::<AsyncPgConnection>::get] function will always return the same connection that has
/// been setup to drop all modification once the test ends.
/// Since this connection will not commit any changes to the database, we ensure the isolation of each test.
///
/// A new pool is expected to be initialized for each test, see `DbConnectionPoolV2::for_tests`.
#[derive(Clone)]
pub struct DbConnectionPoolV2 {
    pool: Arc<Pool<AsyncPgConnection>>,
    #[cfg(feature = "testing")]
    test_connection: Option<DbConnection>,
}

#[cfg(feature = "testing")]
impl Default for DbConnectionPoolV2 {
    fn default() -> Self {
        Self::for_tests()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("an error occurred while building the database pool: '{0}'")]
pub struct DatabasePoolBuildError(#[from] diesel_async::pooled_connection::deadpool::BuildError);

#[derive(Debug, thiserror::Error)]
#[error("an error occurred while getting a connection from the database pool: '{0}'")]
pub struct DatabasePoolError(#[from] diesel_async::pooled_connection::deadpool::PoolError);

impl DbConnectionPoolV2 {
    /// Get inner pool for retro compatibility
    pub fn pool_v1(&self) -> Arc<Pool<AsyncPgConnection>> {
        self.pool.clone()
    }

    /// Creates a connection pool with the given settings
    ///
    /// In a testing environment, you should use `DbConnectionPoolV2::for_tests` instead.
    pub async fn try_initialize(url: Url, max_size: usize) -> Result<Self, DatabasePoolBuildError> {
        let pool = create_connection_pool(url, max_size)?;
        Ok(Self::from_pool(Arc::new(pool)).await)
    }

    #[cfg(feature = "testing")]
    async fn get_connection(&self) -> Result<DbConnection, DatabasePoolError> {
        Ok(self
            .test_connection
            .as_ref()
            .expect("should already exist")
            .clone())
    }

    #[cfg(not(feature = "testing"))]
    async fn get_connection(&self) -> Result<DbConnection, DatabasePoolError> {
        use diesel_async::AsyncConnection as _;

        let mut connection = self.pool.get().await?;
        connection.set_instrumentation(tracing_instrumentation::TracingInstrumentation::default());
        Ok(DbConnection::new(Arc::new(RwLock::new(connection))))
    }

    /// Get a connection from the pool
    ///
    /// This function behaves differently in test mode.
    ///
    /// # Production mode
    ///
    /// In production mode, this function will just return a connection from the pool, which may
    /// hold several opened. This function is intended to be a drop-in replacement for the
    /// `deadpool`'s `get` function.
    ///
    /// # Test mode
    ///
    /// In test mode, this function will return the same connection that has been setup to drop all
    /// modifications once the test ends. This connection is intended to be used in unit tests to
    /// ensure the isolation of each test.
    ///
    /// ## Pitfalls
    ///
    /// However, this comes with several limitations on how connections are used globally.
    ///
    /// 1. Once a connection is used, it should be dropped **AS SOON AS POSSIBLE**. Failing to do so
    ///    may lead to deadlocks. Example:
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() {
    /// let pool = editoast_models::DbConnectionPoolV2::for_tests();
    /// let conn = pool.get_ok();
    /// // Do something with conn
    ///
    /// // This will deadlock because `conn` hasn't been dropped yet. Since this function is
    /// // not async, this is equivalent to an infinite loop.
    /// let conn2 = pool.get_ok();
    /// # }
    /// ```
    ///
    /// 2. If several futures are spawned and each use their own connection, you should make sure
    ///    that the connection usage follows its acquisition. Failing to do so is equivalent to
    ///    the following example:
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() {
    /// let pool = editoast_models::DbConnectionPoolV2::for_tests();
    /// let conn_futures = (0..10).map(|_| async { pool.get() });
    /// let deadlock = futures::future::join_all(conn_futures).await;
    /// # }
    /// ```
    ///
    /// ### Deadlocks
    ///
    /// We encountered a deadlock error in our tests,
    /// especially those using `empty_infra` and `small_infra`.
    /// Adding `#[serial_test::serial]` solved the issue.
    /// We tried increasing the deadlock timeout, but that didn't work.
    /// Using random `infra_id` with rand didn't help either.
    ///
    /// ## Guidelines
    ///
    /// To prevent these issues, prefer the following patterns:
    ///
    /// - Don't declare a variable for a single-use connection:
    ///
    /// ```
    /// # async fn my_function_using_conn(conn: &mut editoast_models::DbConnection) {
    /// #   // Do something with the connection
    /// # }
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), editoast_models::db_connection_pool::DatabasePoolError> {
    /// let pool = editoast_models::DbConnectionPoolV2::for_tests();
    /// // do
    /// my_function_using_conn(&mut pool.get().await?).await;
    /// // instead of
    /// let mut conn = pool.get().await?;
    /// my_function_using_conn(&mut conn).await;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// - If a connection is used repeatedly, prefer using explicit scoping:
    ///
    /// ```
    /// # async fn foo(conn: &mut editoast_models::DbConnection) -> u8 {
    /// #   0
    /// # }
    /// # async fn bar(conn: &mut editoast_models::DbConnection) -> u8 {
    /// #   42
    /// # }
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), editoast_models::db_connection_pool::DatabasePoolError> {
    /// let pool = editoast_models::DbConnectionPoolV2::for_tests();
    /// let my_results = {
    ///     let conn = &mut pool.get().await?;
    ///     foo(conn).await + bar(conn).await
    /// };
    /// // you may acquire a new connection afterwards
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// - If you need to open several connections, then the connection must be
    ///   acquired just before its usage, and dropped just after, **all in the same future**.
    ///   And these futures must all be awaited before attempting to acquire a new connection.
    ///
    /// ```
    /// # trait DoSomething: Sized {
    /// #   async fn do_something(self, conn: &mut editoast_models::DbConnection) -> Result<(), editoast_models::db_connection_pool::DatabasePoolError> {
    /// #     // Do something with the connection
    /// #     Ok(())
    /// #   }
    /// # }
    /// # impl DoSomething for u8 {}
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    /// # let items = vec![0_u8; 2];
    /// let pool = editoast_models::DbConnectionPoolV2::for_tests();
    /// let operations =
    ///     items.into_iter()
    ///         .zip(pool.iter_conn())
    ///         .map(|(item, conn)| async move {
    ///             let mut conn = conn.await?; // note the await here
    ///             item.do_something(&mut conn).await
    ///         });
    /// let results = futures::future::try_join_all(operations).await?;
    /// // you may acquire a new connection afterwards
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self) -> Result<DbConnection, DatabasePoolError> {
        self.get_connection().await
    }

    /// Gets a test connection from the pool synchronously, failing if the connection is not available
    ///
    /// In unit tests, this is the preferred way to get a connection
    ///
    /// See [DbConnectionPoolV2::get] for more information on how connections should be used
    /// in tests.
    #[cfg(feature = "testing")]
    pub fn get_ok(&self) -> DbConnection {
        futures::executor::block_on(self.get()).expect("Failed to get test connection")
    }

    /// Returns an infinite iterator of futures resolving to connections acquired from the pool
    ///
    /// Meant to be used in conjunction with `zip` in order to instantiate a bunch of tasks to spawn.
    ///
    /// # Example
    ///
    /// ```
    /// # trait DoSomething: Sized {
    /// #   async fn do_something(self, conn: &mut editoast_models::DbConnection) -> Result<(), editoast_models::db_connection_pool::DatabasePoolError> {
    /// #     // Do something with the connection
    /// #     Ok(())
    /// #   }
    /// # }
    /// # impl DoSomething for u8 {}
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    /// # let items = vec![0_u8; 2];
    /// let pool = editoast_models::DbConnectionPoolV2::for_tests();
    /// let operations =
    ///     items.into_iter()
    ///         .zip(pool.iter_conn())
    ///         .map(|(item, conn)| async move {
    ///             let mut conn = conn.await?; // note the await here
    ///             item.do_something(&mut conn).await
    ///         });
    /// let results = futures::future::try_join_all(operations).await?;
    /// // you may acquire a new connection afterwards
    /// # Ok(())
    /// # }
    /// ```
    pub fn iter_conn(
        &self,
    ) -> impl Iterator<Item = impl Future<Output = Result<DbConnection, DatabasePoolError>> + '_>
    {
        std::iter::repeat_with(|| self.get())
    }

    #[cfg(not(feature = "testing"))]
    pub async fn from_pool(pool: Arc<Pool<AsyncPgConnection>>) -> Self {
        Self { pool }
    }

    #[cfg(feature = "testing")]
    pub async fn from_pool(pool: Arc<Pool<AsyncPgConnection>>) -> Self {
        Self::from_pool_test(pool, true).await
    }

    #[cfg(feature = "testing")]
    pub async fn from_pool_test(pool: Arc<Pool<AsyncPgConnection>>, transaction: bool) -> Self {
        use diesel_async::AsyncConnection as _;

        let mut connection = pool
            .get()
            .await
            .expect("cannot acquire a connection in the test pool");
        connection.set_instrumentation(tracing_instrumentation::TracingInstrumentation::default());
        if transaction {
            connection
                .begin_test_transaction()
                .await
                .expect("cannot begin a test transaction");
        }
        let test_connection = Some(DbConnection::new(Arc::new(RwLock::new(connection))));

        Self {
            pool,
            test_connection,
        }
    }

    #[cfg(feature = "testing")]
    fn new_for_tests(transaction: bool) -> Self {
        let url = std::env::var("OSRD_TEST_PG_URL")
            .unwrap_or_else(|_| String::from("postgresql://osrd:password@localhost/osrd"));
        let url = Url::parse(&url).expect("Failed to parse postgresql url");
        let pool =
            create_connection_pool(url, 2).expect("Failed to initialize test connection pool");
        futures::executor::block_on(Self::from_pool_test(Arc::new(pool), transaction))
    }

    /// Create a connection pool for testing purposes.
    /// This method will create a connection with a transaction that will be rolled back at the end of the test.
    ///
    /// You can set the `OSRD_TEST_PG_URL` environment variable to use a custom database url.
    #[cfg(feature = "testing")]
    pub fn for_tests() -> Self {
        Self::new_for_tests(true)
    }

    /// Create a connection pool for testing purposes without a transaction
    ///
    /// You can set the `OSRD_TEST_PG_URL` environment variable to use a custom database url.
    #[cfg(feature = "testing")]
    pub fn for_tests_no_transaction() -> Self {
        Self::new_for_tests(false)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("could not ping the database: '{0}'")]
pub struct PingError(#[from] diesel::result::Error);

pub async fn ping_database(conn: &mut DbConnection) -> Result<(), PingError> {
    sql_query("SELECT 1")
        .execute(conn.write().await.deref_mut())
        .await?;
    trace!("Database ping successful");
    Ok(())
}

fn create_connection_pool(
    url: Url,
    max_size: usize,
) -> Result<Pool<AsyncPgConnection>, DatabasePoolBuildError> {
    let mut manager_config = ManagerConfig::default();
    manager_config.custom_setup = Box::new(establish_connection);
    let manager = DbConnectionConfig::new_with_config(url, manager_config);
    Ok(Pool::builder(manager).max_size(max_size).build()?)
}

fn establish_connection(config: &str) -> BoxFuture<ConnectionResult<AsyncPgConnection>> {
    let fut = async {
        let mut connector_builder = SslConnector::builder(SslMethod::tls()).unwrap();
        connector_builder.set_verify(SslVerifyMode::NONE);
        let tls = postgres_openssl::MakeTlsConnector::new(connector_builder.build());
        let (client, conn) = tokio_postgres::connect(config, tls)
            .await
            .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;
        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::error!("connection error: {}", e);
            }
        });
        AsyncPgConnection::try_from(client).await
    };
    fut.boxed()
}
