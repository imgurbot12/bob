//!

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use actix_web::{
    error::PayloadError,
    web::{Bytes, BytesMut},
};
use futures_core::{Stream, stream::LocalBoxStream};

//TODO: need to load stream into memory or new temp file stream
// that can be re-generated/copied to create multiple instances
// of ServiceRequest that can be passed to each module.
//
// OPTIMIATION: avoid redunant iteration or complex processing
// when only single module is present

/// Payload buffer.
pub struct PayloadBuffer {
    pub(crate) stream: LocalBoxStream<'static, Result<Bytes, PayloadError>>,
    pub(crate) buf: BytesMut,
    /// EOF flag. If true, no more payload reads will be attempted.
    pub(crate) eof: bool,
    // TODO: add controls similar to nginx
    // client_body_buffer_size & client_max_body_size
}

impl PayloadBuffer {
    /// Constructs new payload buffer.
    pub(crate) fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, PayloadError>> + 'static,
    {
        PayloadBuffer {
            stream: Box::pin(stream),
            buf: BytesMut::with_capacity(1_024), // pre-allocate 1KiB
            eof: false,
        }
    }

    pub(crate) fn poll_stream(&mut self, cx: &mut Context<'_>) -> Result<(), PayloadError> {
        loop {
            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(data))) => {
                    self.buf.extend_from_slice(&data);
                    // try to read more data
                    continue;
                }
                Poll::Ready(Some(Err(err))) => return Err(err),
                Poll::Ready(None) => {
                    self.eof = true;
                    return Ok(());
                }
                Poll::Pending => return Ok(()),
            }
        }
    }
}
