//! Generated gRPC code from proto files

pub mod bridge {
    pub mod grpc {
        tonic::include_proto!("bridge.grpc");
    }
}

pub mod grpc {
    pub mod health {
        pub mod v1 {
            tonic::include_proto!("grpc.health.v1");
        }
    }
}

pub use bridge::grpc::*;
pub use grpc::health::v1::*;
