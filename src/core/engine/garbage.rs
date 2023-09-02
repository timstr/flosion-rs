use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};

pub(crate) trait Garbage<'ctx> {
    fn toss(self, chute: &GarbageChute<'ctx>);
}

pub trait Droppable: Sync + Send {}

impl<T: Sync + Send> Droppable for T {}

enum WrappedDroppable<'ctx> {
    Box(Box<dyn 'ctx + Droppable>),
    Arc(Arc<dyn 'ctx + Droppable>),
}

pub(crate) struct GarbageChute<'ctx> {
    sender: SyncSender<WrappedDroppable<'ctx>>,
    backlog: Arc<AtomicUsize>,
    capacity: usize,
}

impl<'ctx> GarbageChute<'ctx> {
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

    pub(crate) fn send_arc(&self, item: Arc<dyn 'ctx + Droppable>) {
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

pub(crate) struct GarbageDisposer<'ctx> {
    receiver: Receiver<WrappedDroppable<'ctx>>,
    backlog: Arc<AtomicUsize>,
}

impl<'ctx> GarbageDisposer<'ctx> {
    pub(crate) fn clear(&self) {
        let mut count: usize = 0;
        while let Ok(item) = self.receiver.try_recv() {
            std::mem::drop(item);
            count += 1;
        }
        self.backlog.fetch_sub(count, Ordering::Relaxed);
    }
}

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
