use std::{any::Any, cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

use eframe::egui;

use crate::core::{expression::expressionobject::ExpressionObject, objecttype::ObjectType};

use super::{
    arguments::{ArgumentList, ParsedArguments},
    expressiongraphuicontext::ExpressionGraphUiContext,
    expressiongraphuistate::ExpressionGraphUiState,
    lexicallayout::lexicallayout::ExpressionNodeLayout,
    object_ui::ObjectUiState,
};

pub trait ExpressionObjectUi: Default {
    type ObjectType: ExpressionObject;
    type StateType: ObjectUiState;

    fn ui<'a>(
        &self,
        object: &mut Self::ObjectType,
        graph_ui_state: &mut ExpressionGraphUiState,
        ui: &mut egui::Ui,
        ctx: &ExpressionGraphUiContext,
        state: &mut Self::StateType,
    );

    fn summon_names(&self) -> &'static [&'static str];

    fn summon_arguments(&self) -> ArgumentList {
        ArgumentList::new_empty()
    }

    fn make_properties(&self) -> ExpressionNodeLayout;

    fn make_ui_state(
        &self,
        _handle: &Self::ObjectType,
        _args: ParsedArguments,
    ) -> Result<Self::StateType, ()>;
}

pub trait AnyExpressionObjectUi {
    fn apply(
        &self,
        object: &mut dyn ExpressionObject,
        state: &mut dyn Any,
        graph_state: &mut ExpressionGraphUiState,
        ui: &mut egui::Ui,
        ctx: &ExpressionGraphUiContext,
    );

    fn summon_names(&self) -> &'static [&'static str];

    fn summon_arguments(&self) -> ArgumentList;

    fn object_type(&self) -> ObjectType;

    fn make_properties(&self) -> ExpressionNodeLayout;

    // TODO: remove result here
    fn make_ui_state(
        &self,
        object: &dyn ExpressionObject,
        args: ParsedArguments,
    ) -> Result<Rc<RefCell<dyn Any>>, ()>;
}

impl<T: 'static + ExpressionObjectUi> AnyExpressionObjectUi for T {
    fn apply(
        &self,
        object: &mut dyn ExpressionObject,
        state: &mut dyn Any,
        graph_ui_state: &mut ExpressionGraphUiState,
        ui: &mut egui::Ui,
        ctx: &ExpressionGraphUiContext,
    ) {
        let object = object.as_mut_any().downcast_mut::<T::ObjectType>().unwrap();
        self.ui(
            object,
            graph_ui_state,
            ui,
            ctx,
            state.downcast_mut().unwrap(),
        );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        self.summon_names()
    }

    fn summon_arguments(&self) -> ArgumentList {
        T::summon_arguments(self)
    }

    fn object_type(&self) -> ObjectType {
        <T::ObjectType as ExpressionObject>::get_type()
    }

    fn make_properties(&self) -> ExpressionNodeLayout {
        T::make_properties(&self)
    }

    fn make_ui_state(
        &self,
        object: &dyn ExpressionObject,
        args: ParsedArguments,
    ) -> Result<Rc<RefCell<dyn Any>>, ()> {
        let object = object.as_any().downcast_ref::<T::ObjectType>().unwrap();
        let state = self.make_ui_state(&object, args)?;
        Ok(Rc::new(RefCell::new(state)))
    }
}

pub(crate) struct ExpressionObjectUiFactory {
    mapping: HashMap<ObjectType, Box<dyn AnyExpressionObjectUi>>,
}

impl ExpressionObjectUiFactory {
    pub(crate) fn new_empty() -> ExpressionObjectUiFactory {
        ExpressionObjectUiFactory {
            mapping: HashMap::new(),
        }
    }

    pub(crate) fn register<T: 'static + ExpressionObjectUi>(&mut self) {
        let instance = T::default();
        let object_type = instance.object_type();
        self.mapping.insert(object_type, Box::new(instance));
    }

    pub(crate) fn get(&self, object_type: ObjectType) -> &dyn AnyExpressionObjectUi {
        self.mapping
            .get(&object_type)
            .unwrap_or_else(|| {
                panic!(
                "Tried to create a ui for an expression graph object of unrecognized type \"{}\"",
                object_type.name(),
            )
            })
            .deref()
    }

    pub(crate) fn all_object_uis(&self) -> impl Iterator<Item = &dyn AnyExpressionObjectUi> {
        self.mapping.values().map(|b| b.deref())
    }
}

pub(crate) fn show_expression_node_ui(
    factory: &ExpressionObjectUiFactory,
    object: &mut dyn ExpressionObject,
    ui_state: &mut ExpressionGraphUiState,
    ui: &mut egui::Ui,
    ctx: &ExpressionGraphUiContext,
) {
    let object_type = object.get_dynamic_type();

    let object_ui = factory.get(object_type);

    let state = ui_state.object_states().get_object_data(object.id());
    let state: &mut dyn Any = &mut *state.borrow_mut();
    object_ui.apply(object, state, ui_state, ui, ctx);
}
