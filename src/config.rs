use serde::Deserialize;
use std::fs;
#[derive(Deserialize, Clone)]
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

impl Config {
    pub fn from(file: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let config: String = fs::read_to_string(file).unwrap();
        Ok(toml::from_str(&config).unwrap())
    }
}
