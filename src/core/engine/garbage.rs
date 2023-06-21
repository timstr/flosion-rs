use std::sync::{
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
}

impl<'ctx> GarbageChute<'ctx> {
    pub(crate) fn send_box(&self, item: Box<dyn 'ctx + Droppable>) {
        self.sender.send(WrappedDroppable::Box(item)).unwrap();
    }

    pub(crate) fn send_arc(&self, item: Arc<dyn 'ctx + Droppable>) {
        self.sender.send(WrappedDroppable::Arc(item)).unwrap();
    }
}

pub(crate) struct GarbageDisposer<'ctx> {
    receiver: Receiver<WrappedDroppable<'ctx>>,
}

impl<'ctx> GarbageDisposer<'ctx> {
    pub(crate) fn clear(&self) {
        while let Ok(item) = self.receiver.try_recv() {
            std::mem::drop(item);
        }
    }
}

pub(crate) fn new_garbage_disposer<'ctx>() -> (GarbageChute<'ctx>, GarbageDisposer<'ctx>) {
    let bound = 1024;
    let (box_sender, box_receiver) = sync_channel(bound);
    let chute = GarbageChute { sender: box_sender };
    let disposer = GarbageDisposer {
        receiver: box_receiver,
    };
    (chute, disposer)
}
