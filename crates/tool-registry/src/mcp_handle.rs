use std::sync::Arc;

use rmcp::service::{Peer, QuitReason, RoleClient, RunningService};
use tokio::task::JoinError;

/// Newtype wrapping an rmcp client session.
///
/// Provides a focused API surface over the full `RunningService` type,
/// exposing only the peer accessor and a graceful shutdown method.
///
/// Cloneable via internal `Arc`, so multiple owners can share a handle.
#[derive(Debug)]
pub struct McpHandle {
    inner: Arc<RunningService<RoleClient, ()>>,
}

impl Clone for McpHandle {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl McpHandle {
    pub fn new(service: RunningService<RoleClient, ()>) -> Self {
        Self {
            inner: Arc::new(service),
        }
    }

    pub fn peer(&self) -> &Peer<RoleClient> {
        self.inner.peer()
    }

    pub async fn shutdown(self) -> Result<QuitReason, JoinError> {
        let service = Arc::try_unwrap(self.inner)
            .expect("cannot shutdown McpHandle while other references exist");
        service.cancel().await
    }
}
