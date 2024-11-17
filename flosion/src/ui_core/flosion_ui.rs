use std::thread::{self, ScopedJoinHandle};

use crate::core::{
    engine::{
        garbage::GarbageDisposer,
        soundengine::{create_sound_engine, SoundEngineInterface, StopButton},
    },
    jit::cache::JitCache,
    sound::soundgraph::SoundGraph,
};
use eframe::{
    self,
    egui::{self, Key, KeyboardShortcut, Modifiers},
};
use hashstash::Stash;
use thread_priority::{set_current_thread_priority, ThreadPriority};

use super::{
    appstate::AppState,
    factories::Factories,
    history::{History, SnapshotFlag},
    view::View,
};

/// The very root of the GUI, which manages a SoundGraph instance,
/// responds to inputs, and draws the up-to-date ui via egui
pub struct FlosionApp<'ctx> {
    graph: SoundGraph,

    state: AppState,

    /// Factories for instantiating sound and expression objects and their uis
    factories: Factories,

    /// Undo/redo history
    history: History,

    audio_thread: Option<ScopedJoinHandle<'ctx, ()>>,

    stop_button: StopButton,

    engine_interface: SoundEngineInterface<'ctx>,

    garbage_disposer: GarbageDisposer<'ctx>,

    jit_cache: JitCache<'ctx>,

    stash: Stash,

    view: View,
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

        let graph = SoundGraph::new();

        let state = AppState::new();

        let mut app = FlosionApp {
            graph,
            state,
            factories: Factories::new_all_objects(),
            history: History::new(),
            audio_thread: Some(audio_thread),
            stop_button,
            engine_interface,
            garbage_disposer,
            jit_cache,
            stash: Stash::new(),
            view: View::new(),
        };

        // Initialize all necessary ui state
        app.cleanup();

        #[cfg(debug_assertions)]
        app.check_invariants();

        // Add a history snapshot for the initial state
        app.history
            .push_snapshot(&app.stash, &app.graph, &app.state);

        app
    }

    fn interact_and_draw(&mut self, ui: &mut egui::Ui) {
        let snapshot_flag = SnapshotFlag::new();

        self.state.interact_and_draw(
            ui,
            &mut self.graph,
            &self.factories,
            &self.jit_cache,
            &self.stash,
            &snapshot_flag,
            &self.view,
        );

        self.cleanup();

        #[cfg(debug_assertions)]
        self.check_invariants();

        if snapshot_flag.snapshot_was_requested() {
            self.history
                .push_snapshot(&self.stash, &self.graph, &self.state);
        }

        let (ctrl_z, ctrl_y) = ui.input_mut(|i| {
            (
                i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Z)),
                i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Y)),
            )
        });

        if ctrl_z {
            self.history.undo(
                &self.stash,
                &self.factories,
                &mut self.graph,
                &mut self.state,
            );

            self.cleanup();
        }

        if ctrl_y {
            self.history.redo(
                &self.stash,
                &self.factories,
                &mut self.graph,
                &mut self.state,
            );
            self.cleanup();
        }

        self.view.handle_pan_and_zoom(ui);

        #[cfg(debug_assertions)]
        self.check_invariants();
    }

    fn cleanup(&mut self) {
        self.state.cleanup(&self.graph, &self.factories);
        self.jit_cache.refresh(&self.graph);
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self) {
        assert_eq!(self.graph.validate(), Ok(()));
        self.state.check_invariants(&self.graph);
        // TODO: all expressions are compiled in the jit cache?
    }
}

impl<'ctx> eframe::App for FlosionApp<'ctx> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            #[cfg(debug_assertions)]
            self.check_invariants();

            self.interact_and_draw(ui);

            self.engine_interface
                .update(
                    &self.graph,
                    &self.jit_cache,
                    &self.stash,
                    self.factories.sound_objects(),
                    self.factories.expression_objects(),
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
