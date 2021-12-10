extern crate clap;
use clap::{App, Arg, SubCommand};
use dcron::public_client::PublicClient;
use dcron::JobRequest;
use std::path::Path;
use std::str::FromStr;

mod storage;

pub mod dcron {
    tonic::include_proto!("dcron");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
                .index(4),
        )
        .arg(
            Arg::with_name("script")
                .short("s")
                .long("script")
                .takes_value(true)
                .index(5)
                .required(true),
        )
        .arg(
            Arg::with_name("name")
                .short("n")
                .long("name")
                .takes_value(true)
                .index(6)
                .required(true),
        )
        .get_matches();

    let file = Path::new(matches.value_of("script").unwrap());
    upload_file(&file).await?;

    let mut client = PublicClient::connect("http://[::1]:50051").await?; //TODO: change with ENV variables

    //TODO: upload file

    let request = tonic::Request::new(JobRequest {
        name: matches.value_of("name").unwrap().into(),
        time: matches.value_of("time").unwrap().into(),
        location: file.file_name().unwrap().to_str().unwrap().to_owned(), //TODO omg, so terrible
        timeout: <i32 as FromStr>::from_str(matches.value_of("timeout").unwrap()).unwrap(),
        update_if_exists: matches.is_present("update"),
        job_type: 0,
    });

    let response = client.new_job(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}

async fn upload_file(path: &Path) -> Result<(), anyhow::Error> {
    storage::Client::connect()
        .put(
            path.to_str().unwrap(),
            path.file_name().unwrap().to_str().unwrap(), // :C
        )
        .await?;
    Ok(())
}
