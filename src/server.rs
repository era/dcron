use chrono::Utc;
use db::DB;
use dcron::public_server::{Public, PublicServer};
use dcron::{
    DisableJobRequest, DisableJobResponse, JobRequest, JobResponse, JobStatusRequest,
    JobStatusResponse, ScriptType,
};
use once_cell::sync::OnceCell;
use std::env;
use tonic::{transport::Server, Code, Request, Response, Status};
mod config;
mod db;
mod heartbeat;
mod job;
mod storage;

pub mod dcron {
    tonic::include_proto!("dcron"); // The string specified here must match the proto package name
}

static CONFIG: OnceCell<config::Config> = OnceCell::new();

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
            updated_at: Utc::now().timestamp(),
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

        if request.update_if_exists {
            match db.disable_if_exist(&job.name).await {
                Err(error) => {
                    print!("Erro while disabling job: {:?}", error);
                    return Err(Status::new(Code::Internal, "Error while disabling old job"));
                }
                _ => (),
            };
        }

        let result = db.insert_if_not_exist(&job).await;

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
        let request = request.into_inner();

        let db = match get_db().await {
            Ok(db) => db,
            _ => {
                return Err(Status::new(
                    Code::Internal,
                    "Could not connect to the database",
                ))
            }
        };

        let job = db.find_job(&request.name, true).await;

        let job = match job {
            Some(job) => job,
            None => {
                return Err(Status::new(
                    Code::NotFound,
                    "Error while trying to get object",
                ))
            }
            _ => {
                return Err(Status::new(
                    Code::Internal,
                    "Error while trying to get object",
                ))
            }
        };

        let reply = dcron::JobStatusResponse {
            name: job.name,
            timeout: job.timeout,
            time: job.time,
            error_code: 0,
            job_type: job.job_type,
            location: job.script,
            executions: vec![], //TODO
        };

        Ok(Response::new(reply))
    }

    async fn disable_job(
        &self,
        request: Request<DisableJobRequest>,
    ) -> Result<Response<DisableJobResponse>, Status> {
        let request = request.into_inner();

        let db = match get_db().await {
            Ok(db) => db,
            _ => {
                return Err(Status::new(
                    Code::Internal,
                    "Could not connect to the database",
                ))
            }
        };

        let job = db.find_job(&request.name, true).await;

        let response = match job {
            Some(_job) => db.disable_if_exist(&request.name),
            None => {
                return Err(Status::new(
                    Code::NotFound,
                    "Error while trying to get object",
                ))
            }
            _ => {
                return Err(Status::new(
                    Code::Internal,
                    "Error while trying to get object",
                ))
            }
        };

        let reply = DisableJobResponse {
            error_code: 0,
            error_message: "".into(),
        };

        match response.await {
            Ok(_) => Ok(Response::new(reply)),
            _ => Err(Status::new(
                Code::Internal,
                "Error while trying to update object",
            )),
        }
    }
}

async fn get_db() -> Result<DB, Box<dyn std::error::Error>> {
    // TODO: Should keep a pool of connections
    let config = match CONFIG.get() {
        Some(config) => config,
        _ => return Err("Could not get a config object".into()),
    };

    db::get_db(config).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let server = DcronBasicServer::default();

    let config_file = env::var("DCRON_CONFIG").unwrap_or("app.toml".into());

    let config = config::Config::from(&config_file);
    let config = config.expect("Error while trying to read configuration file");

    CONFIG.set(config);

    Server::builder()
        .add_service(PublicServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
