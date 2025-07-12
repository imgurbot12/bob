//! Actix Service Implementation for File Server

use std::{ops::Deref, path::PathBuf, rc::Rc};

use actix_files::NamedFile;
use actix_web::{
    body::BoxBody,
    dev::{self, Service, ServiceRequest, ServiceResponse},
    error::Error,
    guard::Guard,
    http::Method,
};
use futures_core::future::LocalBoxFuture;

use crate::modules::guard::Location;
use crate::modules::utils::PathBufWrap;
use crate::modules::utils::{check_guards, check_locations, default_response};

#[derive(Clone)]
pub struct FileService(pub(crate) Rc<FileServiceInner>);

impl Deref for FileService {
    type Target = FileServiceInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FileService {
    fn serve_named_file(&self, req: ServiceRequest, named_file: NamedFile) -> ServiceResponse {
        let (req, _) = req.into_parts();
        let res = named_file.into_response(&req);
        ServiceResponse::new(req, res)
    }
}

pub struct FileServiceInner {
    pub(crate) guards: Vec<Rc<dyn Guard>>,
    pub(crate) locations: Vec<Rc<dyn Location>>,
    pub(crate) root: PathBuf,
    pub(crate) dir_index: Option<Vec<PathBuf>>,
    pub(crate) hidden_files: bool,
}

impl Service<ServiceRequest> for FileService {
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
            let path_on_disk = match PathBufWrap::parse_path(&url_path, this.hidden_files) {
                Ok(item) => item,
                Err(err) => return Ok(req.error_response(err)),
            };

            let path = this.root.join(&path_on_disk);
            if let Err(err) = path.canonicalize() {
                return Ok(req.error_response(err));
            }

            if !path.exists() {
                return Ok(default_response(req));
            }

            if path.is_dir() {
                // check if any of the index-paths exist and serve-file
                if let Some(ref indexes) = this.dir_index {
                    for index in indexes.iter() {
                        let index_path = path.join(index);
                        if index_path.exists() {
                            return Ok(match NamedFile::open_async(index_path).await {
                                Ok(named_file) => this.serve_named_file(req, named_file),
                                Err(err) => req.error_response(err),
                            });
                        }
                    }
                }
            }

            Ok(match NamedFile::open_async(&path).await {
                Ok(named_file) => this.serve_named_file(req, named_file),
                Err(err) => req.error_response(err),
            })
        })
    }
}
