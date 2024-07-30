use std::fmt::Debug;

use editoast_models::DbConnection;

use crate::error::EditoastError;
use crate::error::Result;

/// Describes how a [Model](super::Model) can be retrieved from the database
///
/// You can implement this type manually but its recommended to use the `Model`
/// derive macro instead.
#[async_trait::async_trait]
pub trait Retrieve<K>: Sized
where
    for<'async_trait> K: Send + 'async_trait,
{
    /// Retrieves the row #`id` and deserializes it as a model instance
    async fn retrieve(conn: &mut DbConnection, id: K) -> Result<Option<Self>>;

    /// Just like [Retrieve::retrieve] but returns `Err(fail())` if the row was not found
    async fn retrieve_or_fail<E, F>(
        conn: &'async_trait mut DbConnection,
        id: K,
        fail: F,
    ) -> Result<Self>
    where
        E: EditoastError,
        F: FnOnce() -> E + Send + 'async_trait,
    {
        match Self::retrieve(conn, id).await {
            Ok(Some(obj)) => Ok(obj),
            Ok(None) => Err(fail().into()),
            Err(e) => Err(e),
        }
    }
}

/// Describes how to check for the existence of a [Model](super::Model) in the database
///
/// You can implement this type manually but its recommended to use the `Model`
/// derive macro instead.
#[async_trait::async_trait]
pub trait Exists<K>: Sized
where
    for<'async_trait> K: Send + 'async_trait,
{
    /// Returns whether the row #`id` exists in the database
    async fn exists(conn: &mut DbConnection, id: K) -> Result<bool>;
}

/// Unchecked batch retrieval of a [Model](super::Model) from the database
///
/// Any [Model](super::Model) that implement this trait also implement [RetrieveBatch].
/// Unless you know what you're doing, you should use [RetrieveBatch] instead.
///
/// You can implement this type manually but its recommended to use the `Model`
/// derive macro instead.
#[async_trait::async_trait]
pub trait RetrieveBatchUnchecked<K>: Sized + Debug
where
    for<'async_trait> K: Send + Debug + 'async_trait,
{
    /// Retrieves a batch of rows from the database given an iterator of keys
    ///
    /// Returns a collection of the retrieved rows. That collection can contain
    /// fewer items than the number of provided keys if some rows were not found.
    /// Use [RetrieveBatch::retrieve_batch] or [RetrieveBatch::retrieve_batch_or_fail]
    /// if you want to fail if some rows were not found.
    /// Unless you know what you're doing, you should use these functions instead.
    async fn retrieve_batch_unchecked<
        I: IntoIterator<Item = K> + Send + 'async_trait,
        C: Default + std::iter::Extend<Self> + Send + Debug,
    >(
        conn: &mut DbConnection,
        ids: I,
    ) -> Result<C>;

    /// Just like [RetrieveBatchUnchecked::retrieve_batch_unchecked] but the returned models are paired with their key
    ///
    /// Returns a collection of the retrieved rows. That collection can contain
    /// fewer items than the number of provided keys if some rows were not found.
    /// Use [RetrieveBatch::retrieve_batch_with_key] or [RetrieveBatch::retrieve_batch_with_key_or_fail]
    /// if you want to fail if some rows were not found.
    /// Unless you know what you're doing, you should use these functions instead.
    async fn retrieve_batch_with_key_unchecked<
        I: IntoIterator<Item = K> + Send + 'async_trait,
        C: Default + std::iter::Extend<(K, Self)> + Send + Debug,
    >(
        conn: &mut DbConnection,
        ids: I,
    ) -> Result<C>;
}

/// Describes how a [Model](super::Model) can be retrieved from the database given a batch of keys
///
/// This trait is automatically implemented for all models that implement
/// [RetrieveBatchUnchecked]. [RetrieveBatchUnchecked] is a lower-level trait
/// which implementation is automatically generated by the `Model` derive macro.
///
/// 99% of the time you should use this trait instead of [RetrieveBatchUnchecked].
/// This won't be possible however if the model's key is not `Eq` or `Hash`.
#[async_trait::async_trait]
pub trait RetrieveBatch<K>: RetrieveBatchUnchecked<K>
where
    for<'async_trait> K: Eq + std::hash::Hash + Clone + Send + Debug + 'async_trait,
{
    /// Retrieves a batch of rows from the database given an iterator of keys
    ///
    /// Returns a collection of the retrieved rows and a set of the keys
    /// that were not found.
    ///
    /// ```
    /// let mut ids = (0..5).collect::<Vec<_>>();
    /// ids.push(123456789);
    /// let (docs, missing): (HashSet<_>, _) = Document::retrieve_batch(&mut conn, ids).await?;
    /// assert!(ids.contains(&123456789));
    /// assert_eq!(docs.len(), 5);
    /// ```
    async fn retrieve_batch<I, C>(
        conn: &mut DbConnection,
        ids: I,
    ) -> Result<(C, std::collections::HashSet<K>)>
    where
        I: Send + IntoIterator<Item = K> + 'async_trait,
        C: Send
            + Default
            + std::iter::Extend<Self>
            + std::iter::FromIterator<Self>
            + std::iter::IntoIterator<Item = Self>,
    {
        let ids = ids.into_iter().collect::<std::collections::HashSet<_>>();
        let (retrieved_ids, results): (std::collections::HashSet<_>, C) =
            Self::retrieve_batch_with_key_unchecked::<_, Vec<(_, _)>>(
                conn,
                ids.clone().into_iter(),
            )
            .await?
            .into_iter()
            .unzip();
        let missing = ids
            .difference(&retrieved_ids)
            .collect::<std::collections::HashSet<_>>();
        Ok((results, missing.into_iter().cloned().collect()))
    }

    /// Just like [RetrieveBatch::retrieve_batch] but the returned models are paired with their key
    ///
    /// ```
    /// let mut ids = (0..5).collect::<Vec<_>>();
    /// ids.push(123456789);
    /// let (docs, missing): (HashMap<_, _>, _) = Document::retrieve_batch_with_key(&mut conn, ids).await?;
    /// assert!(ids.contains(&123456789));
    /// assert!(docs.contains(&1));
    /// ```
    async fn retrieve_batch_with_key<I, C>(
        conn: &mut DbConnection,
        ids: I,
    ) -> Result<(C, std::collections::HashSet<K>)>
    where
        I: Send + IntoIterator<Item = K> + 'async_trait,
        C: Send
            + Default
            + std::iter::Extend<(K, Self)>
            + std::iter::FromIterator<(K, Self)>
            + std::iter::IntoIterator<Item = (K, Self)>,
    {
        let ids = ids.into_iter().collect::<std::collections::HashSet<_>>();
        let (retrieved_ids, results): (std::collections::HashSet<_>, C) =
            Self::retrieve_batch_with_key_unchecked::<_, Vec<(_, _)>>(
                conn,
                ids.clone().into_iter(),
            )
            .await?
            .into_iter()
            .map(|(k, v)| (k.clone(), (k, v)))
            .unzip();
        let missing = ids
            .difference(&retrieved_ids)
            .collect::<std::collections::HashSet<_>>();
        Ok((results, missing.into_iter().cloned().collect()))
    }

    /// Retrieves a batch of rows from the database given an iterator of keys
    ///
    /// Returns a collection of the retrieved rows and fails if some rows were not found.
    /// On failure, the error returned is the result of calling `fail(missing)` where `missing`
    /// is the set of ids that were not found.
    ///
    /// ```
    /// let ids = (0..5).collect::<Vec<_>>();
    /// let docs: HashSet<_> = Document::retrieve_batch_or_fail(&mut conn, ids, |missing| {
    ///    MyErrorType::DocumentsNotFound(missing)
    /// }).await?;
    /// ```
    async fn retrieve_batch_or_fail<I, C, E, F>(
        conn: &mut DbConnection,
        ids: I,
        fail: F,
    ) -> Result<C>
    where
        I: Send + IntoIterator<Item = K> + 'async_trait,
        C: Send
            + Default
            + std::iter::Extend<Self>
            + std::iter::FromIterator<Self>
            + std::iter::IntoIterator<Item = Self>,
        E: EditoastError,
        F: FnOnce(std::collections::HashSet<K>) -> E + Send + 'async_trait,
    {
        let (result, missing) = Self::retrieve_batch::<_, C>(conn, ids).await?;
        if missing.is_empty() {
            Ok(result)
        } else {
            Err(fail(missing).into())
        }
    }

    /// Just like [RetrieveBatch::retrieve_batch_or_fail] but the returned models are paired with their key
    ///
    /// ```
    /// let ids = (0..5).collect::<Vec<_>>();
    /// let docs: HashMap<_, _> = Document::retrieve_batch_with_key_or_fail(&mut conn, ids, |missing| {
    ///   MyErrorType::DocumentsNotFound(missing)
    /// }).await?;
    /// ```
    async fn retrieve_batch_with_key_or_fail<I, C, E, F>(
        conn: &mut DbConnection,
        ids: I,
        fail: F,
    ) -> Result<C>
    where
        I: Send + IntoIterator<Item = K> + 'async_trait,
        C: Send
            + Default
            + std::iter::Extend<(K, Self)>
            + std::iter::FromIterator<(K, Self)>
            + std::iter::IntoIterator<Item = (K, Self)>,
        E: EditoastError,
        F: FnOnce(std::collections::HashSet<K>) -> E + Send + 'async_trait,
    {
        let (result, missing) = Self::retrieve_batch_with_key::<_, C>(conn, ids).await?;
        if missing.is_empty() {
            Ok(result)
        } else {
            Err(fail(missing).into())
        }
    }
}

// Auto-impl of RetrieveBatch for all models that implement RetrieveBatchUnchecked
#[async_trait::async_trait]
impl<M, K> RetrieveBatch<K> for M
where
    M: RetrieveBatchUnchecked<K>,
    for<'async_trait> K: Eq + std::hash::Hash + Clone + Send + Debug + 'async_trait,
{
}
