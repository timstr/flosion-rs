use flosion::ui_core::flosion_ui::FlosionApp;
// use std::{panic, process};

fn main() {
    // let orig_hook = panic::take_hook();
    // panic::set_hook(Box::new(move |panic_info| {
    //     orig_hook(panic_info);
    //     process::exit(-1);
    // }));

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Flosion",
        native_options,
        Box::new(|cc| Box::new(FlosionApp::new(cc))),
    );
}
