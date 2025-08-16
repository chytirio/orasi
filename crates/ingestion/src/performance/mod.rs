//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Performance optimization modules for high-throughput telemetry ingestion
//!
//! This module provides performance optimization components including:
//! - Connection pooling for HTTP and gRPC
//! - Memory-efficient batch processing
//! - Backpressure handling
//! - Performance monitoring and metrics

pub mod batch_processor;
pub mod connection_pool;

pub use connection_pool::{
    CombinedPoolStats, ConnectionPoolConfig, ConnectionPoolManager, GrpcConnectionPool,
    HttpConnectionPool, PoolError, PoolStats,
};

pub use batch_processor::{
    BackpressureController, BackpressureStats, BatchProcessorConfig, BatchProcessorStats,
    BatchStream, MemoryEfficientBatchProcessor, StreamingBatchProcessor,
};

/// Performance optimization manager
pub struct PerformanceManager {
    connection_pool_manager: ConnectionPoolManager,
    batch_processor: MemoryEfficientBatchProcessor,
    backpressure_controller: BackpressureController,
}

impl PerformanceManager {
    /// Create new performance manager
    pub fn new(
        pool_config: ConnectionPoolConfig,
        batch_config: BatchProcessorConfig,
        backpressure_threshold: f64,
    ) -> Self {
        let connection_pool_manager = ConnectionPoolManager::new(pool_config);
        let batch_processor = MemoryEfficientBatchProcessor::new(batch_config);
        let backpressure_controller = BackpressureController::new(backpressure_threshold);

        Self {
            connection_pool_manager,
            batch_processor,
            backpressure_controller,
        }
    }

    /// Start performance optimization services
    pub async fn start(&self) -> BridgeResult<()> {
        // Start connection pool cleanup
        self.connection_pool_manager.start_cleanup_task().await;

        info!("Performance manager started successfully");
        Ok(())
    }

    /// Get connection pool manager
    pub fn connection_pool_manager(&self) -> &ConnectionPoolManager {
        &self.connection_pool_manager
    }

    /// Get batch processor
    pub fn batch_processor(&self) -> &MemoryEfficientBatchProcessor {
        &self.batch_processor
    }

    /// Get backpressure controller
    pub fn backpressure_controller(&self) -> &BackpressureController {
        &self.backpressure_controller
    }

    /// Get combined performance statistics
    pub async fn get_stats(&self) -> PerformanceStats {
        let pool_stats = self.connection_pool_manager.get_stats().await;
        let batch_stats = self.batch_processor.get_stats().await;
        let backpressure_stats = self.backpressure_controller.get_stats().await;

        PerformanceStats {
            connection_pools: pool_stats,
            batch_processing: batch_stats,
            backpressure: backpressure_stats,
        }
    }
}

/// Combined performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub connection_pools: CombinedPoolStats,
    pub batch_processing: BatchProcessorStats,
    pub backpressure: BackpressureStats,
}

use bridge_core::BridgeResult;
use tracing::info;
