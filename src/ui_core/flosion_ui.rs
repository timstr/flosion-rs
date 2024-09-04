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
    },
    ui_objects::all_objects::{all_expression_graph_objects, all_sound_graph_objects},
};
use eframe::{
    self,
    egui::{self},
};
use hashrevise::{Revisable, RevisionHash};
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

/// The very root of the GUI, which manages a SoundGraph instance,
/// responds to inputs, and draws the up-to-date ui via egui
pub struct FlosionApp<'ctx> {
    /// The sound graph currently being used
    graph: SoundGraph,

    /// Factories for instantiating sound and expression objects and their uis
    factories: Factories,

    /// The state of the uis of all sound processors and their component uis
    ui_state: SoundGraphUiState,

    /// The on-screen layout of sound processors
    graph_layout: StackedLayout,

    properties: GraphProperties,

    previous_clean_revision: Option<RevisionHash>,

    audio_thread: Option<ScopedJoinHandle<'ctx, ()>>,

    stop_button: StopButton,

    engine_interface: SoundEngineInterface<'ctx>,

    garbage_disposer: GarbageDisposer<'ctx>,

    jit_cache: JitCache<'ctx>,
}

impl<'ctx> FlosionApp<'ctx> {
    pub fn new(
        _cc: &eframe::CreationContext,
        inkwell_context: &'ctx inkwell::context::Context,
        scope: &'ctx thread::Scope<'ctx, '_>,
    ) -> FlosionApp<'ctx> {
        let graph = SoundGraph::new();

        let properties = GraphProperties::new(graph.topology());

        let stop_button = StopButton::new();

        let (engine_interface, engine, garbage_disposer) = create_sound_engine(&stop_button);

        let audio_thread = scope.spawn(move || {
            set_current_thread_priority(ThreadPriority::Max).unwrap();
            engine.run();
        });

        let jit_cache = JitCache::new(inkwell_context);

        let mut app = FlosionApp {
            graph,
            factories: Factories::new(),
            ui_state: SoundGraphUiState::new(),
            graph_layout: StackedLayout::new(),
            properties,
            previous_clean_revision: None,
            audio_thread: Some(audio_thread),
            stop_button,
            engine_interface,
            garbage_disposer,
            jit_cache,
        };

        // Initialize all necessary ui state
        app.cleanup();

        #[cfg(debug_assertions)]
        app.check_invariants();

        app
    }

    fn interact_and_draw(&mut self, ui: &mut egui::Ui) {
        self.graph_layout.draw(
            ui,
            &self.factories,
            &mut self.ui_state,
            &mut self.graph,
            &self.properties,
            &self.jit_cache,
        );

        self.ui_state.interact_and_draw(
            ui,
            &self.factories,
            &mut self.graph,
            &self.properties,
            &mut self.graph_layout,
        );
    }

    fn cleanup(&mut self) {
        let topo = self.graph.topology();

        self.properties.refresh(topo);

        let current_revision = topo.get_revision();

        if self.previous_clean_revision != Some(current_revision) {
            self.graph_layout
                .regenerate(topo, self.ui_state.positions());

            self.ui_state
                .cleanup_stale_graph_objects(topo, &self.factories);

            self.previous_clean_revision = Some(current_revision);
        }

        self.ui_state.cleanup_frame_data();
    }

    #[cfg(debug_assertions)]
    fn check_invariants(&self) {
        assert!(self.ui_state.check_invariants(self.graph.topology()));
        assert!(self.graph_layout.check_invariants(self.graph.topology()));
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
                .update(self.graph.topology(), &self.jit_cache)
                .expect("Failed to update engine");

            self.garbage_disposer.clear();
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_button.stop();
        self.audio_thread.take().unwrap().join().unwrap();
    }
}
