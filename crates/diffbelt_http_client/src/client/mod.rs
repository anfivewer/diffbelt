use hyper::client::HttpConnector;
use hyper::{Body, Client};

pub mod methods;

pub struct DiffbeltClientNewOptions {
    pub host: String,
    pub port: u16,
}

pub struct AnotherClient {}

pub struct DiffbeltClient {
    uri_start: String,
    client: Client<HttpConnector, Body>,
}

impl DiffbeltClient {
    pub fn new(options: DiffbeltClientNewOptions) -> Self {
        let DiffbeltClientNewOptions { host, port } = options;

        Self {
            uri_start: format!("http://{}:{}", host, port),
            client: Client::builder().build_http(),
        }
    }
}
