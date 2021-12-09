use anyhow;
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
    pub fn connect() -> Client {
        let minio = Storage {
            name: "minio".into(),
            region: Region::Custom {
                region: "".into(),
                endpoint: "http://127.0.0.1:9001".into(),
            },
            credentials: Credentials {
                access_key: Some("ACCESS_KEY".to_owned()),
                secret_key: Some("SECRET_KEY".to_owned()),
                security_token: None,
                session_token: None,
            },
            bucket: "rust-s3".to_string(),
            location_supported: false,
        };

        Client { storage: minio }
    }

    pub async fn put(self: Self, file: &str, object_name: &str) -> Result<(), anyhow::Error> {
        let content = fs::read_to_string(file).unwrap(); //TODO DO NOT UNWRAP!!!!
        let (_, code) = self
            .bucket()?
            .put_object(object_name, content.as_bytes())
            .await?;
        println!("{:?}", code);
        //TODO if code != 200 throw an error

        Ok(())
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
