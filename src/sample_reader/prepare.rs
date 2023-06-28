use std::{fs::File, path::PathBuf};

use symphonia::core::{
    audio::Layout,
    codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL},
    formats::{FormatOptions, FormatReader, Track},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
    units::TimeBase,
};

use super::error::SampleLoadError;

macro_rules! meta_err {
    ( $x:expr ) => {{
        SampleLoadError::MissingRequiredMetadata($x)
    }};
}

#[derive(Debug, Clone)]
pub struct ReaderMeta {
    pub path: PathBuf,
    pub layout: Layout,
    pub delay: u32,
    pub padding: u32,
    pub sample_rate: u32,
    pub start_ts: u64,
    pub time_base: TimeBase,
    pub max_frames_per_packet: u64,
}

fn prepare_media_source(path: &PathBuf) -> Result<MediaSourceStream, SampleLoadError> {
    match File::open(path) {
        Ok(file) => Ok(MediaSourceStream::new(Box::new(file), Default::default())),
        Err(e) => Err(SampleLoadError::IoError(e)),
    }
}

fn prepare_formatter_hint(path: &PathBuf) -> Hint {
    let extension = path.extension().and_then(|os_str| os_str.to_str());
    let mut hint = Hint::new();
    if let Some(ext) = extension {
        hint.with_extension(ext);
    }
    hint
}

type DecodableFormat = (Track, Box<dyn FormatReader>, Box<dyn Decoder>);

fn prepare_sample_decoder(
    path: &PathBuf,
    meta_opts: &MetadataOptions,
    fmt_opts: &FormatOptions,
    dec_opts: &DecoderOptions,
) -> Result<DecodableFormat, SampleLoadError> {
    // Load the file into a MediaSourceStream
    let media_source = prepare_media_source(path)?;

    // Get metadata information from the path
    let hint = prepare_formatter_hint(&path);

    // Probe the media source.
    match symphonia::default::get_probe().format(&hint, media_source, fmt_opts, meta_opts) {
        Ok(probed) => {
            // Get the instantiated format reader.
            let format = probed.format;

            // Find the first audio track with a known (decodeable) codec.
            match format
                .tracks()
                .iter()
                .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            {
                Some(track) => {
                    // Create a decoder for the track.
                    match symphonia::default::get_codecs().make(&track.codec_params, &dec_opts) {
                        Ok(decoder) => Ok((track.clone(), format, decoder)),
                        Err(e) => return Err(SampleLoadError::SymphoniaError(e)),
                    }
                }
                None => return Err(SampleLoadError::NoSupportedAudioTracks),
            }
        }
        Err(e) => Err(SampleLoadError::SymphoniaError(e)),
    }
}

type ReadableFormat = (Track, Box<dyn FormatReader>, Box<dyn Decoder>, ReaderMeta);

pub fn prepare_sample_reader(
    path: PathBuf,
    meta_opts: MetadataOptions,
    fmt_opts: FormatOptions,
    dec_opts: DecoderOptions,
) -> Result<ReadableFormat, SampleLoadError> {
    let (track, reader, decoder) = prepare_sample_decoder(&path, &meta_opts, &fmt_opts, &dec_opts)?;

    let codec_params = decoder.codec_params();

    let layout = codec_params
        .channel_layout
        .ok_or(meta_err!["channel layout"])?;

    match layout {
        Layout::Mono => Ok(()),
        Layout::Stereo => Ok(()),
        _ => Err(SampleLoadError::UnsupportedChannelLayout(layout)),
    }?;

    let delay = codec_params.delay.unwrap_or(0);
    let padding = codec_params.padding.unwrap_or(0);
    let sample_rate = codec_params.sample_rate.ok_or(meta_err!["sample rate"])?;
    let start_ts = codec_params.start_ts;
    let time_base = codec_params.time_base.ok_or(meta_err!["time base"])?;
    let max_frames_per_packet = codec_params
        .max_frames_per_packet
        .ok_or(meta_err!["max frames per packets"])?;

    Ok((
        track,
        reader,
        decoder,
        ReaderMeta {
            path,
            layout,
            delay,
            padding,
            sample_rate,
            start_ts,
            time_base,
            max_frames_per_packet,
        },
    ))
}
