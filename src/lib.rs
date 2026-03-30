//! Shared types, gRPC protocol, and provider SDK for the Konveyor migration ecosystem.
//!
//! This crate provides the canonical types and protocol definitions used by
//! Konveyor-compatible migration tools:
//!
//! - [`incident`] -- Incident types (`Incident`, `Position`, `Location`)
//!   representing matched violations in source code.
//! - [`report`] -- Analysis output types (`RuleSet`, `Violation`, `Category`)
//!   matching the Konveyor analyzer-lsp output format.
//! - [`rule`] -- Rule definition types (`KonveyorRuleset`, `KonveyorRule`,
//!   `KonveyorCondition`) for the YAML rule format kantra consumes.
//! - [`fix`] -- Fix strategy types (`FixStrategyEntry`, `MappingEntry`,
//!   `FixConfidence`) for the `fix-strategies.json` bridge between analyzers
//!   and fix engines.
//! - [`grpc`] -- (behind the `grpc` feature) Proto-generated gRPC types,
//!   the ergonomic `Provider` trait, and generic server functions.
//!
//! # Features
//!
//! - **`grpc`** -- Enables the gRPC module with tonic/prost dependencies,
//!   the `Provider` trait, `ProviderAdapter`, and `serve_tcp`/`serve_unix`.
//! - **`generate-proto`** -- Re-generates proto code from `provider.proto`.

pub mod fix;
pub mod incident;
pub mod report;
pub mod rule;

#[cfg(feature = "grpc")]
pub mod grpc;
