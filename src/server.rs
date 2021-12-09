use db::DB;
use dcron::public_server::{Public, PublicServer};
use dcron::{JobRequest, JobResponse, JobStatusRequest, JobStatusResponse, ScriptType};
use once_cell::sync::OnceCell;
use std::env;
use tonic::{transport::Server, Code, Request, Response, Status};
mod config;
mod db;
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
    // TODO: Should keep a pool of connections
    let config = match CONFIG.get() {
        Some(config) => config,
        _ => return Err("Could not get a config object".into()),
    };
    if let Some(db_config) = &config.database {
        let url = DB::connection_url(
            &db_config.username,
            &db_config.password,
            &db_config.cluster_url,
        );

        return DB::connect(url).await;
    } else {
        return DB::local_connection().await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let server = DcronBasicServer::default();

    let config_file = match env::var("DCRON_CONFIG") {
        Ok(config_file) => config_file,
        _ => "app.toml".into(),
    };

    let config = config::Config::from(&config_file);

    let config = match config {
        Ok(config) => config,
        _ => panic!("Error while trying to read configuration file"),
    };

    CONFIG.set(config);

    Server::builder()
        .add_service(PublicServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
