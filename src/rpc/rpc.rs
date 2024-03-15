use std::net::SocketAddr;

use crate::network::Logical;
use crate::rpc::{on_account_balance, on_work_generate};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming as IncomingBody, header, StatusCode};
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

pub struct Rpc {
    logical: Logical,
}

impl Rpc {
    pub fn new(logical: Logical) -> Self {
        Self { logical }
    }
}

fn b2s<S: serde::de::DeserializeOwned>(b: &[u8]) -> Result<S> {
    Ok(serde_json::from_slice(b)?)
}

fn s2j<S: Serialize>(s: S) -> String {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    s.serialize(&mut ser).unwrap();
    String::from_utf8(buf).unwrap()
}

impl Rpc {
    async fn handle_request(req: Request<IncomingBody>) -> Result<Response<BoxBody>> {
        // Aggregate the body...
        let whole_body = req.collect().await?.to_bytes();
        // Decode as JSON...
        #[derive(Deserialize)]
        struct TopLevel {
            action: String,
        }
        let action = b2s::<TopLevel>(&whole_body)?.action;
        let json = (move || -> Result<String> {
            Ok(match action.as_str() {
                "account_balance" => s2j(on_account_balance(b2s(&whole_body)?)),
                "work_generate" => s2j(on_work_generate(b2s(&whole_body)?)),
                _ => "{\n    \"error\": \"Unknown action\"\n}".to_string(),
            })
        }())
        .unwrap_or_else(|err| {
            s2j(json!({
                "error": format!("{}", err)
            }))
        });
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(full(json))?;
        Ok(response)
    }

    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(self.logical.to_socket_addr()).await?;
        println!("Listening on http://{}", self.logical);
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);

            tokio::task::spawn(async move {
                let service = service_fn(move |req| Self::handle_request(req));

                _ = http1::Builder::new().serve_connection(io, service).await;
            });
        }
    }
}
