//! "Bare URL" Configurable File Resolver Extensions to Tower ServeDir
//! initial design stolen from: https://github.com/tower-rs/tower-http/issues/383#issuecomment-2856072323

use axum::http::{Request, Response, Uri, uri::PathAndQuery};
use std::{
    convert::Infallible,
    future::Future,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;
use tower_http::services::{
    ServeDir,
    fs::{DefaultServeDirFallback, ServeFileSystemResponseBody},
};

/// Middleware to support "bare urls" (without .html extension)
#[derive(Clone, Debug)]
pub struct SmartServeDir {
    inner: ServeDir,
    indexes: Vec<PathBuf>,
    root: PathBuf,
    try_files: Vec<String>,
}

impl SmartServeDir {
    pub fn new(root: &PathBuf, try_files: &Vec<String>, indexes: Vec<PathBuf>) -> Self {
        Self {
            inner: ServeDir::new(root).append_index_html_on_directories(false),
            indexes,
            root: root.clone(),
            try_files: try_files.clone(),
        }
    }
}

impl<ReqBody> Service<Request<ReqBody>> for SmartServeDir
where
    ReqBody: Send + 'static,
{
    type Response = Response<ServeFileSystemResponseBody>;
    type Error = Infallible;
    type Future = Pin<
        Box<dyn Future<Output = Result<Response<ServeFileSystemResponseBody>, Infallible>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <ServeDir<DefaultServeDirFallback> as Service<Request<ReqBody>>>::poll_ready(
            &mut self.inner,
            cx,
        )
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let req_path = req.uri().path().trim_start_matches('/');

        let mut path = self.root.join(req_path);
        if !path.exists() {
            for pattern in self.try_files.iter() {
                let new_path_str = pattern.replace("$uri", req_path);
                let new_path = PathBuf::from(&new_path_str);
                if new_path.exists() {
                    log::trace!("resolved uri {path:?} to {new_path:?}");
                    path = new_path;
                    *req.uri_mut() = new_uri(req.uri(), &new_path_str);
                    break;
                }
            }
        }
        if path.is_dir() {
            for index in self.indexes.iter() {
                let new_path = path.join(index);
                if new_path.exists() {
                    log::trace!("resolved index {path:?} at {new_path:?}");
                    let new_path_str = new_path.to_str().expect("invalid index path");
                    *req.uri_mut() = new_uri(req.uri(), new_path_str);
                }
            }
        }

        Box::pin(self.inner.call(req))
    }
}

fn new_uri(uri: &Uri, new_path_str: &str) -> Uri {
    let mut parts = uri.clone().into_parts();
    let new_path_and_query = if let Some(query) = uri.query() {
        PathAndQuery::from_maybe_shared(format!("{new_path_str}?{query}"))
    } else {
        let path_bytes = new_path_str.to_string().as_bytes().to_owned();
        PathAndQuery::from_maybe_shared(path_bytes)
    }
    .expect("Uri to still be valid");
    parts.path_and_query = Some(new_path_and_query);
    Uri::from_parts(parts).expect("parts to be still valid")
}
