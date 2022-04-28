use serde::Deserialize;
use std::fs;
use std::fmt;
#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub database: Option<Database>,
    pub minio: Option<Minio>,
}

#[derive(Deserialize, Clone)]
pub struct Database {
    pub username: String,
    pub password: String,
    pub cluster_url: String,
}

#[derive(Deserialize, Clone)]
pub struct Minio {
    pub username: String,
    pub password: String,
    pub host: String,
}

impl fmt::Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Database")
         .field("username", &self.username)
         .field("password", &format!("*****"))
         .field("cluster_url", &self.cluster_url)
         .finish()
    }
}

impl fmt::Debug for Minio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Minio")
         .field("username", &self.username)
         .field("password", &format!("*****"))
         .field("host", &self.host)
         .finish()
    }
}

impl Config {
    pub fn from(file: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let config: String = fs::read_to_string(file).unwrap();
        Ok(toml::from_str(&config).unwrap())
    }
}
