use symphonia::core::{
    audio::AudioBufferRef,
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions,
    probe::Hint,
};

use crate::core::{samplefrequency::SAMPLE_FREQUENCY, soundchunk::CHUNK_SIZE};

use super::soundbuffer::SoundBuffer;

pub(crate) fn load_audio_file(path: &std::path::Path) -> Result<SoundBuffer, String> {
    // Open the media source
    let src = std::fs::File::open(&path).map_err(|_| "Failed to open file".to_string())?;

    let mss = MediaSourceStream::new(Box::new(src), MediaSourceStreamOptions::default());

    // Create a probe hint using the file's extension
    let mut hint = Hint::new();
    hint.with_extension("mp4");

    // Use the default options for metadata and format readers
    let meta_opts = MetadataOptions::default();
    let fmt_opts = FormatOptions::default();

    // Probe the media source
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .map_err(|_| "Unsupported format".to_string())?;

    // Get the instantiated format reader
    let mut format = probed.format;

    // Find the first audio track with a known (decodable) codec
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| "No supported audio tracks".to_string())?;

    // Use the default options for the decoder
    let dec_opts = DecoderOptions::default();

    // create a decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .map_err(|_| "unsupported codec".to_string())?;

    // store the track identifier, it will be used to filter packets
    let track_id = track.id;

    if decoder.codec_params().sample_rate != Some(SAMPLE_FREQUENCY as u32) {
        println!("Warning: audio file sample rate differs from program sample rate");
    }

    let num_frames = decoder.codec_params().n_frames.unwrap_or(0) as usize;

    let mut soundbuffer = SoundBuffer::new_with_capacity(num_frames / CHUNK_SIZE);

    // the decode loop
    loop {
        // Get the next packet from the media format
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::ResetRequired) => unimplemented!(),
            Err(symphonia::core::errors::Error::IoError(_)) => {
                // End of stream???
                break;
            }
            Err(err) => panic!("Oh crap: {:?}", err),
        };

        // consume any new metadata that has been read since the last packet
        while !format.metadata().is_latest() {
            // pop the old head of the metadata queue
            format.metadata().pop();
        }

        // if the packet does not belong to the selected track, skip over it
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                // consume the decoded audio samples
                match decoded {
                    AudioBufferRef::F32(buf) => {
                        let planes = buf.planes();
                        let planes_planes = planes.planes();
                        assert_eq!(planes_planes.len(), 2);
                        let plane_l = planes_planes[0];
                        let plane_r = planes_planes[1];
                        assert_eq!(plane_l.len(), plane_r.len());

                        for (l, r) in plane_l.iter().zip(plane_r) {
                            // all_the_samples.push(*l);
                            // all_the_samples.push(*r);
                            soundbuffer.push_sample(*l, *r);
                        }
                    }
                    _ => unimplemented!(),
                }
            }
            Err(symphonia::core::errors::Error::IoError(_)) => {
                // the packet failed to decode due to an IO error, skip the packet
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => {
                // the packet failed to decode due to invalid data, skip the packet
                continue;
            }
            Err(err) => {
                panic!("Aw sheeeit: {}", err);
            }
        }
    }

    Ok(soundbuffer)
}
