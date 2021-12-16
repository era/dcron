use crate::config::Minio;
use anyhow;
use chrono;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use std::fs;

pub struct Storage {
    name: String,
    region: Region,
    credentials: Credentials,
    bucket: String,
    location_supported: bool,
}

pub struct Client {
    pub storage: Storage,
}
impl Client {
    pub fn connect(minio_config: &Minio) -> Client {
        let minio = Storage {
            name: "minio".into(),
            region: Region::Custom {
                region: "".into(),
                endpoint: (&minio_config.host).to_owned(),
            },
            credentials: Credentials {
                access_key: Some((&minio_config.username).to_owned()),
                secret_key: Some((&minio_config.password).to_owned()),
                security_token: None,
                session_token: None,
            },
            bucket: "rust-s3".to_string(),
            location_supported: false,
        };

        Client { storage: minio }
    }

    pub async fn put(self: Self, file: &str, object_name: &str) -> Result<String, anyhow::Error> {
        let bucket = self.bucket()?;

        let (_, code) = bucket.get_object(object_name).await?;

        let name = match code {
            200 => format!("{}_{}", chrono::offset::Utc::now().timestamp(), object_name).into(),
            404 => object_name.into(),
            _ => return Err(anyhow::anyhow!("Could not verify if file exists already")),
        };

        let content = fs::read_to_string(file).unwrap(); //TODO DO NOT UNWRAP!!!!
        let (_, code) = bucket.put_object(&name, content.as_bytes()).await?;

        match code {
            200 => Ok(name),
            _ => Err(anyhow::anyhow!(format!(
                "Error while uploading file, http code = {}",
                code
            ))),
        }
    }

    fn bucket(self: Self) -> Result<Bucket, anyhow::Error> {
        Bucket::new_with_path_style(
            &self.storage.bucket,
            self.storage.region,
            self.storage.credentials,
        )
    }

    pub async fn get(self: Self, object_name: &str) -> Result<Option<String>, anyhow::Error> {
        let (data, code) = self.bucket()?.get_object(object_name).await?;
        // todo check code
        match std::str::from_utf8(&data) {
            Ok(v) => Ok(Some(v.into())),
            Err(e) => Err(anyhow::anyhow!("Could not read file as UTF8")),
        }
    }
}
