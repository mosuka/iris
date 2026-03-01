pub mod config;
mod context;
mod convert;
pub mod server;
mod service;

/// Generated protobuf/gRPC code.
pub mod proto {
    pub mod laurus {
        pub mod v1 {
            tonic::include_proto!("laurus.v1");
        }
    }
}
