//! Request Path Translation and Security Stolen from Actix-Files

use std::{
    path::{Component, Path, PathBuf},
    str::FromStr,
};

use actix_utils::future::{Ready, ready};
use actix_web::{FromRequest, HttpRequest, dev::Payload};

use super::error::UriSegmentError;

#[derive(Debug, PartialEq, Eq)]
pub struct PathBufWrap(PathBuf);

impl FromStr for PathBufWrap {
    type Err = UriSegmentError;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        Self::parse_path(path, false)
    }
}

impl PathBufWrap {
    /// Parse a path, giving the choice of allowing hidden files to be considered valid segments.
    ///
    /// Path traversal is guarded by this method.
    pub fn parse_path(path: &str, hidden_files: bool) -> Result<Self, UriSegmentError> {
        let mut buf = PathBuf::new();

        // equivalent to `path.split('/').count()`
        let mut segment_count = path.matches('/').count() + 1;

        // we can decode the whole path here (instead of per-segment decoding)
        // because we will reject `%2F` in paths using `segment_count`.
        let path = percent_encoding::percent_decode_str(path)
            .decode_utf8()
            .map_err(|_| UriSegmentError::NotValidUtf8)?;

        // disallow decoding `%2F` into `/`
        if segment_count != path.matches('/').count() + 1 {
            return Err(UriSegmentError::BadChar('/'));
        }

        for segment in path.split('/') {
            if segment == ".." {
                segment_count -= 1;
                buf.pop();
            } else if !hidden_files && segment.starts_with('.') {
                return Err(UriSegmentError::BadStart('.'));
            } else if segment.starts_with('*') {
                return Err(UriSegmentError::BadStart('*'));
            } else if segment.ends_with(':') {
                return Err(UriSegmentError::BadEnd(':'));
            } else if segment.ends_with('>') {
                return Err(UriSegmentError::BadEnd('>'));
            } else if segment.ends_with('<') {
                return Err(UriSegmentError::BadEnd('<'));
            } else if segment.is_empty() {
                segment_count -= 1;
                continue;
            } else if cfg!(windows) && segment.contains('\\') {
                return Err(UriSegmentError::BadChar('\\'));
            } else if cfg!(windows) && segment.contains(':') {
                return Err(UriSegmentError::BadChar(':'));
            } else {
                buf.push(segment)
            }
        }

        // make sure we agree with stdlib parser
        for (i, component) in buf.components().enumerate() {
            assert!(
                matches!(component, Component::Normal(_)),
                "component `{:?}` is not normal",
                component
            );
            assert!(i < segment_count);
        }

        Ok(PathBufWrap(buf))
    }
}

impl AsRef<Path> for PathBufWrap {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl FromRequest for PathBufWrap {
    type Error = UriSegmentError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(req.match_info().unprocessed().parse())
    }
}
