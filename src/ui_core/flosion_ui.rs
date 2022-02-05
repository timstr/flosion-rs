use eframe::{egui, epi};

struct NodeState {
    text: String,
    position: Option<egui::Pos2>,
}

pub struct FlosionApp {
    nodes: Vec<NodeState>,
    node_connections: Vec<(usize, usize)>,
}

impl Default for FlosionApp {
    fn default() -> FlosionApp {
        FlosionApp {
            nodes: (1..=5)
                .map(|i| NodeState {
                    text: format!("Node {}", i),
                    position: None,
                })
                .collect(),
            node_connections: vec![(0, 1), (0, 2), (2, 3), (3, 4)],
        }
    }
}

impl epi::App for FlosionApp {
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hi earthguy");
            self.nodes.iter_mut().for_each(|node| {
                let r = egui::Window::new(&node.text)
                    .title_bar(false)
                    .resizable(false)
                    .frame(
                        egui::Frame::none()
                            .fill(egui::Color32::DARK_BLUE)
                            .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                            .margin(egui::Vec2::splat(10.0)),
                    )
                    .show(ctx, |ui| ui.label(&node.text))
                    .unwrap();
                let new_position = r.response.rect.min;

                let position_changed = (|| -> bool {
                    let p = match node.position {
                        None => return true,
                        Some(p) => p,
                    };
                    p != new_position
                })();

                if position_changed {
                    println!(
                        "\"{}\" was moved to ({}, {})",
                        node.text, new_position.x, new_position.y
                    );
                }

                node.position = Some(new_position);
            });

            let stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);

            for (n1, n2) in &self.node_connections {
                let p1 = self.nodes[*n1].position.unwrap();
                let p2 = self.nodes[*n2].position.unwrap();
                ui.painter().line_segment([p1, p2], stroke);
            }
        });
    }

    fn name(&self) -> &str {
        "Flosion"
    }
}
