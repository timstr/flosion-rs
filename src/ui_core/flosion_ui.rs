use crate::{
    core::soundgraph::SoundGraph,
    objects::{
        dac::Dac,
        functions::{Constant, UnitSine},
        wavegenerator::WaveGenerator,
    },
    ui_objects::all_objects::AllObjectUis,
};
use eframe::{egui, epi};
use futures::executor::block_on;

pub struct FlosionApp {
    graph: SoundGraph,
    all_object_uis: AllObjectUis,
}

async fn create_test_sound_graph() -> SoundGraph {
    let mut sg: SoundGraph = SoundGraph::new();
    let wavegen = sg.add_dynamic_sound_processor::<WaveGenerator>().await;
    let dac = sg.add_static_sound_processor::<Dac>().await;
    let dac_input_id = dac.instance().input().id();
    let constant = sg.add_number_source::<Constant>().await;
    let usine = sg.add_number_source::<UnitSine>().await;
    sg.connect_number_input(wavegen.instance().amplitude.id(), usine.id())
        .await
        .unwrap();
    sg.connect_number_input(usine.instance().input.id(), wavegen.instance().phase.id())
        .await
        .unwrap();
    sg.connect_number_input(wavegen.instance().frequency.id(), constant.id())
        .await
        .unwrap();
    constant.instance().set_value(440.0);
    sg.connect_sound_input(dac_input_id, wavegen.id())
        .await
        .unwrap();
    sg
}

impl Default for FlosionApp {
    fn default() -> FlosionApp {
        let graph = block_on(create_test_sound_graph());
        FlosionApp {
            graph,
            all_object_uis: AllObjectUis::new(),
        }
    }
}

impl epi::App for FlosionApp {
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hi earthguy");
            let running = self.graph.is_running();
            if ui.button(if running { "Pause" } else { "Play" }).clicked() {
                if running {
                    self.graph.stop();
                } else {
                    self.graph.start();
                }
            }
            for o in self.graph.graph_objects() {
                self.all_object_uis.ui(o.as_ref(), o.get_type(), ui);
            }
        });
    }

    fn name(&self) -> &str {
        "Flosion"
    }
}
