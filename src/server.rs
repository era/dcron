use db::DB;
use dcron::public_server::{Public, PublicServer};
use dcron::{JobRequest, JobResponse, JobStatusRequest, JobStatusResponse, ScriptType};
use tonic::{transport::Server, Code, Request, Response, Status};
mod db;
mod job;
pub mod dcron {
    tonic::include_proto!("dcron"); // The string specified here must match the proto package name
}

#[derive(Debug, Default)]
pub struct DcronBasicServer {}

#[tonic::async_trait]
impl Public for DcronBasicServer {
    async fn new_job(&self, request: Request<JobRequest>) -> Result<Response<JobResponse>, Status> {
        let request = request.into_inner();
        let job = job::Job {
            name: request.name,
            time: request.time,
            job_type: request.job_type,
            timeout: request.timeout,
            script: request.location,
            active: true,
        };

        let db = match get_db().await {
            Ok(db) => db,
            _ => {
                return Err(Status::new(
                    Code::Internal,
                    "Could not connect to the database",
                ))
            }
        };

        // should check if it should update or insert
        // for now let's just insert

        let result = db.insert_if_not_exists(&job).await;

        if let Err(error) = result {
            println!("{:?}", error);
            return Err(Status::new(
                Code::Internal,
                "Error while trying to save object",
            ));
        }

        let reply = dcron::JobResponse {
            name: job.name.into(),
            error_code: 0,
            error_message: "".into(),
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

async fn get_db() -> Result<DB, Box<dyn std::error::Error>> {
    DB::connect("username", "password", "cluster_url").await
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
