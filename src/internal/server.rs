use http::{Request, Response};
use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Bytes};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use std::convert::Infallible;
use std::fmt::Debug;
use std::sync::mpsc;
use tokio::{self, net::TcpListener, runtime::Runtime, sync::oneshot};
mod model;

// use super::tokio_io::TokioIo;
pub(crate) use model::*;

pub struct Server {
    pending_requests: mpsc::Receiver<RequestResponse>,
    shutdown_signal: oneshot::Sender<()>,
}

impl Server {
    pub fn new(port: String) -> Self {
        let (request_sender, request_receiver) = mpsc::channel();
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();

        spawn_server(port, request_sender, shutdown_receiver);

        Server {
            pending_requests: request_receiver,
            shutdown_signal: shutdown_sender,
        }
    }

    pub fn pending_requests(&mut self) -> impl Iterator<Item = RequestResponse> + '_ {
        self.pending_requests.try_iter()
    }

    pub fn shutdown(self) {
        // NOTE: Don't care if this fails.
        let _ = self.shutdown_signal.send(());
    }
}

fn spawn_server(
    port: String,
    request_sender: mpsc::Sender<RequestResponse>,
    shutdown_receiver: oneshot::Receiver<()>,
) {
    std::thread::spawn(move || {
        let runtime = Runtime::new().expect("Failed to create tokio runtime.");

        runtime.block_on(async move {
            tokio::spawn(async move {
                let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
                    .await
                    .expect("Unable to open socket.");

                tokio::pin!(shutdown_receiver);

                loop {
                    tokio::select! {
                        _ = &mut shutdown_receiver => break,
                        Ok((stream, _)) = listener.accept() => {
                            let stream = hyper_util::rt::TokioIo::new(stream);
                            let request_sender = request_sender.clone();

                            tokio::task::spawn(async move {
                                let result = http1::Builder::new()
                                    .serve_connection(
                                        stream,
                                        service_fn( |request| handler(request_sender.clone(), request)),
                                    )
                                    .await;

                                if let Err(err) = result {
                                    eprintln!("Error serving connection: {:?}", err);
                                }
                            });
                        }
                    }
                }
            }).await.expect("server task unexpectedly terminated");
        });
    });
}

async fn handler<B>(
    sender: mpsc::Sender<RequestResponse>,
    request: Request<B>,
) -> Result<Response<Full<Bytes>>, Infallible>
where
    B: Body,
    B::Error: Debug,
{
    let (tx, rx) = oneshot::channel();

    let (parts, body) = request.into_parts();
    let body = body
        .collect()
        .await
        .expect("Unable to read body.")
        .to_bytes();

    let request = ServerRequest {
        headers: parts.headers,
        method: parts.method,
        uri: parts.uri,
        body: body.into(),
    };

    sender
        .send(RequestResponse(request, tx))
        .expect("Failed to send request.");

    let server_response = rx.await.expect("Failed to receive response.");
    let mut response = Response::builder().status(server_response.status_code);

    if let Some(headers) = response.headers_mut() {
        headers.extend(server_response.headers);
    }

    let response = response
        .body(Full::new(Bytes::from(server_response.body)))
        .expect("Unable to construct response");

    Ok(response)
}
