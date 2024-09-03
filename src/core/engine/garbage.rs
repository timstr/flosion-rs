use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};

/// Garbage is a trait for types consisting of things that may be
/// expensive to dispove of. The single method `toss` is intended
/// to consume unpack that object and send those which need
/// deallocation down the GarbageChute.
pub(crate) trait Garbage<'ctx> {
    /// Consume the object, and send any of its parts that need
    /// deallocating down the chute.
    fn toss(self, chute: &GarbageChute<'ctx>);
}

/// Droppable is a trait for types which may be expensive to dispose
/// of, for example due to possible locking during memory deallocation,
/// and which may be sent down a GarbageChute and dropped on a different
/// thread.
pub trait Droppable: Send {}

/// Blanket implementation for everything that is Send and Sync
impl<T: Send> Droppable for T {}

/// Wrapped item type for things that travel between threads down the
/// garbage chute.
enum WrappedDroppable<'ctx> {
    Box(Box<dyn 'ctx + Droppable>),
    Arc(Arc<dyn 'ctx + Sync + Droppable>),
}

/// GarbageChute is a system for sending resources to a different
/// thread to dispose of and drop separately, and thereby not incur
/// any possible delays due to memory allocation locking on the
/// original thread.
pub(crate) struct GarbageChute<'ctx> {
    sender: SyncSender<WrappedDroppable<'ctx>>,
    backlog: Arc<AtomicUsize>,
    capacity: usize,
}

impl<'ctx> GarbageChute<'ctx> {
    /// Send an item which lives in a Box down the chute.
    /// When the garbage is cleared, the inner item will be dropped
    /// immediately.
    pub(crate) fn send_box(&self, item: Box<dyn 'ctx + Droppable>) {
        self.sender.try_send(WrappedDroppable::Box(item)).unwrap();
        let backlog = self.backlog.fetch_add(1, Ordering::Relaxed);
        if backlog * 4 > self.capacity {
            println!(
                "Warning: garbage chute is filling up: {} undisposed items",
                backlog
            );
        }
    }

    /// Send an item which lives in an Arc down the chute
    /// When the garbage is cleared, the inner item will be dropped
    /// immediately only if the Arc holds the last strong reference.
    pub(crate) fn send_arc(&self, item: Arc<dyn 'ctx + Sync + Droppable>) {
        self.sender.try_send(WrappedDroppable::Arc(item)).unwrap();
        let backlog = self.backlog.fetch_add(1, Ordering::Relaxed);
        if backlog * 4 > self.capacity {
            println!(
                "Warning: garbage chute is filling up: {} undisposed items",
                backlog
            );
        }
    }
}

/// The receiving end of a GarbageChute. Its only responsibility is to
/// periodically be cleared, thereby disposing of and dropping everything
/// that has been sent down the chute so far.
pub(crate) struct GarbageDisposer<'ctx> {
    receiver: Receiver<WrappedDroppable<'ctx>>,
    backlog: Arc<AtomicUsize>,
}

impl<'ctx> GarbageDisposer<'ctx> {
    /// Dispose of and drop everything that has come down the chute so far.
    pub(crate) fn clear(&self) {
        let mut count: usize = 0;
        while let Ok(item) = self.receiver.try_recv() {
            std::mem::drop(item);
            count += 1;
        }
        self.backlog.fetch_sub(count, Ordering::Relaxed);
    }
}

/// Create a new GarbageChute and GarbageDisposer pair.
pub(crate) fn new_garbage_disposer<'ctx>() -> (GarbageChute<'ctx>, GarbageDisposer<'ctx>) {
    let capacity = 1024;
    let (box_sender, box_receiver) = sync_channel(capacity);
    let backlog = Arc::new(AtomicUsize::new(0));
    let chute = GarbageChute {
        sender: box_sender,
        backlog: Arc::clone(&backlog),
        capacity,
    };
    let disposer = GarbageDisposer {
        receiver: box_receiver,
        backlog,
    };
    (chute, disposer)
}
