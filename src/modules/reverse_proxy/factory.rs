//!

use std::rc::Rc;

use actix_service::ServiceFactory;
use actix_web::{
    Error,
    dev::{AppService, HttpServiceFactory, ResourceDef, ServiceRequest, ServiceResponse},
    guard::Guard,
};
use awc::{Client, http::Uri};
use futures_core::future::LocalBoxFuture;

use super::service::{ProxyService, ProxyServiceInner};
use crate::modules::{guard::Location, utils::impl_http_service};

#[derive(Clone)]
pub struct ReverseProxy {
    mount_path: String,
    guards: Vec<Rc<dyn Guard>>,
    locations: Vec<Rc<dyn Location>>,
    client: Rc<Client>,
    resolve: Uri,
}

impl ReverseProxy {
    pub fn new(mount_path: &str, client: Client, resolve: Uri) -> Self {
        Self {
            mount_path: mount_path.to_owned(),
            guards: Vec::new(),
            locations: Vec::new(),
            client: Rc::new(client),
            resolve,
        }
    }
    pub fn add_guard<G: Guard + 'static>(&mut self, guards: G) {
        self.guards.push(Rc::new(guards));
    }
    pub fn add_location<L: Location + 'static>(&mut self, locations: L) {
        self.locations.push(Rc::new(locations));
    }
}

impl_http_service!(ReverseProxy);

impl ServiceFactory<ServiceRequest> for ReverseProxy {
    type Response = ServiceResponse;
    type Error = Error;
    type Config = ();
    type Service = ProxyService;
    type InitError = ();
    type Future = LocalBoxFuture<'static, Result<Self::Service, Self::InitError>>;

    fn new_service(&self, _: ()) -> Self::Future {
        let inner = ProxyServiceInner {
            guards: self.guards.clone(),
            locations: self.locations.clone(),
            client: Rc::clone(&self.client),
            resolve: self.resolve.clone(),
        };
        Box::pin(async move { Ok(ProxyService(Rc::new(inner))) })
    }
}
