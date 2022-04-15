use crate::{config::Config, heartbeat, job};
use async_trait::async_trait;
use chrono::Utc;
use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client, Collection, Database,
};
use std::error::Error;

#[derive(Debug)]
pub struct DBError {
    pub message: String,
}
pub enum DBClient {
    MongoDB(mongodb::Client),
}

pub struct MongoDBClient {
    client: Option<DBClient>,
}

#[async_trait]
pub trait DB {
    async fn send_heartbeat(self: &Self, server_name: &str) -> Result<(), DBError>;

    async fn most_recent_heartbeat(
        self: &Self,
    ) -> Result<Option<heartbeat::Heartbeat>, DBError>;

    async fn find_job(self: &Self, name: &str, active: bool) -> Option<job::Job>;

    async fn find_all_active(self: &Self) -> Result<Vec<job::Job>, DBError>;

    async fn find_all_since(
        self: &Self,
        active: bool,
        since: i64,
    ) -> Result<Vec<job::Job>, anyhow::Error>;

    async fn disable_if_exist(self: &Self, name: &str) -> Result<(), DBError>;

    async fn insert_if_not_exist(self: &Self, job: &job::Job) -> Result<(), DBError>;
}

impl MongoDBClient {
    pub fn connection_url(username: &str, password: &str, cluster_url: &str) -> String {
        format!(
            "mongodb+srv://{}:{}@{}/dcron?w=majority",
            username, password, cluster_url
        )
    }
    pub async fn local_connection() -> Result<Self, Box<dyn std::error::Error>> {
        Self::connect("mongodb://localhost:27017/dcron".into()).await
    }

    async fn connect(url: String) -> Result<Self, Box<dyn std::error::Error>> {
        let client_options = ClientOptions::parse(url).await?;

        let client = Client::with_options(client_options)?;
        client
            .database("admin")
            .run_command(doc! {"ping": 1}, None)
            .await?;

        Ok(Self {
            client: Some(DBClient::MongoDB(client)),
        })
    }
    fn get_db(self: &Self) -> Option<Database> {
        if let Some(DBClient::MongoDB(client)) = &self.client {
            return Some(client.database("dcron"));
        }
        None
    }
    async fn insert(self: &Self, job: &job::Job) -> Result<(), DBError> {
        if let Some(database) = self.get_db() {
            let collection = database.collection("jobs");
            return match collection
                    .insert_one(
                        doc! { "name": &job.name, "script_type": &job.script, "script": &job.script, "time": &job.time, "timeout": &job.timeout,
                        "updated_at": &job.updated_at},
                        None,
                    )
                    .await {
                Ok(_) => Ok(()),
                Err(e) => Err(DBError{message: e.to_string()})
            };
        }
        Err(DBError{message: "Unknown error while saving the Job".into()})
    }
}

#[async_trait]
impl DB for MongoDBClient {
    async fn send_heartbeat(self: &Self, server_name: &str) -> Result<(), DBError> {
        if let Some(database) = self.get_db() {
            let collection = database.collection("heartbeats");
            return match collection
                .insert_one(
                    doc! { "server": server_name, "timestamp": Utc::now().timestamp()},
                    None,
                )
                .await {
                Ok(_) => Ok(()),
                Err(e) => Err(DBError{message: e.to_string()})
            };
        }
        Err(DBError{message: "Unknown error while saving heartbeats".into()})
    }

    async fn most_recent_heartbeat(
        self: &Self,
    ) -> Result<Option<heartbeat::Heartbeat>, DBError> {
        if let Some(database) = self.get_db() {
            let collection = database.collection::<heartbeat::Heartbeat>("heartbeats");
            let since = Utc::now().timestamp() - 30000;
            let hb = collection
                .find_one(
                    doc! { "timestamp": {"$gt": since },"$sort": {
                       "server": 1,
                    } },
                    None,
                )
                .await;

            return match hb {
                Ok(Some(hb)) => Ok(Some(hb)),
                _ => Ok(None),
            }
        }
        Err(DBError{message: "Could not get a heartbeat".to_string()})
    }

    async fn find_job(self: &Self, name: &str, active: bool) -> Option<job::Job> {
        if let Some(database) = self.get_db() {
            let collection = database.collection::<job::Job>("jobs");

            let job = collection
                .find_one(doc! { "name": name, "active": active }, None)
                .await;

            return match job {
                Ok(Some(job)) => Some(job),
                _ => None,
            };
        }
        None
    }
    async fn find_all_active(self: &Self) -> Result<Vec<job::Job>, DBError> {
        return match self.get_db() {
            Some(database) => {
                let collection = database.collection::<job::Job>("jobs");
                let jobs_cursor = collection.find(doc! {"active": true}, None).await;

                let result = match jobs_cursor {
                    Ok(jobs_cursor) => jobs_cursor.try_collect().await,
                    Err(e) => return Err(DBError{message: e.to_string()}),
                };

                match result {
                    Ok(result) => Ok(result),
                    Err(e) => Err(DBError{message: e.to_string()})
                }

            }
            None => {
                Err(DBError{message: "Could not connect to the database".to_string()})
            }
        };
    }

    async fn find_all_since(
        self: &Self,
        active: bool,
        since: i64,
    ) -> Result<Vec<job::Job>, anyhow::Error> {
        match self.get_db() {
            Some(database) => {
                let collection = database.collection::<job::Job>("jobs");
                let jobs_cursor = collection
                    .find(doc! {"active": active, "updated_at": {"$gt": since}}, None)
                    .await?;
                Ok(jobs_cursor.try_collect().await?)
            }
            None => {
                Err(anyhow::anyhow!("Could not connect to the database"))
            }
        }
    }

    async fn disable_if_exist(self: &Self, name: &str) -> Result<(), DBError> {
        if let Some(database) = self.get_db() {
            if let Some(_job) = self.find_job(name, true).await {
                let collection: Collection<Document> = database.collection("jobs");
                return match collection
                    .update_one(
                        doc! {"name": name, "active": true},
                        doc! {"active": false, "updated_at": Utc::now().timestamp()},
                        None,
                    )
                    .await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(DBError{message: e.to_string()})
                };
            }
        } else {
            return Err(DBError{message: "Could not get the database object".to_string()});
        }
        Ok(())
    }

    async fn insert_if_not_exist(self: &Self, job: &job::Job) -> Result<(), DBError> {
        if let Some(_job) = self.find_job(&job.name, true).await {
            Err(DBError{message: "Job already in database".into()})
        } else {
            self.insert(job).await
        }
    }
}

//TODO: Based on the config pick other clients
pub async fn get_db(
    config: &Config,
) -> Result<Box<dyn DB + std::marker::Send + Sync>, DBError> {
    //TODO for now only returns mongo
    if let Some(db_config) = &config.database {
        let url = MongoDBClient::connection_url(
            &db_config.username,
            &db_config.password,
            &db_config.cluster_url,
        );

        match MongoDBClient::connect(url).await {
            Ok(c) => Ok(Box::new(c)),
            Err(e) => Err(DBError{message: e.to_string()})
        }


    } else {
        match MongoDBClient::local_connection().await {
            Ok(c) => Ok(Box::new(c)),
            Err(e) => Err(DBError{message: e.to_string()})
        }
    }
}
