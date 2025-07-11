//! Fixed Static Bytes Payload

use std::task::Poll;

use actix_web::{dev::Payload, error::PayloadError, web::Bytes};
use futures_core::{Stream, stream::LocalBoxStream};

pub(crate) struct BytesPayload(Bytes);

impl BytesPayload {
    #[inline]
    pub(crate) fn new(b: Bytes) -> Self {
        Self(b)
    }

    #[inline]
    pub(crate) fn into_stream(self) -> LocalBoxStream<'static, Result<Bytes, PayloadError>> {
        Box::pin(self)
    }
    #[inline]
    pub(crate) fn into_payload(self) -> Payload {
        Payload::Stream {
            payload: self.into_stream(),
        }
    }
}

impl Stream for BytesPayload {
    type Item = Result<Bytes, PayloadError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.0.is_empty() {
            true => Poll::Ready(None),
            false => Poll::Ready(Some(Ok(self.0.slice(0..)))),
        }
    }
}
