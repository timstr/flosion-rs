use eframe::egui;

use crate::{
    core::graphobject::ObjectId,
    objects::functions::{Add, Constant, Divide, Multiply, Negate, Sine, Subtract, UnitSine},
    ui_core::{
        arguments::{ArgumentList, ArgumentValue, ParsedArguments},
        graph_ui_state::GraphUIState,
        object_ui::{NumberInputWidget, NumberOutputWidget, ObjectUi, ObjectWindow},
    },
};

#[derive(Default)]
pub struct ConstantUi {}

pub struct ConstantUiState {
    min_value: f32,
    max_value: f32,
}

impl Default for ConstantUiState {
    fn default() -> Self {
        Self {
            min_value: 0.0,
            max_value: 1.0,
        }
    }
}

impl ObjectUi for ConstantUi {
    type WrapperType = Constant;
    type StateType = ConstantUiState;
    fn ui(
        &self,
        id: ObjectId,
        object: &Constant,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        state: &ConstantUiState,
    ) {
        let id = id.as_number_source_id().unwrap();
        ObjectWindow::new_number_source(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            let mut v = object.get_value();
            let v_old = v;
            ui.label(format!("Constant, value={}", v));
            ui.add(egui::Slider::new(&mut v, state.min_value..=state.max_value));
            if v != v_old {
                object.set_value(v);
            }
            ui.add(NumberOutputWidget::new(id, "Output", graph_tools));
        });
    }

    fn arguments(&self) -> ArgumentList {
        let mut args = ArgumentList::new();
        args.add("value", ArgumentValue::Float(0.0));
        args.add("min", ArgumentValue::Float(0.0));
        args.add("max", ArgumentValue::Float(1.0));
        args
    }

    fn init_object(&self, object: &Constant, args: &ParsedArguments) {
        object.set_value(args.get("value").as_float());
    }

    fn make_state(&self, args: &ParsedArguments) -> Self::StateType {
        ConstantUiState {
            min_value: args.get("min").as_float(),
            max_value: args.get("max").as_float(),
        }
    }
}

macro_rules! unary_number_source_ui {
    ($name: ident, $object: ident, $display_name: literal, $aliases: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ObjectUi for $name {
            type WrapperType = $object;
            type StateType = ();
            fn ui(
                &self,
                id: ObjectId,
                object: &$object,
                graph_tools: &mut GraphUIState,
                ui: &mut eframe::egui::Ui,
                _state: &(),
            ) {
                let id = id.as_number_source_id().unwrap();
                ObjectWindow::new_number_source(id).show(
                    ui.ctx(),
                    graph_tools,
                    |ui, graph_tools| {
                        ui.label($display_name);
                        ui.add(NumberInputWidget::new(
                            object.input.id(),
                            "Input",
                            graph_tools,
                        ));
                        ui.add(NumberOutputWidget::new(id, "Output", graph_tools));
                    },
                );
            }

            fn aliases(&self) -> &'static [&'static str] {
                &$aliases
            }
        }
    };
}

macro_rules! binary_number_source_ui {
    ($name: ident, $object: ident, $display_name: literal, $aliases: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ObjectUi for $name {
            type WrapperType = $object;
            type StateType = ();
            fn ui(
                &self,
                id: ObjectId,
                object: &$object,
                graph_tools: &mut GraphUIState,
                ui: &mut eframe::egui::Ui,
                _state: &(),
            ) {
                let id = id.as_number_source_id().unwrap();
                ObjectWindow::new_number_source(id).show(
                    ui.ctx(),
                    graph_tools,
                    |ui, graph_tools| {
                        ui.label($display_name);
                        ui.add(NumberInputWidget::new(
                            object.input_1.id(),
                            "Input 1",
                            graph_tools,
                        ));
                        ui.add(NumberInputWidget::new(
                            object.input_2.id(),
                            "Input 2",
                            graph_tools,
                        ));
                        ui.add(NumberOutputWidget::new(id, "Output", graph_tools));
                    },
                );
            }

            fn aliases(&self) -> &'static [&'static str] {
                &$aliases
            }
        }
    };
}

unary_number_source_ui!(NegateUi, Negate, "Negate", []);

binary_number_source_ui!(AddUi, Add, "Add", ["+", "plus"]);
binary_number_source_ui!(SubtractUi, Subtract, "Subtract", ["-", "minus"]);
binary_number_source_ui!(MultiplyUi, Multiply, "Multiply", ["*", "times"]);
binary_number_source_ui!(DivideUi, Divide, "Divide", ["/"]);
unary_number_source_ui!(SineUi, Sine, "sin", []);
unary_number_source_ui!(UnitSineUi, UnitSine, "usin", []);

// #[derive(Default)]
// pub struct SineUi {}

// impl ObjectUi for SineUi {
//     type WrapperType = Sine;
//     type Stat
//     fn ui(
//         &self,
//         id: ObjectId,
//         object: &Sine,
//         graph_tools: &mut GraphUIState,
//         ui: &mut eframe::egui::Ui,
//     ) {
//         let id = id.as_number_source_id().unwrap();
//         ObjectWindow::new_number_source(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
//             ui.label("Sine");
//             ui.add(NumberInputWidget::new(
//                 object.input.id(),
//                 "Input",
//                 graph_tools,
//             ));
//             ui.add(NumberOutputWidget::new(id, "Output", graph_tools));
//         });
//     }

//     fn aliases(&self) -> &'static [&'static str] {
//         &["sin"]
//     }
// }

// #[derive(Default)]
// pub struct UnitSineUi {}

// impl ObjectUi for UnitSineUi {
//     type WrapperType = UnitSine;
//     fn ui(
//         &self,
//         id: ObjectId,
//         object: &UnitSine,
//         graph_tools: &mut GraphUIState,
//         ui: &mut eframe::egui::Ui,
//     ) {
//         let id = id.as_number_source_id().unwrap();
//         ObjectWindow::new_number_source(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
//             ui.label("UnitSine");
//             ui.add(NumberInputWidget::new(
//                 object.input.id(),
//                 "Input",
//                 graph_tools,
//             ));
//             ui.add(NumberOutputWidget::new(id, "Output", graph_tools));
//         });
//     }

//     fn aliases(&self) -> &'static [&'static str] {
//         &["usin"]
//     }
// }
