//! Generic gRPC server transport setup.
//!
//! Provides `serve_tcp` and `serve_unix` functions that work with any
//! `Provider` implementation via the `ProviderAdapter`.

use super::proto::provider_code_location_service_server::ProviderCodeLocationServiceServer;
use super::proto::provider_service_server::ProviderServiceServer;
use super::provider::{Provider, ProviderAdapter};
use std::sync::Arc;
use tonic::transport::Server;

/// Start a gRPC server on a TCP port for any `Provider` implementation.
pub async fn serve_tcp<T: Provider>(provider: Arc<T>, port: u16) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port).parse()?;
    tracing::info!("Provider listening on {}", addr);

    let adapter = Arc::new(ProviderAdapter::new(provider));

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(super::proto::FILE_DESCRIPTOR_SET)
        .build_v1alpha()?;

    Server::builder()
        .add_service(ProviderServiceServer::from_arc(adapter.clone()))
        .add_service(ProviderCodeLocationServiceServer::from_arc(adapter))
        .add_service(reflection)
        .serve(addr)
        .await?;

    Ok(())
}

/// Start a gRPC server on a Unix domain socket for any `Provider` implementation.
#[cfg(unix)]
pub async fn serve_unix<T: Provider>(provider: Arc<T>, socket_path: &str) -> anyhow::Result<()> {
    use tokio::net::UnixListener;
    use tokio_stream::wrappers::UnixListenerStream;

    // Remove existing socket file
    let _ = std::fs::remove_file(socket_path);

    let uds = UnixListener::bind(socket_path)?;
    let uds_stream = UnixListenerStream::new(uds);
    tracing::info!("Provider listening on unix://{}", socket_path);

    let adapter = Arc::new(ProviderAdapter::new(provider));

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(super::proto::FILE_DESCRIPTOR_SET)
        .build_v1alpha()?;

    Server::builder()
        .add_service(ProviderServiceServer::from_arc(adapter.clone()))
        .add_service(ProviderCodeLocationServiceServer::from_arc(adapter))
        .add_service(reflection)
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}
