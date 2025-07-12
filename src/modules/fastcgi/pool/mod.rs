//! FastGCI Client Pool

use anyhow::{Context, Error, Result};
use fastcgi_client::{Client, Params, Request, conn::KeepAlive};

mod conn;
use conn::AbsStream;

pub type Pool = bb8::Pool<ConnectionManager>;

#[derive(Clone, Debug)]
pub struct ConnectionManager(String);

impl ConnectionManager {
    pub fn new(addr: &str) -> Self {
        Self(addr.to_owned())
    }
}

impl bb8::ManageConnection for ConnectionManager {
    type Connection = Client<AbsStream, KeepAlive>;
    type Error = Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let stream = AbsStream::connect(&self.0).await?;
        Ok(Client::new_keep_alive(stream))
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        let mut empty = tokio::io::empty();
        let request = Request::new(Params::default(), &mut empty);
        let _ = conn
            .execute(request)
            .await
            .context("empty request failed")?;
        Ok(())
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}
