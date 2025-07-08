//!

use std::{
    cell::{RefCell, RefMut},
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use actix_web::{
    dev::Payload,
    error::PayloadError,
    web::{Bytes, BytesMut},
};
use futures_core::{Stream, stream::LocalBoxStream};

pub(crate) struct PayloadRef {
    payload: Rc<RefCell<PayloadBuffer>>,
}

impl PayloadRef {
    pub(crate) fn new(payload: PayloadBuffer) -> PayloadRef {
        PayloadRef {
            payload: Rc::new(RefCell::new(payload)),
        }
    }

    #[inline]
    pub(crate) fn get_mut(&self) -> RefMut<'_, PayloadBuffer> {
        self.payload.borrow_mut()
    }

    #[inline]
    pub(crate) fn into_stream(&self) -> LocalBoxStream<'static, Result<Bytes, PayloadError>> {
        Box::pin(self.clone())
    }

    pub(crate) fn into_payload(&self) -> Payload {
        Payload::Stream {
            payload: self.into_stream(),
        }
    }
}

impl Clone for PayloadRef {
    fn clone(&self) -> PayloadRef {
        PayloadRef {
            payload: Rc::clone(&self.payload),
        }
    }
}

/// Payload buffer.
pub struct PayloadBuffer {
    pub(crate) stream: LocalBoxStream<'static, Result<Bytes, PayloadError>>,
    pub(crate) buf: BytesMut,
    /// EOF flag. If true, no more payload reads will be attempted.
    pub(crate) eof: bool,
    pub(crate) overflow: bool,
    // TODO: add controls similar to nginx
    // client_body_buffer_size & client_max_body_size
    pub(crate) cursor: usize,
    pub(crate) body_buffer_size: usize,
    pub(crate) max_body_size: usize,
}

impl PayloadBuffer {
    /// Constructs new payload buffer.
    pub(crate) fn new<S>(stream: S, buffer_size: usize) -> Self
    where
        S: Stream<Item = Result<Bytes, PayloadError>> + 'static,
    {
        PayloadBuffer {
            stream: Box::pin(stream),
            buf: BytesMut::with_capacity(1_024), // pre-allocate 1KiB
            eof: false,
            overflow: false,
            cursor: 0,
            body_buffer_size: buffer_size,
            max_body_size: buffer_size,
        }
    }

    #[inline]
    pub(crate) fn reset_stream(&mut self) {
        self.cursor = 0;
    }

    pub(crate) fn read_buffered(&mut self) -> Option<Bytes> {
        if self.cursor < self.buf.len() {
            let data = self
                .buf
                .clone()
                .split_to(self.buf.len() - self.cursor)
                .freeze();
            self.cursor += data.len();
            return Some(data);
        }
        None
    }
}

impl Stream for PayloadRef {
    type Item = Result<Bytes, PayloadError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // read from memory on reuse of buffer
        let mut this = self.get_mut().payload.borrow_mut();
        if let Some(data) = this.read_buffered() {
            return Poll::Ready(Some(Ok(data)));
        }
        // check for eof before re-reading again
        if this.eof {
            return Poll::Ready(None);
        }
        // check for overflow error before re-reading again
        if this.overflow {
            return Poll::Ready(Some(Err(PayloadError::Overflow)));
        }
        // read from active stream
        match Pin::new(&mut this.stream).poll_next(cx) {
            Poll::Ready(Some(Ok(data))) => {
                // check for overflow before appending slice
                if this.cursor + data.len() > this.body_buffer_size {
                    this.overflow = true;
                    return Poll::Ready(Some(Err(PayloadError::Overflow)));
                }
                // extend internal buffer and update cursor location
                this.buf.extend_from_slice(&data);
                this.cursor += data.len();
                Poll::Ready(Some(Ok(data)))
            }
            Poll::Ready(None) => {
                this.eof = true;
                Poll::Ready(None)
            }
            status => status,
        }
    }
}
