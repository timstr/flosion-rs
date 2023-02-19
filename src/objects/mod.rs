pub mod adsr;
pub mod audioclip;
pub mod dac;
pub mod functions;
pub mod keyboard;
pub mod melody;
pub mod mixer;
pub mod recorder;
pub mod resampler;
pub mod wavegenerator;
pub mod whitenoise;

#[cfg(test)]
mod test;

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
