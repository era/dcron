use tonic::{transport::Server, Request, Response, Status};

use dcron::public_server::{Public, PublicServer};
use dcron::{JobRequest, JobResponse};

pub mod dcron {
    tonic::include_proto!("dcron"); // The string specified here must match the proto package name
}
