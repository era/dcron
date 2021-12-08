
struct Client {}


impl Client {
    pub fb connect() {
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
    }
    }

}
