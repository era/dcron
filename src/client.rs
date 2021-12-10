extern crate clap;
use clap::{App, Arg, SubCommand};
use dcron::public_client::PublicClient;
use dcron::JobRequest;

pub mod dcron {
    tonic::include_proto!("dcron");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
                .index(4)
                .required(true),
        )
        .arg(
            Arg::with_name("script")
                .short("s")
                .long("script")
                .takes_value(true)
                .index(5)
                .required(true),
        )
        .get_matches();

    let mut client = PublicClient::connect("http://[::1]:50051").await?; //TODO: change with ENV variables

    let request = tonic::Request::new(JobRequest {
        //TODO get this from clap
        name: "Tonic".into(),
        time: "* * * * 1".into(),
        location: "service.py".into(),
        timeout: 0,
        update_if_exists: true,
        job_type: 0,
    });

    let response = client.new_job(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
