//! Configuration Components for ReverseProxy

use std::str::FromStr;

use serde::{Deserialize, de::Error};

use crate::config::de_fromstr;

#[derive(Clone, Debug)]
pub struct Uri(pub(crate) awc::http::Uri);

#[derive(Clone, Debug)]
pub struct Duration(pub(crate) std::time::Duration);

impl Duration {
    #[inline]
    pub fn from_secs(secs: u64) -> std::time::Duration {
        std::time::Duration::from_secs(secs)
    }
}

impl FromStr for Uri {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            awc::http::Uri::from_str(s).map_err(|e| e.to_string())?,
        ))
    }
}

impl FromStr for Duration {
    type Err = humantime::DurationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(humantime::parse_duration(s)?))
    }
}

de_fromstr!(Uri);
de_fromstr!(Duration);
