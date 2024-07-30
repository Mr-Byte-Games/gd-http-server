use crate::internal::server::{ServerRequest, ServerResponse};
use godot::prelude::*;
use http::Method;
use querystring::querify;

#[derive(GodotConvert, Var, Export, Default, PartialEq, Eq, Hash, Debug)]
#[godot(via = GString)]
pub enum HttpMethod {
    #[default]
    UNKNOWN,
    DELETE,
    GET,
    PATCH,
    POST,
    PUT,
}

impl From<Method> for HttpMethod {
    fn from(value: Method) -> Self {
        match value {
            Method::DELETE => HttpMethod::DELETE,
            Method::GET => HttpMethod::GET,
            Method::PATCH => HttpMethod::PATCH,
            Method::POST => HttpMethod::POST,
            Method::PUT => HttpMethod::PUT,
            _ => HttpMethod::UNKNOWN,
        }
    }
}

#[derive(GodotClass)]
#[class(base=Object,init)]
pub struct HttpRequest {
    #[var]
    pub headers: Dictionary,
    #[var]
    pub method: HttpMethod,
    #[var]
    pub path: GString,
    #[var]
    pub body: PackedByteArray,
    #[var]
    pub params: Dictionary,
    #[var]
    pub query_params: Dictionary,
}

impl From<ServerRequest> for HttpRequest {
    fn from(value: ServerRequest) -> Self {
        let path = value.uri.path().into_godot();

        let parsed_query_params = querify(value.uri.query().unwrap_or_default());
        let mut query_params = Dictionary::new();
        query_params.extend(parsed_query_params.into_iter());

        let headers_iter = value.headers.iter().map(|(key, value)| {
            (
                key.as_str(),
                value.to_str().expect("Header should be a string."),
            )
        });
        let mut headers = Dictionary::new();

        headers.extend(headers_iter);

        Self {
            headers,
            path,
            method: value.method.into(),
            body: value.body.into(),
            query_params,
            params: Default::default(),
        }
    }
}

#[derive(GodotClass)]
#[class(base=Object,init)]
pub struct HttpResponse {
    #[var]
    pub headers: Dictionary,
    #[var]
    pub status_code: u16,
    #[var]
    pub body: PackedByteArray,
}

impl Default for HttpResponse {
    fn default() -> Self {
        Self {
            headers: Dictionary::default(),
            status_code: 200,
            body: PackedByteArray::default(),
        }
    }
}

impl From<ServerResponse> for HttpResponse {
    fn from(value: ServerResponse) -> Self {
        let headers_iter = value
            .headers
            .iter()
            .map(|(key, value)| (key.as_str(), PackedByteArray::from(value.as_bytes())));
        let mut headers = Dictionary::new();

        headers.extend(headers_iter);

        Self {
            headers,
            status_code: value.status_code,
            body: value.body.into(),
        }
    }
}
