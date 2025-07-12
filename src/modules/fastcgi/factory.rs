//! FastCGI Service Factory

use std::{path::PathBuf, rc::Rc};

use actix_service::ServiceFactory;
use actix_web::{
    Error,
    dev::{AppService, HttpServiceFactory, ResourceDef, ServiceRequest, ServiceResponse},
    guard::Guard,
};
use futures_core::future::LocalBoxFuture;

use super::pool::Pool;
use super::service::{Addr, FastCGIInner, FastCGIService};
use crate::modules::{guard::Location, utils::impl_http_service};

#[derive(Clone)]
pub struct FastCGI {
    mount_path: String,
    guards: Vec<Rc<dyn Guard>>,
    locations: Vec<Rc<dyn Location>>,
    root: PathBuf,
    pool: Rc<Pool>,
    path_param: Option<regex::Regex>,
    server_address: Addr,
}

impl FastCGI {
    pub fn new(mount_path: &str, root: PathBuf, server_address: Addr, pool: Pool) -> Self {
        Self {
            mount_path: mount_path.to_owned(),
            guards: Vec::new(),
            locations: Vec::new(),
            root,
            pool: Rc::new(pool),
            path_param: None,
            server_address,
        }
    }
    pub fn add_guard<G: Guard + 'static>(&mut self, guards: G) {
        self.guards.push(Rc::new(guards));
    }
    pub fn add_location<L: Location + 'static>(&mut self, location: L) {
        self.locations.push(Rc::new(location));
    }
    pub fn path_param(mut self, path_param: Option<regex::Regex>) -> Self {
        self.path_param = path_param.or(self.path_param);
        self
    }
}

impl_http_service!(FastCGI);

impl ServiceFactory<ServiceRequest> for FastCGI {
    type Response = ServiceResponse;
    type Error = Error;
    type Config = ();
    type Service = FastCGIService;
    type InitError = ();
    type Future = LocalBoxFuture<'static, Result<Self::Service, Self::InitError>>;

    fn new_service(&self, _: ()) -> Self::Future {
        let inner = FastCGIInner {
            guards: self.guards.clone(),
            locations: self.locations.clone(),
            root: self.root.clone(),
            pool: self.pool.clone(),
            path_param: self.path_param.clone(),
            server_address: self.server_address.clone(),
        };
        Box::pin(async move { Ok(FastCGIService(Rc::new(inner))) })
    }
}
