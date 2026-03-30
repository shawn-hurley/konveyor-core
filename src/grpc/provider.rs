//! Ergonomic Provider trait for implementing Konveyor providers.
//!
//! Instead of implementing the raw tonic `ProviderService` trait with
//! proto types, implement this trait with ergonomic Rust types and let
//! the SDK handle proto marshaling.

use crate::incident::{Incident, Location};
use std::fmt;

/// Error type for provider operations.
#[derive(Debug)]
pub struct ProviderError {
    pub message: String,
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ProviderError {}

impl ProviderError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for ProviderError {
    fn from(err: anyhow::Error) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

/// Information about a provider capability.
#[derive(Debug, Clone)]
pub struct CapabilityInfo {
    pub name: String,
}

/// Result of evaluating a condition.
#[derive(Debug)]
pub struct EvaluateResult {
    /// Whether any incidents were found.
    pub matched: bool,
    /// The incidents found.
    pub incidents: Vec<Incident>,
}

/// The ergonomic trait a provider implementor writes.
///
/// Implement this trait instead of the raw proto-generated `ProviderService`
/// trait. The SDK provides an adapter that converts between this trait and
/// the proto types, and generic server functions that handle gRPC setup.
///
/// # Example
///
/// ```ignore
/// struct MyProvider;
///
/// impl konveyor_core::grpc::Provider for MyProvider {
///     fn capabilities(&self) -> Vec<CapabilityInfo> {
///         vec![CapabilityInfo { name: "referenced".into() }]
///     }
///
///     fn init(&self, location: &str) -> Result<(), ProviderError> {
///         // Set up project root...
///         Ok(())
///     }
///
///     fn evaluate(&self, capability: &str, condition: &str)
///         -> Result<EvaluateResult, ProviderError>
///     {
///         // Scan files, return incidents...
///         Ok(EvaluateResult { matched: false, incidents: vec![] })
///     }
/// }
/// ```
pub trait Provider: Send + Sync + 'static {
    /// Return the list of capabilities this provider supports.
    fn capabilities(&self) -> Vec<CapabilityInfo>;

    /// Initialize the provider with a project location.
    fn init(&self, location: &str) -> Result<(), ProviderError>;

    /// Evaluate a condition and return matching incidents.
    fn evaluate(
        &self,
        capability: &str,
        condition: &str,
    ) -> Result<EvaluateResult, ProviderError>;

    /// Stop the provider.
    fn stop(&self) {
        // Default no-op
    }

    /// Get a code snippet for a file location.
    fn get_code_snip(
        &self,
        _uri: &str,
        _location: &Location,
        _context_lines: usize,
    ) -> Result<String, ProviderError> {
        Ok(String::new())
    }
}

// ── Adapter: Provider trait -> proto ProviderService ─────────────────────

use super::proto;
use super::proto::provider_code_location_service_server::ProviderCodeLocationService;
use super::proto::provider_service_server::ProviderService;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

type ProgressStream = Pin<Box<dyn Stream<Item = Result<proto::ProgressEvent, Status>> + Send>>;

/// Adapter that implements the proto `ProviderService` and
/// `ProviderCodeLocationService` traits for any `T: Provider`.
///
/// This is the bridge between the ergonomic `Provider` trait and the
/// raw gRPC protocol. Users should not need to interact with this
/// directly -- use `serve_tcp` / `serve_unix` instead.
pub struct ProviderAdapter<T: Provider> {
    inner: Arc<T>,
}

impl<T: Provider> ProviderAdapter<T> {
    pub fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }
}

#[tonic::async_trait]
impl<T: Provider> ProviderService for ProviderAdapter<T> {
    async fn capabilities(
        &self,
        _request: Request<()>,
    ) -> Result<Response<proto::CapabilitiesResponse>, Status> {
        let caps = self.inner.capabilities();
        let capabilities = caps
            .into_iter()
            .map(|c| proto::Capability {
                name: c.name,
                template_context: None,
            })
            .collect();

        Ok(Response::new(proto::CapabilitiesResponse { capabilities }))
    }

    async fn init(
        &self,
        request: Request<proto::Config>,
    ) -> Result<Response<proto::InitResponse>, Status> {
        let config = request.into_inner();
        let location = config.location.clone();

        tracing::info!("Initializing provider with location: {}", location);

        match self.inner.init(&location) {
            Ok(()) => Ok(Response::new(proto::InitResponse {
                error: String::new(),
                successful: true,
                id: 1,
                builtin_config: None,
            })),
            Err(e) => Ok(Response::new(proto::InitResponse {
                error: e.to_string(),
                successful: false,
                id: 0,
                builtin_config: None,
            })),
        }
    }

    async fn evaluate(
        &self,
        request: Request<proto::EvaluateRequest>,
    ) -> Result<Response<proto::EvaluateResponse>, Status> {
        let req = request.into_inner();

        tracing::info!(
            "Evaluate request: cap={}, condition_info={}",
            &req.cap,
            &req.condition_info
        );

        match self.inner.evaluate(&req.cap, &req.condition_info) {
            Ok(result) => {
                let incident_contexts: Vec<proto::IncidentContext> =
                    result.incidents.iter().map(proto::IncidentContext::from).collect();

                Ok(Response::new(proto::EvaluateResponse {
                    error: String::new(),
                    successful: true,
                    response: Some(proto::ProviderEvaluateResponse {
                        matched: result.matched,
                        incident_contexts,
                        template_context: None,
                    }),
                }))
            }
            Err(e) => Ok(Response::new(proto::EvaluateResponse {
                error: e.to_string(),
                successful: false,
                response: None,
            })),
        }
    }

    async fn stop(
        &self,
        _request: Request<proto::ServiceRequest>,
    ) -> Result<Response<()>, Status> {
        tracing::info!("Provider stopping");
        self.inner.stop();
        Ok(Response::new(()))
    }

    async fn get_dependencies(
        &self,
        _request: Request<proto::ServiceRequest>,
    ) -> Result<Response<proto::DependencyResponse>, Status> {
        Ok(Response::new(proto::DependencyResponse {
            successful: true,
            error: String::new(),
            file_dep: vec![],
        }))
    }

    async fn get_dependencies_dag(
        &self,
        _request: Request<proto::ServiceRequest>,
    ) -> Result<Response<proto::DependencyDagResponse>, Status> {
        Ok(Response::new(proto::DependencyDagResponse {
            successful: true,
            error: String::new(),
            file_dag_dep: vec![],
        }))
    }

    async fn notify_file_changes(
        &self,
        _request: Request<proto::NotifyFileChangesRequest>,
    ) -> Result<Response<proto::NotifyFileChangesResponse>, Status> {
        Ok(Response::new(proto::NotifyFileChangesResponse {
            error: String::new(),
        }))
    }

    async fn prepare(
        &self,
        _request: Request<proto::PrepareRequest>,
    ) -> Result<Response<proto::PrepareResponse>, Status> {
        Ok(Response::new(proto::PrepareResponse {
            error: String::new(),
        }))
    }

    type StreamPrepareProgressStream = ProgressStream;

    async fn stream_prepare_progress(
        &self,
        _request: Request<proto::PrepareProgressRequest>,
    ) -> Result<Response<Self::StreamPrepareProgressStream>, Status> {
        let stream = async_stream::stream! {
            yield Ok(proto::ProgressEvent {
                r#type: 0,
                provider_name: "provider".into(),
                files_processed: 0,
                total_files: 0,
            });
        };
        Ok(Response::new(Box::pin(stream)))
    }
}

#[tonic::async_trait]
impl<T: Provider> ProviderCodeLocationService for ProviderAdapter<T> {
    async fn get_code_snip(
        &self,
        request: Request<proto::GetCodeSnipRequest>,
    ) -> Result<Response<proto::GetCodeSnipResponse>, Status> {
        let req = request.into_inner();

        let code_location = req
            .code_location
            .ok_or_else(|| Status::invalid_argument("no code location sent"))?;
        let start_position = code_location
            .start_position
            .ok_or_else(|| Status::invalid_argument("no start position sent"))?;
        let end_position = code_location
            .end_position
            .ok_or_else(|| Status::invalid_argument("no end position sent"))?;

        let location = crate::incident::Location {
            start: crate::incident::Position {
                line: start_position.line as u32,
                character: start_position.character as u32,
            },
            end: crate::incident::Position {
                line: end_position.line as u32,
                character: end_position.character as u32,
            },
        };

        match self.inner.get_code_snip(&req.uri, &location, 3) {
            Ok(snip) => Ok(Response::new(proto::GetCodeSnipResponse { snip })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}
