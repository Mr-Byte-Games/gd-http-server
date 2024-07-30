use http::{HeaderMap, Method, Uri};
use tokio::sync::oneshot;

pub struct ServerRequest {
    pub headers: HeaderMap,
    pub method: Method,
    pub uri: Uri,
    pub body: Vec<u8>,
}

pub struct ServerResponse {
    pub headers: HeaderMap,
    pub status_code: u16,
    pub body: Vec<u8>,
}

impl ServerResponse {
    pub fn not_found() -> Self {
        Self {
            headers: Default::default(),
            status_code: 404,
            body: "Not Found".as_bytes().to_vec(),
        }
    }
}

pub struct RequestResponse(pub ServerRequest, pub oneshot::Sender<ServerResponse>);
