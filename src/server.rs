use dcron::public_server::{Public, PublicServer};
use dcron::{JobRequest, JobResponse, JobStatusRequest, JobStatusResponse, ScriptType};
use tonic::{transport::Server, Request, Response, Status};

pub mod dcron {
    tonic::include_proto!("dcron"); // The string specified here must match the proto package name
}

#[derive(Debug, Default)]
pub struct DcronBasicServer {}

#[tonic::async_trait]
impl Public for DcronBasicServer {
    async fn new_job(&self, request: Request<JobRequest>) -> Result<Response<JobResponse>, Status> {
        let reply = dcron::JobResponse {
            name: "test".to_string(),
            error_code: 0,
            error_message: "".to_string(),
        };

        Ok(Response::new(reply))
    }

    async fn get_job(
        &self,
        request: Request<JobStatusRequest>,
    ) -> Result<Response<JobStatusResponse>, Status> {
        let reply = dcron::JobStatusResponse {
            error_code: 0,
            job_type: 0, // how to ScriScriptType::Python.,
            location: "".to_string(),
            executions: vec![],
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let server = DcronBasicServer::default();

    Server::builder()
        .add_service(PublicServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
