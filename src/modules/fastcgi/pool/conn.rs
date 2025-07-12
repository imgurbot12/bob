//! Socket Connection Abstraction with Support for Unix/TCP

use pin_project::pin_project;

use std::{
    io::Error,
    pin::Pin,
    task::{Context, Poll},
};

use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpStream, UnixStream},
};

#[pin_project(project = AbsStreamProj)]
pub enum AbsStream {
    Unix(#[pin] UnixStream),
    TCP(#[pin] TcpStream),
}

impl AbsStream {
    pub async fn connect(addr: &str) -> std::io::Result<Self> {
        let (scheme, addr) = addr.split_once("://").unwrap_or_else(|| ("tcp", addr));
        match &scheme.to_lowercase() == "unix" {
            true => Ok(Self::Unix(UnixStream::connect(addr).await?)),
            false => Ok(Self::TCP(TcpStream::connect(addr).await?)),
        }
    }
}

impl AsyncRead for AbsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.project() {
            AbsStreamProj::Unix(u) => u.poll_read(cx, buf),
            AbsStreamProj::TCP(t) => t.poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for AbsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, Error>> {
        match self.project() {
            AbsStreamProj::Unix(u) => u.poll_write(cx, buf),
            AbsStreamProj::TCP(t) => t.poll_write(cx, buf),
        }
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Result<(), Error>> {
        match self.project() {
            AbsStreamProj::Unix(u) => u.poll_flush(cx),
            AbsStreamProj::TCP(t) => t.poll_flush(cx),
        }
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.project() {
            AbsStreamProj::Unix(u) => u.poll_shutdown(cx),
            AbsStreamProj::TCP(t) => t.poll_shutdown(cx),
        }
    }
}
