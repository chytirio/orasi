//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!
//! gRPC services for Bridge API

pub mod bridge;
pub mod health;
pub mod middleware;
pub mod server;
pub mod telemetry;

// Re-export main components for convenience
pub use bridge::BridgeGrpcService;
pub use health::GrpcHealthService;
pub use server::{create_grpc_server, GrpcServer};
pub use telemetry::TelemetryGrpcService;
