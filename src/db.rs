use crate::{config::Config, job};
use chrono::Utc;
use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client, Collection, Database,
};
use std::error::Error;

pub enum DBClient {
    MongoDB(mongodb::Client),
}

pub struct DB {
    client: Option<DBClient>,
}

//TODO transform this in a trait/impl
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

    pub async fn send_heartbeat(self: &Self, server_name: &str) -> Result<(), anyhow::Error> {
        if let Some(database) = self.get_db() {
            let collection = database.collection("heartbeats");
            collection
                .insert_one(
                    doc! { "server": server_name, "timestamp": Utc::now().timestamp()},
                    None,
                )
                .await?;

            return Ok(());
        }
        Err(anyhow::anyhow!("Unknown error while saving heartbeats"))
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

    pub async fn find_job(self: &Self, name: &str, active: bool) -> Option<job::Job> {
        if let Some(database) = self.get_db() {
            let collection = database.collection::<job::Job>("jobs");

            let job = collection
                .find_one(doc! { "name": name, "active": active }, None)
                .await;

            match job {
                Ok(Some(job)) => return Some(job),
                _ => return None,
            }
        }
        None
    }
    pub async fn find_all_active(self: &Self) -> Result<Vec<job::Job>, anyhow::Error> {
        if let Some(database) = self.get_db() {
            let collection = database.collection::<job::Job>("jobs");
            let jobs_cursor = collection.find(doc! {"active": true}, None).await?;
            return Ok(jobs_cursor.try_collect().await?);
        } else {
            return Err(anyhow::anyhow!("Could not connect to the database"));
        }
    }

    pub async fn find_all_since(
        self: &Self,
        active: bool,
        since: i64,
    ) -> Result<Vec<job::Job>, anyhow::Error> {
        if let Some(database) = self.get_db() {
            let collection = database.collection::<job::Job>("jobs");
            let jobs_cursor = collection
                .find(doc! {"active": active, "updated_at": {"$gt": since}}, None)
                .await?;
            return Ok(jobs_cursor.try_collect().await?);
        } else {
            return Err(anyhow::anyhow!("Could not connect to the database"));
        }
    }

    pub async fn disable_if_exist(self: &Self, name: &str) -> Result<(), Box<dyn Error>> {
        if let Some(database) = self.get_db() {
            if let Some(_job) = self.find_job(name, true).await {
                let collection: Collection<Document> = database.collection("jobs");
                collection
                    .update_one(
                        doc! {"name": name, "active": true},
                        doc! {"active": false, "updated_at": Utc::now().timestamp()},
                        None,
                    )
                    .await?;
                return Ok(());
            }
        } else {
            return Err("Could not get the database object".into());
        }
        Ok(())
    }

    pub async fn insert_if_not_exist(self: &Self, job: &job::Job) -> Result<(), Box<dyn Error>> {
        if let Some(_job) = self.find_job(&job.name, true).await {
            Err("Job already in database".into())
        } else {
            self.insert(job).await
        }
    }

    async fn insert(self: &Self, job: &job::Job) -> Result<(), Box<dyn Error>> {
        if let Some(database) = self.get_db() {
            let collection = database.collection("jobs");
            collection
                    .insert_one(
                        doc! { "name": &job.name, "script_type": &job.script, "script": &job.script, "time": &job.time, "timeout": &job.timeout,
                        "updated_at": &job.updated_at},
                        None,
                    )
                    .await?;

            return Ok(());
        }
        Err("Unknown error while saving the Job".into())
    }
}

pub async fn get_db(config: &Config) -> Result<DB, Box<dyn std::error::Error>> {
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
