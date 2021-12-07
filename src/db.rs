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
    pub async fn connect(
        username: &str,
        password: &str,
        cluster_url: &str,
    ) -> Result<DB, Box<dyn std::error::Error>> {
        let url = format!(
            "mongodb+srv://{}:{}@{}/dcron?w=majority",
            username, password, cluster_url
        );

        let mut client_options = ClientOptions::parse(url).await?;

        let client = Client::with_options(client_options)?;
        client
            .database("admin")
            .run_command(doc! {"ping": 1}, None)
            .await?;

        Ok(DB {
            client: Some(DBClient::MongoDB(client)),
        })
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

    pub async fn insert_if_not_exists(self: &Self, job: job::Job) -> Result<(), Box<dyn Error>> {
        if let Some(job) = self.find_job(&job.name).await {
            return Err("Job already in database".into());
        } else {
            if let Some(database) = self.get_db() {
                let collection = database.collection::<job::Job>("jobs");
                // collection.insert_one(doc! { job }, None).await?;

                return Ok(());
            }
        }
        Err("Unknown error while saving the Job".into())
    }
}
