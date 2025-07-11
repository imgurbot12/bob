//! File Server Service Factory

use std::{path::PathBuf, rc::Rc};

use actix_service::ServiceFactory;
use actix_web::{
    Error,
    dev::{AppService, HttpServiceFactory, ResourceDef, ServiceRequest, ServiceResponse},
    guard::Guard,
};
use futures_core::future::LocalBoxFuture;

use super::service::{FileService, FileServiceInner};
use crate::modules::{guard::Location, utils::impl_http_service};

#[derive(Clone)]
pub struct FileServer {
    mount_path: String,
    guards: Vec<Rc<dyn Guard>>,
    locations: Vec<Rc<dyn Location>>,
    root: PathBuf,
    dir_index: Option<Vec<PathBuf>>,
    hidden_files: bool,
}

impl FileServer {
    pub fn new(mount_path: &str, root: PathBuf) -> Self {
        Self {
            mount_path: mount_path.to_owned(),
            guards: Vec::new(),
            locations: Vec::new(),
            root,
            dir_index: None,
            hidden_files: false,
        }
    }
    pub fn add_guard<G: Guard + 'static>(&mut self, guards: G) {
        self.guards.push(Rc::new(guards));
    }
    pub fn add_location<L: Location + 'static>(&mut self, location: L) {
        self.locations.push(Rc::new(location));
    }
    pub fn directory_index(mut self, dir_index: Vec<PathBuf>) -> Self {
        if !dir_index.is_empty() {
            self.dir_index = Some(dir_index);
        }
        self
    }
    pub fn hidden_files(mut self, hidden_files: bool) -> Self {
        self.hidden_files = hidden_files;
        self
    }
}

impl_http_service!(FileServer);

impl ServiceFactory<ServiceRequest> for FileServer {
    type Response = ServiceResponse;
    type Error = Error;
    type Config = ();
    type Service = FileService;
    type InitError = ();
    type Future = LocalBoxFuture<'static, Result<Self::Service, Self::InitError>>;

    fn new_service(&self, _: ()) -> Self::Future {
        let inner = FileServiceInner {
            guards: self.guards.clone(),
            locations: self.locations.clone(),
            root: self.root.clone(),
            dir_index: self.dir_index.clone(),
            hidden_files: self.hidden_files,
        };
        Box::pin(async move { Ok(FileService(Rc::new(inner))) })
    }
}
