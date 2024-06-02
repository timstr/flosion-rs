use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[cfg(debug_assertions)]
use std::any::type_name;

use eframe::egui;

use crate::core::sound::{soundgraphid::SoundObjectId, soundgraphtopology::SoundGraphTopology};

use super::{
    expressiongraphui::ExpressionGraphUi,
    graph_ui::{ObjectUiData, ObjectUiState},
    object_ui::Color,
    object_ui_states::AnyObjectUiState,
    soundgraphui::SoundGraphUi,
    soundgraphuicontext::SoundGraphUiContext,
    soundgraphuistate::SoundGraphUiState,
    ui_factory::UiFactory,
};

pub struct AnySoundObjectUiData {
    id: SoundObjectId,
    state: RefCell<Box<dyn AnyObjectUiState>>,
    color: Color,
}

impl ObjectUiData for AnySoundObjectUiData {
    type GraphUi = SoundGraphUi;
    type ConcreteType<'a, T: ObjectUiState> = SoundObjectUiData<'a, T>;

    type RequiredData = Color;

    fn new<S: ObjectUiState>(id: SoundObjectId, state: S, data: Self::RequiredData) -> Self {
        AnySoundObjectUiData {
            id,
            state: RefCell::new(Box::new(state)),
            color: data,
        }
    }

    fn downcast_with<
        T: ObjectUiState,
        F: FnOnce(SoundObjectUiData<'_, T>, &mut SoundGraphUiState, &mut SoundGraphUiContext<'_>),
    >(
        &self,
        ui_state: &mut SoundGraphUiState,
        ctx: &mut SoundGraphUiContext<'_>,
        f: F,
    ) {
        let mut state_mut = self.state.borrow_mut();
        #[cfg(debug_assertions)]
        {
            let actual_name = state_mut.get_language_type_name();
            let state_any = state_mut.as_mut_any();
            debug_assert!(
                state_any.is::<T>(),
                "AnySoundObjectUiData expected to receive state type {}, but got {:?} instead",
                type_name::<T>(),
                actual_name
            );
        }
        let state_any = state_mut.as_mut_any();
        let state = state_any.downcast_mut::<T>().unwrap();
        let color = ctx.object_states().get_object_color(self.id);
        f(SoundObjectUiData { state, color }, ui_state, ctx);
    }
}

pub struct SoundObjectUiData<'a, T: ObjectUiState> {
    pub state: &'a mut T,
    pub color: egui::Color32,
}

pub struct SoundObjectUiStates {
    data: HashMap<SoundObjectId, Rc<AnySoundObjectUiData>>,
}

impl SoundObjectUiStates {
    pub(super) fn new() -> SoundObjectUiStates {
        SoundObjectUiStates {
            data: HashMap::new(),
        }
    }

    pub(super) fn set_object_data(&mut self, id: SoundObjectId, state: AnySoundObjectUiData) {
        self.data.insert(id, Rc::new(state));
    }

    pub(super) fn get_object_data(&self, id: SoundObjectId) -> Rc<AnySoundObjectUiData> {
        Rc::clone(self.data.get(&id).unwrap())
    }

    pub(super) fn get_object_color(&self, id: SoundObjectId) -> egui::Color32 {
        self.data.get(&id).unwrap().color.color
    }

    pub(super) fn cleanup(&mut self, topo: &SoundGraphTopology) {
        self.data.retain(|i, _| match i {
            SoundObjectId::Sound(spid) => topo.sound_processors().contains_key(spid),
        });
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, topo: &SoundGraphTopology) -> bool {
        use crate::core::uniqueid::UniqueId;

        let mut good = true;
        for i in topo.sound_processors().keys() {
            if !self.data.contains_key(&i.into()) {
                println!("Sound processor {} does not have a ui state", i.value());
                good = false;
            }
        }
        for i in self.data.keys() {
            match i {
                SoundObjectId::Sound(i) => {
                    if !topo.sound_processors().contains_key(i) {
                        println!(
                            "A ui state exists for non-existent sound processor {}",
                            i.value()
                        );
                        good = false;
                    }
                }
            }
        }
        // TODO: invariant check for expression object states
        good
    }

    pub(super) fn create_state_for(
        &mut self,
        object_id: SoundObjectId,
        topo: &SoundGraphTopology,
        ui_factory: &UiFactory<SoundGraphUi>,
        expression_ui_factory: &UiFactory<ExpressionGraphUi>,
    ) {
        self.data.entry(object_id).or_insert_with(|| {
            let graph_object = topo.graph_object(object_id).unwrap();
            Rc::new(ui_factory.create_default_state(&graph_object))
        });
    }
}
