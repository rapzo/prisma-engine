mod connection;
mod mysql;
mod postgresql;
mod sqlite;
mod transaction;

pub(crate) mod operations;

use async_trait::async_trait;
use connector_interface::{error::ConnectorError, Connector};
use datamodel::Source;

pub use mysql::*;
pub use postgresql::*;
pub use sqlite::*;

#[async_trait]
pub trait FromSource {
    async fn from_source(source: &dyn Source) -> crate::Result<Self>
    where
        Self: Connector + Sized;
}

async fn catch<O>(
    connection_info: &quaint::prelude::ConnectionInfo,
    fut: impl std::future::Future<Output = Result<O, crate::SqlError>>,
) -> Result<O, ConnectorError> {
    match fut.await {
        Ok(o) => Ok(o),
        Err(err) => Err(err.into_connector_error(connection_info)),
    }
}
