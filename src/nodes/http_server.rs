mod model;

use std::collections::HashMap;

use crate::internal::server::{RequestResponse, Server, ServerRequest, ServerResponse};
use godot::prelude::*;
use http::{HeaderMap, HeaderName, HeaderValue};
use matchit::Router;

pub use model::*;

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

        let uri = request.uri.clone();
        let Ok(route) = router.at(uri.path()) else {
            return ServerResponse::not_found();
        };

        let mut request: HttpRequest = request.into();
        request.params.extend(route.params.iter());

        let request: Gd<HttpRequest> = Gd::from_object(request);
        let response: Gd<HttpResponse> = Gd::from_object(Default::default());
        let args = varray![request, response];

        route.value.callv(&args);

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
