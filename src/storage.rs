use crate::config::Minio;
use chrono;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use std::fs;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

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

    pub async fn put(self: Self, file: &str, object_name: &str) -> Result<String, Error> {
        let bucket = match self.bucket(){
            Ok(b) => b,
            Err(e) => return Err(e)
        };

        let code = match bucket.get_object(object_name).await  {
            Ok((data, code)) => code,
            Err(e) => return Err(Error{message: e.to_string()}),
        };

        let name = match code {
            200 => format!("{}_{}", chrono::offset::Utc::now().timestamp(), object_name).into(),
            404 => object_name.into(),
            _ => return Err(Error{message: "Could not verify if file exists already".into()}),
        };

        let content = fs::read_to_string(file).unwrap(); //TODO DO NOT UNWRAP!!!!
        let result = bucket.put_object(&name, content.as_bytes()).await;

        let code= match result {
            Ok((_, code)) => code,
            Err(e) => return Err(Error{message: e.to_string()})
        };

        match code {
            200 => Ok(name),
            _ => Err(Error{message: format!(
                "Error while uploading file, http code = {}",
                code
            )}),
        }
    }

    fn bucket(self: Self) -> Result<Bucket, Error> {
        match Bucket::new_with_path_style(
            &self.storage.bucket,
            self.storage.region,
            self.storage.credentials,
        ) {
            Ok(b) => Ok(b),
            Err(e) => Err(Error{message: e.to_string()})
        }
    }

    pub async fn get(self: Self, object_name: &str) -> Result<Option<String>, Error> {
        // todo check code
        let object = match self.bucket() {
            Ok(bucket) => bucket.get_object(object_name).await,
            Err(e) => return Err(e),
        };

        let (data, code) = match object {
            Ok((data, code)) => (data, code),
            Err(e) => return Err(Error{message: e.to_string()}),
        };

        match std::str::from_utf8(&data) {
            Ok(v) => Ok(Some(v.into())),
            Err(e) => Err(Error{message: e.to_string()}),
        }
    }
}
