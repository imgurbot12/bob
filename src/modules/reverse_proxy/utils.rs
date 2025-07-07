//! HTTP Proxy Utilities

use std::{collections::HashMap, path::PathBuf};

use awc::http::Uri;

type Query = HashMap<String, String>;

pub(crate) fn resolve_uri(resolve: &Uri, path: &str, request: &Uri) -> Uri {
    let path = PathBuf::from(resolve.path().to_string())
        .join(path)
        .to_str()
        .map(|s| s.to_owned())
        .expect("invalid request path");
    let query = match resolve.query() {
        None => request.query().map(|s| s.to_string()).unwrap_or_default(),
        Some(base) => {
            let mut query: Query = serde_urlencoded::from_str(base).unwrap();
            if let Some(more) = request.query() {
                let more: Query = serde_urlencoded::from_str(more).unwrap();
                query.extend(more.into_iter());
            }
            serde_urlencoded::to_string(query).unwrap_or_default()
        }
    };
    let path_and_query = match query.is_empty() {
        true => path,
        false => format!("{path}?{query}"),
    };
    Uri::builder()
        .scheme(resolve.scheme_str().unwrap_or("http"))
        .authority(
            resolve
                .authority()
                .map(|s| s.as_str())
                .expect("missing url authority"),
        )
        .path_and_query(path_and_query)
        .build()
        .expect("invalid request uri")
}
