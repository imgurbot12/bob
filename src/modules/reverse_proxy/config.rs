//! Configuration Components for ReverseProxy

use std::str::FromStr;

use serde::{Deserialize, de::Error};

use crate::config::de_fromstr;

#[derive(Clone, Debug)]
pub struct Uri(pub(crate) awc::http::Uri);

impl FromStr for Uri {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            awc::http::Uri::from_str(s).map_err(|e| e.to_string())?,
        ))
    }
}

de_fromstr!(Uri);
