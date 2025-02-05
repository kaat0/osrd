use std::collections::HashMap;

use itertools::Itertools;
use serde::Deserialize;
use url::Url;

#[cfg(any(test, feature = "mock_client"))]
mod mock_client;

pub struct OsrdyneClient {
    inner: OsrdyneClientInternal,
}

enum OsrdyneClientInternal {
    HTTPClient(HTTPClient),
    #[cfg(any(test, feature = "mock_client"))]
    MockClient(mock_client::MockClient),
}

struct HTTPClient {
    client: reqwest::Client,
    base_url: url::Url,
}

impl OsrdyneClient {
    pub fn new(osrdyne_url: Url) -> Self {
        OsrdyneClient {
            inner: OsrdyneClientInternal::HTTPClient(HTTPClient {
                client: reqwest::Client::new(),
                base_url: osrdyne_url,
            }),
        }
    }

    #[cfg(any(test, feature = "mock_client"))]
    pub fn mock() -> mock_client::MockClientBuilder {
        mock_client::MockClientBuilder::default()
    }

    #[cfg(any(test, feature = "mock_client"))]
    pub fn default_mock() -> Self {
        OsrdyneClient {
            inner: OsrdyneClientInternal::MockClient(mock_client::MockClient::default()),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum WorkerStatus {
    Unscheduled,
    Started,
    Ready,
    Error,
}

fn combine_worker_status(a: WorkerStatus, b: WorkerStatus) -> WorkerStatus {
    match (a, b) {
        (WorkerStatus::Ready, _) | (_, WorkerStatus::Ready) => WorkerStatus::Ready,
        (WorkerStatus::Started, _) | (_, WorkerStatus::Started) => WorkerStatus::Started,
        (WorkerStatus::Error, _) | (_, WorkerStatus::Error) => WorkerStatus::Error,
        (WorkerStatus::Unscheduled, WorkerStatus::Unscheduled) => WorkerStatus::Unscheduled,
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum OsrdyneWorkerStatus {
    Loading,
    Ready,
}

impl From<OsrdyneWorkerStatus> for WorkerStatus {
    fn from(status: OsrdyneWorkerStatus) -> Self {
        match status {
            OsrdyneWorkerStatus::Loading => WorkerStatus::Started,
            OsrdyneWorkerStatus::Ready => WorkerStatus::Ready,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    HTTPError(#[from] reqwest::Error),
    #[error("URL parse error: {0}")]
    URLParseError(#[from] url::ParseError),
}

impl OsrdyneClient {
    pub async fn get_worker_status(&self, worker_key: &str) -> Result<WorkerStatus, Error> {
        let res = self
            .get_workers_statuses(&[worker_key])
            .await?
            .remove(worker_key)
            .expect("We should have a status for the requested worker");
        Ok(res)
    }

    pub async fn get_workers_statuses<T>(
        &self,
        keys: &[T],
    ) -> Result<HashMap<String, WorkerStatus>, Error>
    where
        T: AsRef<str>,
    {
        match &self.inner {
            OsrdyneClientInternal::HTTPClient(client) => client.get_workers_statuses(keys).await,
            #[cfg(any(test, feature = "mock_client"))]
            OsrdyneClientInternal::MockClient(client) => client.get_workers_statuses(keys).await,
        }
    }
}

#[derive(Deserialize)]
struct OsrdyneStatusResponse {
    workers: HashMap<String, OsrdyneWorkerState>,
}

#[derive(Deserialize)]
struct OsrdyneWorkerState {
    status: OsrdyneWorkerStatus,
    worker_metadata: OsrdyneWorkerMetadata,
}

#[derive(Deserialize)]
struct OsrdyneWorkerMetadata {
    worker_key: String,
}

impl HTTPClient {
    pub async fn get_workers_statuses<T>(
        &self,
        keys: &[T],
    ) -> Result<HashMap<String, WorkerStatus>, Error>
    where
        T: AsRef<str>,
    {
        #[expect(unstable_name_collisions)] // intersperse
        let encoded_keys = keys
            .iter()
            .map(|key| format!("keys={}", urlencoding::encode(key.as_ref())))
            .intersperse("&".to_string())
            .collect::<String>();

        let url = self.base_url.join(&format!("/status?{encoded_keys}"))?;
        let response = self.client.get(url).send().await?;
        let response: OsrdyneStatusResponse = response.json().await?;

        // Build the map of worker status by key
        let mut statuses = HashMap::new();
        for worker in response.workers.into_values() {
            let status = worker.status.into();
            statuses
                .entry(worker.worker_metadata.worker_key)
                .and_modify(|s| *s = combine_worker_status(*s, status))
                .or_insert(status);
        }

        Ok(keys
            .iter()
            .map(|key| {
                let status = statuses
                    .get(key.as_ref())
                    .copied()
                    .map(WorkerStatus::from)
                    .unwrap_or(WorkerStatus::Unscheduled);
                (key.as_ref().to_owned(), status)
            })
            .collect::<HashMap<String, WorkerStatus>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_default_is_ready() {
        let client = OsrdyneClient::default_mock();
        let worker_key = "worker_key";
        let status = client.get_worker_status(worker_key).await.unwrap();
        assert_eq!(status, WorkerStatus::Ready);
    }

    #[tokio::test]
    async fn test_mock_get_worker_status() {
        let client = OsrdyneClient::mock()
            .with_status("started_key", WorkerStatus::Started)
            .with_status("unscheduled_key", WorkerStatus::Unscheduled)
            .build();
        let status = client.get_worker_status("started_key").await.unwrap();
        assert_eq!(status, WorkerStatus::Started);
        let status = client.get_worker_status("unscheduled_key").await.unwrap();
        assert_eq!(status, WorkerStatus::Unscheduled);
        let status = client.get_worker_status("unknown_key").await.unwrap();
        assert_eq!(status, WorkerStatus::Ready);
    }

    #[tokio::test]
    async fn test_mock_get_worker_statuses() {
        let client = OsrdyneClient::mock()
            .with_status("started_key", WorkerStatus::Started)
            .with_status("unscheduled_key", WorkerStatus::Unscheduled)
            .build();
        let status = client
            .get_workers_statuses(&["started_key", "unscheduled_key", "unknown_key"])
            .await
            .unwrap();
        assert_eq!(status["started_key"], WorkerStatus::Started);
        assert_eq!(status["unscheduled_key"], WorkerStatus::Unscheduled);
        assert_eq!(status["unknown_key"], WorkerStatus::Ready);
    }
}
