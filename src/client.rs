use dcron::public_client::PublicClient;
use dcron::JobRequest;

pub mod dcron {
    tonic::include_proto!("dcron");
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = PublicClient::connect("http://[::1]:50051").await?; //TODO: change with ENV variables

    let request = tonic::Request::new(JobRequest { //TODO get this from clap 
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
