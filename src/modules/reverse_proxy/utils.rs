//! HTTP Proxy Utilities

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use awc::http::Uri;

type Query = HashMap<String, String>;

pub(crate) fn combine_uri(resolve: &Uri, path: &str, request: &Uri) -> Result<Uri> {
    let path = PathBuf::from(resolve.path())
        .join(path)
        .to_str()
        .map(|s| s.to_owned())
        .context("invalid request path")?;
    let query = match resolve.query() {
        None => request.query().map(|s| s.to_string()).unwrap_or_default(),
        Some(base) => {
            let mut query: Query =
                serde_urlencoded::from_str(base).context("invalid resolve query-string")?;
            if let Some(more) = request.query() {
                let more: Query =
                    serde_urlencoded::from_str(more).context("invalid request query-string")?;
                query.extend(more.into_iter());
            }
            serde_urlencoded::to_string(query).context("failed to combine query strings")?
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
                .context("missing base url authority")?,
        )
        .path_and_query(path_and_query)
        .build()
        .context("failed to build request uri")
}
