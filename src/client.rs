extern crate clap;
use clap::{App, Arg, SubCommand};
use dcron::public_client::PublicClient;
use dcron::{JobRequest, ScriptType};
use once_cell::sync::OnceCell;
use std::env;
use std::path::Path;
use std::str::FromStr;

mod config;
mod storage;

pub mod dcron {
    tonic::include_proto!("dcron");
}

static CONFIG: OnceCell<config::Config> = OnceCell::new();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    //TODO subcommand for creating/updating and for disabling job
    let matches = App::new("dcron_client")
        .version("0.0.1")
        .author("Elias Granja <me@elias.sh>")
        .about("Sets up a script to be run at a DCRON instance")
        .arg(
            Arg::with_name("time")
                .short("t")
                .long("time")
                .value_name("CRON_SYNTAX")
                .help("Sets the frequence you want the job to be run")
                .required(true)
                .index(1)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("timeout")
                .short("o")
                .long("timeout")
                .value_name("TIMEOUT")
                .help("Defines the timeout for the job, if zero no timeout will be set")
                .index(2)
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("type")
                .short("p")
                .long("type")
                .value_name("TYPE")
                .takes_value(true)
                .index(3)
                .required(true),
        )
        .arg(
            Arg::with_name("update")
                .short("e")
                .long("update")
                .takes_value(false)
                .index(6),
        )
        .arg(
            Arg::with_name("script")
                .short("s")
                .long("script")
                .takes_value(true)
                .index(4)
                .required(true),
        )
        .arg(
            Arg::with_name("name")
                .short("n")
                .long("name")
                .takes_value(true)
                .index(5)
                .required(true),
        )
        .get_matches();

    let file = Path::new(matches.value_of("script").unwrap());
    upload_file(&file).await?;

    let mut client = PublicClient::connect("http://[::1]:50051").await?; //TODO: change with ENV variables

    upload_file(file).await?;

    let request = tonic::Request::new(JobRequest {
        name: matches.value_of("name").unwrap().into(),
        time: matches.value_of("time").unwrap().into(),
        location: file.file_name().unwrap().to_str().unwrap().to_owned(), //TODO omg, so terrible
        timeout: <i32 as FromStr>::from_str(matches.value_of("timeout").unwrap()).unwrap(),
        update_if_exists: matches.is_present("update"),
        job_type: job_type(matches.value_of("type").unwrap()),
    });

    let response = client.new_job(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}

fn job_type(user_type: &str) -> i32 {
    match user_type {
        "python" => ScriptType::Python as i32,
        "ruby" => ScriptType::Ruby as i32,
        _ => panic!("Script type not supported"),
    }
}

async fn upload_file(path: &Path) -> Result<(), anyhow::Error> {
    let config = match CONFIG.get() {
        Some(config) => config,
        _ => return Err(anyhow::anyhow!("Could not get a config object")),
    };

    let minio_config = match &config.minio {
        Some(minio_config) => minio_config,
        _ => panic!("No configuration for minio"),
    };

    storage::Client::connect(&minio_config)
        .put(
            path.to_str().unwrap(),
            path.file_name().unwrap().to_str().unwrap(), //TODO: the problem here is that two users may define different scripts with same name
                                                         // names should be prefixed with something unique to avoid this
                                                         // and we should verify if we are
                                                         // coliding before pushing the file
        )
        .await?;
    Ok(())
}
