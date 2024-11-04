pub mod api {
    tonic::include_proto!("statsig_forward_proxy");
}

pub use api::*;
