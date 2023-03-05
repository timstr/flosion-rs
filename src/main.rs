use flosion::ui_core::flosion_ui::FlosionApp;
use std::{panic, process};

fn main() {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(-1);
    }));

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Flosion",
        native_options,
        Box::new(|cc| Box::new(FlosionApp::new(cc))),
    );
}

// TODO
// - sequencer
// - microphone
// - lowpass
// - highpass
// - bandpass
// - stateful number sources
// - FFT filter
// - convolver
// - granular synth
// - feedback
// - phase vocoder
// - scatter
// - ensemble
// - compressor
// - overlap-add helpers
// - fft helpers
// - interactive display (spectrogram, waveform, oscilloscope)
// - undo/redo (consider using StateGraphEdit for this rather than serialization)
