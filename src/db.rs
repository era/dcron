use mongodb::{bson::doc, options::ClientOptions, Client};



pub enum DBClient {
    MongoDB(mongodb::Client),
}

pub struct DB {client: DBClient}

impl for DB {
    pub connect(username: &str, password: &str, cluster_url: &str) -> Result<DB> {
        let url = format!("mongodb+srv://{}:{}@{}/dcron?w=majority", username, password, cluster_url);

        let mut client_options = ClientOptions::parse(url).await?;


        let client = Client::with_options(client_options)?;
        client
            .database("admin")
            .run_command(doc! {"ping": 1}, None)
            .await?;

        OK(DB{client: DBClient::MongoDB(client)})
    }

}
