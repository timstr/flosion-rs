use std::thread::{self, ScopedJoinHandle};

use crate::{
    core::{
        engine::{
            garbage::GarbageDisposer,
            soundengine::{create_sound_engine, SoundEngineInterface, StopButton},
        },
        expression::expressionobject::ExpressionObjectFactory,
        jit::cache::JitCache,
        sound::{soundgraph::SoundGraph, soundobject::SoundObjectFactory},
        stashing::StashingContext,
    },
    ui_objects::all_objects::{all_expression_graph_objects, all_sound_graph_objects},
};
use eframe::{
    self,
    egui::{self},
};
use hashstash::{ObjectHash, Stash};
use thread_priority::{set_current_thread_priority, ThreadPriority};

use super::{
    expressionobjectui::ExpressionObjectUiFactory, graph_properties::GraphProperties,
    soundgraphuistate::SoundGraphUiState, soundobjectui::SoundObjectUiFactory,
    stackedlayout::stackedlayout::StackedLayout,
};

/// Convenience struct for passing all the different factories together
pub(crate) struct Factories {
    sound_objects: SoundObjectFactory,
    expression_objects: ExpressionObjectFactory,
    sound_uis: SoundObjectUiFactory,
    expression_uis: ExpressionObjectUiFactory,
}

impl Factories {
    /// Creates a new set of factories pre-filled with all statically registered types
    pub(crate) fn new() -> Factories {
        let (object_factory, ui_factory) = all_sound_graph_objects();
        let (expression_object_factory, expression_ui_factory) = all_expression_graph_objects();

        Factories {
            sound_objects: object_factory,
            expression_objects: expression_object_factory,
            sound_uis: ui_factory,
            expression_uis: expression_ui_factory,
        }
    }

    pub(crate) fn sound_objects(&self) -> &SoundObjectFactory {
        &self.sound_objects
    }

    pub(crate) fn expression_objects(&self) -> &ExpressionObjectFactory {
        &self.expression_objects
    }

    pub(crate) fn sound_uis(&self) -> &SoundObjectUiFactory {
        &self.sound_uis
    }

    pub(crate) fn expression_uis(&self) -> &ExpressionObjectUiFactory {
        &self.expression_uis
    }
}

pub(crate) struct AppState {
    /// The sound graph currently being used
    graph: SoundGraph,

    /// The state of the uis of all sound processors and their component uis
    ui_state: SoundGraphUiState,

    /// The on-screen layout of sound processors
    graph_layout: StackedLayout,

    properties: GraphProperties,

    previous_clean_revision: Option<ObjectHash>,
}

impl AppState {
    fn interact_and_draw(
        &mut self,
        ui: &mut egui::Ui,
        factories: &Factories,
        jit_cache: &JitCache,
        stash: &Stash,
    ) {
        self.graph_layout.draw(
            ui,
            factories,
            &mut self.ui_state,
            &mut self.graph,
            &self.properties,
            jit_cache,
            stash,
        );

        self.ui_state.interact_and_draw(
            ui,
            factories,
            &mut self.graph,
            &self.properties,
            &mut self.graph_layout,
            stash,
        );
    }

    fn cleanup(&mut self, factories: &Factories) {
        self.properties.refresh(&self.graph);

        let current_revision = ObjectHash::from_stashable_and_context(
            &self.graph,
            &StashingContext::new_checking_recompilation(),
        );

        if self.previous_clean_revision != Some(current_revision) {
            self.graph_layout
                .regenerate(&self.graph, self.ui_state.positions());

            self.ui_state.cleanup(&self.graph, factories);

            self.previous_clean_revision = Some(current_revision);
        }

        self.ui_state.cleanup_frame_data();
    }

    #[cfg(debug_assertions)]
    fn check_invariants(&self) {
        assert_eq!(self.graph.validate(), Ok(()));
        self.ui_state.check_invariants(&self.graph);
        assert!(self.graph_layout.check_invariants(&self.graph));
    }
}

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
        let graph = SoundGraph::new();

        let properties = GraphProperties::new(&graph);

        let stop_button = StopButton::new();

        let (engine_interface, engine, garbage_disposer) = create_sound_engine(&stop_button);

        let audio_thread = scope.spawn(move || {
            set_current_thread_priority(ThreadPriority::Max).unwrap();
            engine.run();
        });

        let jit_cache = JitCache::new(inkwell_context);

        let state = AppState {
            graph,
            ui_state: SoundGraphUiState::new(),
            graph_layout: StackedLayout::new(),
            properties,
            previous_clean_revision: None,
        };

        let mut app = FlosionApp {
            state,
            factories: Factories::new(),
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

        self.jit_cache.refresh(&self.state.graph);
    }

    #[cfg(debug_assertions)]
    fn check_invariants(&self) {
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
                    &self.state.graph,
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
