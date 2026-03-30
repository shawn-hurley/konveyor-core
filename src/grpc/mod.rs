//! Konveyor provider gRPC protocol and SDK.
//!
//! This module provides:
//! - Proto-generated types and service traits from `provider.proto`
//! - An ergonomic `Provider` trait that hides proto complexity
//! - `From` implementations for converting between ergonomic and proto types
//! - Generic server functions (`serve_tcp`, `serve_unix`) for any `Provider` impl

pub mod provider;
pub mod server;

/// Generated protobuf code from `provider.proto`.
///
/// Contains the raw tonic/prost types: `ProviderService` trait,
/// `ProviderCodeLocationService` trait, and all message types.
/// Prefer using the ergonomic `Provider` trait and `crate::incident` types
/// instead of these proto types directly.
pub mod proto {
    include!("generated/provider.rs");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        include_bytes!("generated/provider_service_descriptor.bin");
}

// ── Proto <-> Ergonomic type conversions ────────────────────────────────

use crate::incident;

/// Convert a `serde_json::Value` to a `prost_types::Value`.
fn json_to_prost_value(v: &serde_json::Value) -> prost_types::Value {
    match v {
        serde_json::Value::String(s) => prost_types::Value {
            kind: Some(prost_types::value::Kind::StringValue(s.clone())),
        },
        serde_json::Value::Number(n) => prost_types::Value {
            kind: Some(prost_types::value::Kind::NumberValue(
                n.as_f64().unwrap_or_default(),
            )),
        },
        serde_json::Value::Bool(b) => prost_types::Value {
            kind: Some(prost_types::value::Kind::BoolValue(*b)),
        },
        serde_json::Value::Null => prost_types::Value {
            kind: Some(prost_types::value::Kind::NullValue(0)),
        },
        _ => prost_types::Value {
            kind: Some(prost_types::value::Kind::StringValue(v.to_string())),
        },
    }
}

impl From<&incident::Incident> for proto::IncidentContext {
    fn from(incident: &incident::Incident) -> Self {
        let variables = if incident.variables.is_empty() {
            None
        } else {
            let fields = incident
                .variables
                .iter()
                .map(|(k, v)| {
                    let prost_value = json_to_prost_value(v);
                    (k.clone(), prost_value)
                })
                .collect();
            Some(prost_types::Struct { fields })
        };

        proto::IncidentContext {
            file_uri: incident.file_uri.clone(),
            effort: incident.effort,
            code_location: incident.code_location.as_ref().map(|loc| proto::Location {
                start_position: Some(proto::Position {
                    line: loc.start.line as f64,
                    character: loc.start.character as f64,
                }),
                end_position: Some(proto::Position {
                    line: loc.end.line as f64,
                    character: loc.end.character as f64,
                }),
            }),
            line_number: incident.line_number.map(|n| n as i64),
            variables,
            links: incident
                .links
                .iter()
                .map(|l| proto::ExternalLink {
                    url: l.url.clone(),
                    title: l.title.clone(),
                })
                .collect(),
            is_dependency_incident: incident.is_dependency_incident,
        }
    }
}

impl From<&incident::Position> for proto::Position {
    fn from(pos: &incident::Position) -> Self {
        proto::Position {
            line: pos.line as f64,
            character: pos.character as f64,
        }
    }
}

impl From<&incident::Location> for proto::Location {
    fn from(loc: &incident::Location) -> Self {
        proto::Location {
            start_position: Some(proto::Position::from(&loc.start)),
            end_position: Some(proto::Position::from(&loc.end)),
        }
    }
}

impl From<&incident::ExternalLink> for proto::ExternalLink {
    fn from(link: &incident::ExternalLink) -> Self {
        proto::ExternalLink {
            url: link.url.clone(),
            title: link.title.clone(),
        }
    }
}
