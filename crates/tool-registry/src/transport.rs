use std::path::PathBuf;

use rmcp::service::{RoleClient, RunningService};
use rmcp::ServiceExt;
use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;

use crate::registry_error::RegistryError;

#[derive(Debug, PartialEq)]
pub(crate) enum TransportTarget {
    Tcp { host: String, port: u16 },
    #[cfg(unix)]
    Unix { path: PathBuf },
}

const UNIX_SCHEME: &str = "mcp+unix://";
const TCP_SCHEME: &str = "mcp://";

pub(crate) fn parse_endpoint(endpoint: &str) -> Result<TransportTarget, RegistryError> {
    if endpoint.starts_with(UNIX_SCHEME) {
        parse_unix_endpoint(endpoint)
    } else if endpoint.starts_with(TCP_SCHEME) {
        parse_tcp_endpoint(endpoint)
    } else {
        Err(RegistryError::ConnectionFailed {
            endpoint: endpoint.to_string(),
            reason: "unsupported scheme; expected mcp:// or mcp+unix://".to_string(),
        })
    }
}

fn parse_tcp_endpoint(endpoint: &str) -> Result<TransportTarget, RegistryError> {
    let authority = &endpoint[TCP_SCHEME.len()..];
    let (host, port_str) = authority.rsplit_once(':').ok_or_else(|| {
        RegistryError::ConnectionFailed {
            endpoint: endpoint.to_string(),
            reason: "missing port in TCP endpoint".to_string(),
        }
    })?;

    if host.is_empty() {
        return Err(RegistryError::ConnectionFailed {
            endpoint: endpoint.to_string(),
            reason: "empty host in TCP endpoint".to_string(),
        });
    }

    let port: u16 = port_str.parse().map_err(|_| RegistryError::ConnectionFailed {
        endpoint: endpoint.to_string(),
        reason: format!("invalid port '{}'; expected a number 0-65535", port_str),
    })?;

    Ok(TransportTarget::Tcp {
        host: host.to_string(),
        port,
    })
}

#[cfg(unix)]
fn parse_unix_endpoint(endpoint: &str) -> Result<TransportTarget, RegistryError> {
    let path_str = &endpoint[UNIX_SCHEME.len()..];
    if path_str.is_empty() {
        return Err(RegistryError::ConnectionFailed {
            endpoint: endpoint.to_string(),
            reason: "empty path in Unix socket endpoint".to_string(),
        });
    }
    Ok(TransportTarget::Unix {
        path: PathBuf::from(path_str),
    })
}

#[cfg(not(unix))]
fn parse_unix_endpoint(endpoint: &str) -> Result<TransportTarget, RegistryError> {
    Err(RegistryError::ConnectionFailed {
        endpoint: endpoint.to_string(),
        reason: "Unix sockets are not supported on this platform".to_string(),
    })
}

pub(crate) async fn connect_transport(
    endpoint: &str,
) -> Result<RunningService<RoleClient, ()>, RegistryError> {
    let target = parse_endpoint(endpoint)?;
    match target {
        TransportTarget::Tcp { host, port } => connect_tcp(endpoint, &host, port).await,
        #[cfg(unix)]
        TransportTarget::Unix { path } => connect_unix(endpoint, &path).await,
    }
}

async fn connect_tcp(
    endpoint: &str,
    host: &str,
    port: u16,
) -> Result<RunningService<RoleClient, ()>, RegistryError> {
    let stream = TcpStream::connect(format!("{host}:{port}"))
        .await
        .map_err(|e| RegistryError::ConnectionFailed {
            endpoint: endpoint.to_string(),
            reason: e.to_string(),
        })?;
    ().serve(stream)
        .await
        .map_err(|e| RegistryError::ConnectionFailed {
            endpoint: endpoint.to_string(),
            reason: e.to_string(),
        })
}

#[cfg(unix)]
async fn connect_unix(
    endpoint: &str,
    path: &std::path::Path,
) -> Result<RunningService<RoleClient, ()>, RegistryError> {
    let stream = UnixStream::connect(path)
        .await
        .map_err(|e| RegistryError::ConnectionFailed {
            endpoint: endpoint.to_string(),
            reason: e.to_string(),
        })?;
    ().serve(stream)
        .await
        .map_err(|e| RegistryError::ConnectionFailed {
            endpoint: endpoint.to_string(),
            reason: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tcp_endpoint_with_hostname_and_port() {
        let result = parse_endpoint("mcp://localhost:7001").unwrap();
        assert_eq!(
            result,
            TransportTarget::Tcp {
                host: "localhost".into(),
                port: 7001,
            }
        );
    }

    #[test]
    fn parses_tcp_endpoint_with_ip_address_and_port() {
        let result = parse_endpoint("mcp://127.0.0.1:8080").unwrap();
        assert_eq!(
            result,
            TransportTarget::Tcp {
                host: "127.0.0.1".into(),
                port: 8080,
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn parses_unix_endpoint_with_absolute_path() {
        let result = parse_endpoint("mcp+unix:///var/run/tool.sock").unwrap();
        assert_eq!(
            result,
            TransportTarget::Unix {
                path: PathBuf::from("/var/run/tool.sock"),
            }
        );
    }

    #[test]
    fn rejects_missing_scheme() {
        let err = parse_endpoint("localhost:7001").unwrap_err();
        match &err {
            RegistryError::ConnectionFailed { reason, .. } => {
                assert!(
                    reason.contains("unsupported scheme"),
                    "expected reason to contain 'unsupported scheme', got: {reason}"
                );
            }
            other => panic!("expected ConnectionFailed, got: {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_scheme() {
        let err = parse_endpoint("http://localhost:7001").unwrap_err();
        match &err {
            RegistryError::ConnectionFailed { reason, .. } => {
                assert!(
                    reason.contains("unsupported scheme"),
                    "expected reason to contain 'unsupported scheme', got: {reason}"
                );
            }
            other => panic!("expected ConnectionFailed, got: {other:?}"),
        }
    }

    #[test]
    fn rejects_tcp_endpoint_missing_port() {
        let err = parse_endpoint("mcp://localhost").unwrap_err();
        match &err {
            RegistryError::ConnectionFailed { reason, .. } => {
                assert!(
                    reason.contains("missing port"),
                    "expected reason to contain 'missing port', got: {reason}"
                );
            }
            other => panic!("expected ConnectionFailed, got: {other:?}"),
        }
    }

    #[test]
    fn rejects_tcp_endpoint_invalid_port() {
        let err = parse_endpoint("mcp://localhost:notaport").unwrap_err();
        match &err {
            RegistryError::ConnectionFailed { reason, .. } => {
                assert!(
                    reason.contains("invalid port"),
                    "expected reason to contain 'invalid port', got: {reason}"
                );
            }
            other => panic!("expected ConnectionFailed, got: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_unix_endpoint_with_no_path() {
        let err = parse_endpoint("mcp+unix://").unwrap_err();
        match &err {
            RegistryError::ConnectionFailed { reason, .. } => {
                assert!(
                    reason.contains("empty path"),
                    "expected reason to contain 'empty path', got: {reason}"
                );
            }
            other => panic!("expected ConnectionFailed, got: {other:?}"),
        }
    }
}
