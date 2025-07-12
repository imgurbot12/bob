//! Actix Service Implementation for FasgCGI

use std::{ops::Deref, path::PathBuf, rc::Rc};

use actix_web::{
    HttpResponse,
    body::BoxBody,
    dev::{self, Service, ServiceRequest, ServiceResponse},
    error::Error,
    guard::Guard,
    http::Method,
};
use fastcgi_client::{Params, Request};
use futures_core::future::LocalBoxFuture;

use super::pool::Pool;
use crate::modules::utils::{check_guards, check_locations, default_response};
use crate::modules::{guard::Location, utils::PathBufWrap};

pub type Addr = (String, u16);

#[derive(Clone)]
pub struct FastCGIService(pub(crate) Rc<FastCGIInner>);

impl Deref for FastCGIService {
    type Target = FastCGIInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct FastCGIInner {
    pub(crate) guards: Vec<Rc<dyn Guard>>,
    pub(crate) locations: Vec<Rc<dyn Location>>,
    pub(crate) pool: Rc<Pool>,
    pub(crate) root: PathBuf,
    pub(crate) path_param: Option<regex::Regex>,
    pub(crate) server_address: Addr,
}

impl Service<ServiceRequest> for FastCGIService {
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::always_ready!();

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // skip processing if not a GET/HEAD
        if !matches!(*req.method(), Method::HEAD | Method::GET) {
            return Box::pin(async move { Ok(default_response(req)) });
        }

        // skip processing if locations/guards do not match
        let ctx = req.guard_ctx();
        let url_path = check_locations!(req, &ctx, self.locations);
        check_guards!(req, &ctx, self.guards);

        let this = self.clone();
        Box::pin(async move {
            let path_on_disk = match PathBufWrap::parse_path(&url_path, false) {
                Ok(item) => item,
                Err(err) => return Ok(req.error_response(err)),
            };

            let path = this.root.join(path_on_disk);
            let path_str = match path.to_str() {
                Some(pstr) => pstr,
                None => {
                    return Ok(
                        req.into_response(HttpResponse::BadRequest().body("invalid request path"))
                    );
                }
            };
            let script_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            let mut params = Params::default()
                .document_uri(script_name)
                .request_method(req.method().as_str())
                .request_uri(&url_path)
                .script_name(script_name)
                .script_filename(path_str)
                .server_addr(&this.server_address.0)
                .server_port(this.server_address.1)
                .server_name(req.connection_info().host().to_owned());

            if let Some(peer) = req.peer_addr() {
                let client = peer.ip().to_string();
                params = params.remote_addr(client).remote_port(peer.port());
            }

            println!("getting client!");
            let mut client = this
                .pool
                .get()
                .await
                .expect("failed to access connection pool");

            let empty = tokio::io::empty();
            let request = Request::new(params, empty);

            println!("running request!");
            let res = client.execute(request).await.unwrap();

            println!("stdout: {:?}", res.stdout);
            println!("stderr: {:?}", res.stderr);

            Ok(default_response(req))
        })
    }
}
