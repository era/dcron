use db::DB;
use dcron::public_server::{Public, PublicServer};
use dcron::{JobRequest, JobResponse, JobStatusRequest, JobStatusResponse, ScriptType};
use std::env;
use tonic::{transport::Server, Code, Request, Response, Status};
mod config;
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

async fn get_db() -> Result<DB, anyhow::Error> {
    // TODO: Should keep a pool of connections
    // TODO: TRansfer the config things to a sync method
    let config_file = match env::var("DCRON_CONFIG") {
        Ok(config_file) => config_file,
        _ => "app.toml".into(),
    };

    let mut config = config::Config::from(&config_file);

    let config = match config {
        Ok(config) => config,
        _ => panic!("Error while trying to read configuration file"),
    };

    if let Some(db_config) = config.database {
        let url = DB::connection_url(
            &db_config.username,
            &db_config.password,
            &db_config.cluster_url,
        );

        let result = tokio::task::spawn_blocking(|| DB::connect(url)).await;
    } else {
        let result = tokio::task::spawn_blocking(|| DB::local_connection()).await;
    }

    if let Ok(connection) = result {
        return Ok(connection);
    } else {
        return anyhow::Error("Could not connect to the database");
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
