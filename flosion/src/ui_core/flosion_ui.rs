use std::thread::{self, ScopedJoinHandle};

use crate::core::{
    engine::{
        garbage::GarbageDisposer,
        soundengine::{create_sound_engine, SoundEngineInterface, StopButton},
    },
    jit::cache::JitCache,
};
use eframe::{
    self,
    egui::{self},
};
use hashstash::Stash;
use thread_priority::{set_current_thread_priority, ThreadPriority};

use super::{appstate::AppState, factories::Factories};

/// The very root of the GUI, which manages a SoundGraph instance,
/// responds to inputs, and draws the up-to-date ui via egui
pub struct FlosionApp<'ctx> {
    state: AppState,

    /// Factories for instantiating sound and expression objects and their uis
    factories: Factories,

    audio_thread: Option<ScopedJoinHandle<'ctx, ()>>,

    stop_button: StopButton,

    engine_interface: SoundEngineInterface<'ctx>,

    garbage_disposer: GarbageDisposer<'ctx>,

    jit_cache: JitCache<'ctx>,

    stash: Stash,
}

impl<'ctx> FlosionApp<'ctx> {
    pub fn new(
        _cc: &eframe::CreationContext,
        inkwell_context: &'ctx inkwell::context::Context,
        scope: &'ctx thread::Scope<'ctx, '_>,
    ) -> FlosionApp<'ctx> {
        let stop_button = StopButton::new();

        let (engine_interface, engine, garbage_disposer) = create_sound_engine(&stop_button);

        let audio_thread = scope.spawn(move || {
            set_current_thread_priority(ThreadPriority::Max).unwrap();
            engine.run();
        });

        let jit_cache = JitCache::new(inkwell_context);

        let state = AppState::new();

        let mut app = FlosionApp {
            state,
            factories: Factories::new_all_objects(),
            audio_thread: Some(audio_thread),
            stop_button,
            engine_interface,
            garbage_disposer,
            jit_cache,
            stash: Stash::new(),
        };

        // Initialize all necessary ui state
        app.cleanup();

        #[cfg(debug_assertions)]
        app.check_invariants();

        app
    }

    fn interact_and_draw(&mut self, ui: &mut egui::Ui) {
        self.state
            .interact_and_draw(ui, &self.factories, &self.jit_cache, &self.stash);
    }

    fn cleanup(&mut self) {
        self.state.cleanup(&self.factories);

        self.jit_cache.refresh(self.state.graph());
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self) {
        self.state.check_invariants();
        // TODO: all expressions are compiled in the jit cache?
    }
}

impl<'ctx> eframe::App for FlosionApp<'ctx> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            #[cfg(debug_assertions)]
            self.check_invariants();

            self.interact_and_draw(ui);

            self.cleanup();

            #[cfg(debug_assertions)]
            self.check_invariants();

            self.engine_interface
                .update(
                    self.state.graph(),
                    &self.jit_cache,
                    &self.stash,
                    &self.factories,
                )
                .expect("Failed to update engine");

            self.garbage_disposer.clear();
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_button.stop();
        self.audio_thread.take().unwrap().join().unwrap();
    }
}
