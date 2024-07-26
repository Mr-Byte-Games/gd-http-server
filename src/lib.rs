mod server;
mod tokio_io;

use std::collections::HashMap;

use godot::prelude::*;
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use matchit::Router;
use querystring::querify;
use server::{RequestResponse, Server, ServerRequest, ServerResponse};

struct HttpServerExtension;

#[gdextension]
unsafe impl ExtensionLibrary for HttpServerExtension {}

#[derive(GodotClass)]
#[class(base=Node,init)]
pub struct HttpServer {
    #[export]
    pub port: GString,
    server: Option<Server>,
    routes: HashMap<HttpMethod, Router<Callable>>,
    base: Base<Node>,
}

#[godot_api]
impl HttpServer {
    #[func]
    pub fn route(&mut self, method: HttpMethod, path: GString, handler: Callable) {
        let router = self.routes.entry(method).or_default();
        if let Err(err) = router.insert(path, handler) {
            godot_error!("{err}");
        }
    }

    fn process_request(
        routes: &HashMap<HttpMethod, Router<Callable>>,
        request: ServerRequest,
    ) -> ServerResponse {
        let method = request.method.clone().into();
        let Some(router) = routes.get(&method) else {
            return ServerResponse::not_found();
        };

        let path = request.uri.path().to_string();
        let Ok(route) = router.at(&path) else {
            return ServerResponse::not_found();
        };

        let mut request: HttpRequest = request.into();
        request.params.extend(route.params.iter());

        let request: Gd<HttpRequest> = Gd::from_object(request);
        let response: Gd<HttpResponse> = Gd::from_object(Default::default());
        let args = array![request.to_variant(), response.to_variant()];

        route.value.callv(args);

        let response = response.bind();
        let headers = response
            .headers
            .iter_shared()
            .map(|(key, value)| {
                (
                    HeaderName::from_bytes(key.to_godot().stringify().to_string().as_bytes())
                        .expect("Unable to convert header name."),
                    HeaderValue::from_bytes(value.to_godot().stringify().to_string().as_bytes())
                        .expect("Unable to convert header value."),
                )
            })
            .collect::<HeaderMap>();

        ServerResponse {
            headers,
            status_code: response.status_code,
            body: response.body.to_vec(),
        }
    }
}

#[godot_api]
impl INode for HttpServer {
    fn process(&mut self, _delta: f64) {
        let Some(ref mut server) = self.server else {
            return;
        };

        for RequestResponse(request, sender) in server.pending_requests() {
            let response = Self::process_request(&self.routes, request);
            let _ = sender.send(response);
        }
    }

    fn exit_tree(&mut self) {
        if let Some(server) = self.server.take() {
            server.shutdown();
        }
    }

    fn ready(&mut self) {
        self.server = Some(Server::new(self.port.to_string()));
    }
}

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
    headers: Dictionary,
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
    headers: Dictionary,
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
