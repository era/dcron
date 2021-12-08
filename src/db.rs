use crate::job;
use mongodb::{bson::doc, options::ClientOptions, Client, Database};

use std::error::Error;

pub enum DBClient {
    MongoDB(mongodb::Client),
}

pub struct DB {
    client: Option<DBClient>,
}

impl DB {
    pub fn connection_url(username: &str, password: &str, cluster_url: &str) -> String {
        format!(
            "mongodb+srv://{}:{}@{}/dcron?w=majority",
            username, password, cluster_url
        )
    }
    pub async fn connect(url: String) -> Result<DB, Box<dyn std::error::Error>> {
        let client_options = ClientOptions::parse(url).await?;

        let client = Client::with_options(client_options)?;
        client
            .database("admin")
            .run_command(doc! {"ping": 1}, None)
            .await?;

        Ok(DB {
            client: Some(DBClient::MongoDB(client)),
        })
    }

    pub async fn local_connection() -> Result<DB, Box<dyn std::error::Error>> {
        DB::connect("mongodb://localhost:27017/dcron".into()).await
    }

    fn get_db(self: &Self) -> Option<Database> {
        if let Some(DBClient::MongoDB(client)) = &self.client {
            return Some(client.database("dcron"));
        }
        None
    }

    pub async fn find_job(self: &Self, name: &str) -> Option<job::Job> {
        if let Some(database) = self.get_db() {
            let collection = database.collection::<job::Job>("jobs");

            let job = collection.find_one(doc! { "name": name }, None).await;

            match job {
                Ok(Some(job)) => return Some(job),
                _ => return None,
            }
        }
        None
    }

    pub async fn insert_if_not_exists(self: &Self, job: &job::Job) -> Result<(), Box<dyn Error>> {
        if let Some(_job) = self.find_job(&job.name).await {
            return Err("Job already in database".into());
        } else {
            if let Some(database) = self.get_db() {
                let collection = database.collection("jobs");
                collection
                    .insert_one(
                        doc! { "name": &job.name, "script_type": &job.script, "script": &job.script, "time": &job.time, "timeout": &job.timeout  },
                        None,
                    )
                    .await?;

                return Ok(());
            }
        }
        Err("Unknown error while saving the Job".into())
    }
}
