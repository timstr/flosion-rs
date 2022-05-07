use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use parking_lot::Mutex;

pub struct ResultFuture<T, E> {
    shared_state: Arc<Mutex<SharedState<T, E>>>,
}

impl<T, E> ResultFuture<T, E> {
    pub fn new() -> (ResultFuture<T, E>, OutboundResult<T, E>) {
        let shared_state: SharedState<T, E> = SharedState {
            result: None,
            waker: None,
        };
        let shared_state = Arc::new(Mutex::new(shared_state));
        (
            ResultFuture {
                shared_state: Arc::clone(&shared_state),
            },
            OutboundResult { shared_state },
        )
    }
}

struct SharedState<T, E> {
    result: Option<Result<T, E>>,
    waker: Option<Waker>,
}

impl<T, E> Future for ResultFuture<T, E> {
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock();
        if let Some(res) = shared_state.result.take() {
            Poll::Ready(res)
        } else {
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct OutboundResult<T, E> {
    shared_state: Arc<Mutex<SharedState<T, E>>>,
}

impl<T, E> OutboundResult<T, E> {
    pub fn fulfill(self, result: Result<T, E>) {
        let mut shared_state = self.shared_state.lock();
        assert!(
            shared_state.result.is_none(),
            "Attempted to fulfill an OutboundResult which has already been fulfilled"
        );
        shared_state.result = Some(result);
        if let Some(waker) = shared_state.waker.take() {
            waker.wake()
        }
    }
}
