use flosion::ui_core::flosion_ui::FlosionApp;
use std::{panic, process, thread};

fn main() {
    // Exit immediately if something panics
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(-1);
    }));

    // Context for inkwell/LLVM jit things. Compiled JIT artefacts
    // are used throughout the app, both the audio and GUI threads,
    // and so
    let inkwell_context = inkwell::context::Context::create();

    thread::scope(|scope| {
        eframe::run_native(
            "Flosion",
            eframe::NativeOptions::default(),
            Box::new(|cc| Ok(Box::new(FlosionApp::new(cc, &inkwell_context, scope)))),
        )
        .unwrap();
    });
}

// TODO
// - sequencer
// - lowpass
// - highpass
// - bandpass
// - FFT filter
// - convolver
// - granular synth
// - feedback
// - phase vocoder
// - compressor
// - overlap-add helpers
// - fft helpers
// - interactive display (spectrogram, waveform, oscilloscope)
// - undo/redo (consider using StateGraphEdit for this rather than serialization)
