use std::rc::Rc;

use crate::core::{
    expression::{
        expressiongraph::{ExpressionGraph, ExpressionGraphParameterId, ExpressionGraphResultId},
        expressionnode::ExpressionNodeId,
    },
    jit::server::JitClient,
    sound::{
        soundgraph::SoundGraph, soundgrapherror::SoundError, expression::SoundExpressionId,
        expressionargument::SoundExpressionArgumentId, soundprocessor::SoundProcessorId,
    },
};

use super::{
    graph_ui::GraphUiContext,
    numbergraphui::NumberGraphUi,
    numbergraphuistate::{AnyNumberObjectUiData, NumberObjectUiStates},
    soundgraphuinames::SoundGraphUiNames,
    temporallayout::{SoundGraphLayout, TimeAxis},
    ui_factory::UiFactory,
};

pub(crate) struct OuterSoundNumberInputContext<'a> {
    sound_number_input_id: SoundExpressionId,
    parent_sound_processor_id: SoundProcessorId,
    graph_layout: &'a SoundGraphLayout,
    sound_graph: &'a mut SoundGraph,
    sound_graph_names: &'a SoundGraphUiNames,
    jit_client: &'a JitClient,
    time_axis: TimeAxis,
}

impl<'a> OuterSoundNumberInputContext<'a> {
    pub(super) fn new(
        sound_number_input_id: SoundExpressionId,
        parent_sound_processor_id: SoundProcessorId,
        graph_layout: &'a SoundGraphLayout,
        sound_graph: &'a mut SoundGraph,
        sound_graph_names: &'a SoundGraphUiNames,
        jit_client: &'a JitClient,
        time_axis: TimeAxis,
    ) -> Self {
        Self {
            sound_number_input_id,
            parent_sound_processor_id,
            graph_layout,
            sound_graph,
            sound_graph_names,
            jit_client,
            time_axis,
        }
    }

    pub(super) fn sound_number_input_id(&self) -> SoundExpressionId {
        self.sound_number_input_id
    }

    pub(super) fn parent_sound_processor_id(&self) -> SoundProcessorId {
        self.parent_sound_processor_id
    }

    pub(super) fn graph_layout(&self) -> &SoundGraphLayout {
        self.graph_layout
    }

    pub(crate) fn sound_graph(&self) -> &SoundGraph {
        self.sound_graph
    }

    pub(crate) fn sound_graph_mut(&mut self) -> &mut SoundGraph {
        self.sound_graph
    }

    pub(crate) fn sound_graph_names(&self) -> &SoundGraphUiNames {
        self.sound_graph_names
    }

    pub(crate) fn jit_client(&self) -> &JitClient {
        self.jit_client
    }

    pub(crate) fn time_axis(&self) -> &TimeAxis {
        &self.time_axis
    }

    pub(crate) fn find_graph_id_for_number_source(
        &self,
        nsid: SoundExpressionArgumentId,
    ) -> Option<ExpressionGraphParameterId> {
        self.sound_graph
            .topology()
            .expression(self.sound_number_input_id)
            .unwrap()
            .parameter_mapping()
            .parameter_from_argument(nsid)
    }

    pub(crate) fn connect_to_number_source(
        &mut self,
        nsid: SoundExpressionArgumentId,
    ) -> ExpressionGraphParameterId {
        self.sound_graph
            .edit_expression(self.sound_number_input_id, |ni_data| {
                let (numbergraph, mapping) = ni_data.expression_graph_and_mapping_mut();
                mapping.add_argument(nsid, numbergraph)
            })
            .unwrap()
    }

    pub(crate) fn disconnect_from_number_source(&mut self, nsid: SoundExpressionArgumentId) {
        self.sound_graph
            .edit_expression(self.sound_number_input_id, |ni_data| {
                let (numbergraph, mapping) = ni_data.expression_graph_and_mapping_mut();
                mapping.remove_argument(nsid, numbergraph);
            })
            .unwrap();
    }
}

pub(crate) enum OuterNumberGraphUiContext<'a> {
    // TODO: top level number graph/function also
    SoundNumberInput(OuterSoundNumberInputContext<'a>),
}

impl<'a> From<OuterSoundNumberInputContext<'a>> for OuterNumberGraphUiContext<'a> {
    fn from(value: OuterSoundNumberInputContext<'a>) -> Self {
        OuterNumberGraphUiContext::SoundNumberInput(value)
    }
}

impl<'a> OuterNumberGraphUiContext<'a> {
    pub(crate) fn graph_input_name(&self, input_id: ExpressionGraphParameterId) -> String {
        match self {
            OuterNumberGraphUiContext::SoundNumberInput(ctx) => {
                let nsid = ctx
                    .sound_graph()
                    .topology()
                    .expression(ctx.sound_number_input_id())
                    .unwrap()
                    .parameter_mapping()
                    .argument_from_parameter(input_id)
                    .unwrap();
                ctx.sound_graph_names().combined_number_source_name(nsid)
            }
        }
    }

    pub(crate) fn graph_output_name(&self, output_id: ExpressionGraphResultId) -> String {
        match self {
            OuterNumberGraphUiContext::SoundNumberInput(ctx) => {
                assert!(self.inspect_number_graph(|g| {
                    let outputs = g.topology().results();
                    assert_eq!(outputs.len(), 1);
                    outputs[0].id() == output_id
                }));
                ctx.sound_graph_names()
                    .number_input(ctx.sound_number_input_id())
                    .unwrap()
                    .name()
                    .to_string()
            }
        }
    }

    pub(crate) fn inspect_number_graph<R, F: FnOnce(&ExpressionGraph) -> R>(&self, f: F) -> R {
        match self {
            OuterNumberGraphUiContext::SoundNumberInput(ctx) => f(ctx
                .sound_graph()
                .topology()
                .expression(ctx.sound_number_input_id())
                .unwrap()
                .expression_graph()),
        }
    }

    pub(crate) fn edit_number_graph<R, F: FnOnce(&mut ExpressionGraph) -> R>(
        &mut self,
        f: F,
    ) -> Result<R, SoundError> {
        match self {
            OuterNumberGraphUiContext::SoundNumberInput(ctx) => {
                let niid = ctx.sound_number_input_id();
                ctx.sound_graph_mut()
                    .edit_expression(niid, |ni_data| f(ni_data.expression_graph_mut()))
            }
        }
    }

    pub(crate) fn remove_graph_input(&mut self, giid: ExpressionGraphParameterId) {
        match self {
            OuterNumberGraphUiContext::SoundNumberInput(ctx) => {
                let niid = ctx.sound_number_input_id();
                ctx.sound_graph_mut()
                    .edit_expression(niid, |ni_data| {
                        let (numbergraph, mapping) = ni_data.expression_graph_and_mapping_mut();
                        let source_id = mapping.argument_from_parameter(giid).unwrap();
                        mapping.remove_argument(source_id, numbergraph);
                    })
                    .unwrap();
            }
        }
    }
}

pub struct NumberGraphUiContext<'a> {
    ui_factory: &'a UiFactory<NumberGraphUi>,
    object_states: &'a NumberObjectUiStates,
}

impl<'a> NumberGraphUiContext<'a> {
    pub(super) fn new(
        ui_factory: &'a UiFactory<NumberGraphUi>,
        object_states: &'a NumberObjectUiStates,
    ) -> NumberGraphUiContext<'a> {
        NumberGraphUiContext {
            ui_factory,
            object_states,
        }
    }

    pub(super) fn ui_factory(&self) -> &UiFactory<NumberGraphUi> {
        self.ui_factory
    }

    pub(super) fn object_ui_states(&self) -> &NumberObjectUiStates {
        self.object_states
    }
}

impl<'a> GraphUiContext<'a> for NumberGraphUiContext<'a> {
    type GraphUi = NumberGraphUi;

    fn get_object_ui_data(&self, id: ExpressionNodeId) -> Rc<AnyNumberObjectUiData> {
        self.object_states.get_object_data(id)
    }
}
