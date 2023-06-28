use std::path::PathBuf;

use symphonia::core::{
    audio::{AudioBuffer, AudioBufferRef},
    codecs::{Decoder, DecoderOptions},
    errors,
    formats::{FormatOptions, FormatReader, Packet, Track},
    meta::MetadataOptions,
};

use crate::StereoBuffer;

use super::{
    error::{SampleDecodeError, SampleLoadError},
    prepare::{prepare_sample_reader, ReaderMeta},
};

macro_rules! symph_err {
    ( $x:expr ) => {{
        SampleDecodeError::LoadError(SampleLoadError::SymphoniaError($x))
    }};
}

pub struct Reader {
    /// Data related to the MediaSourceStream to be decoded
    pub meta: ReaderMeta,
    /// Data structures used to decode the targeted MediaSourceStream
    track: Track,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
}

impl Reader {
    pub fn new(
        path: PathBuf,
        meta_opts: MetadataOptions,
        fmt_opts: FormatOptions,
        dec_opts: DecoderOptions,
    ) -> Result<Self, SampleLoadError> {
        let (track, format, decoder, meta) = prepare_sample_reader(
            path,
            MetadataOptions::default(),
            FormatOptions::default(),
            DecoderOptions::default(),
        )?;

        Ok(Self {
            meta,
            track,
            format,
            decoder,
        })
    }

    fn decode_next(&mut self, packet: &Packet) -> Result<AudioBufferRef<'_>, SampleDecodeError> {
        // Consume any new metadata that has been read since the last packet.
        while !self.format.metadata().is_latest() {
            // Pop the old head of the metadata queue.
            self.format.metadata().pop();
        }
        // Consume the new metadata at the head of the metadata queue.
        // Currently there is no use for that.

        // If the packet does not belong to the selected track, skip over it.
        if packet.track_id() != self.track.id {
            return Err(SampleDecodeError::SkippablePacket);
        }

        // Decode the packet into audio samples.
        return match self.decoder.decode(&packet) {
            Ok(decoded) => Ok(decoded),
            Err(errors::Error::IoError(e)) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    Err(SampleDecodeError::EndReached)
                } else {
                    Err(SampleDecodeError::SkippablePacket)
                }
            }
            Err(errors::Error::DecodeError(_)) => Err(SampleDecodeError::SkippablePacket),
            Err(e) => Err(symph_err![e]),
        };
    }

    pub fn next_packet(
        &mut self,
        buffer: &mut StereoBuffer,
        remainder: &mut StereoBuffer,
    ) -> Result<usize, SampleLoadError> {
        let already_written = buffer.written();
        loop {
            let decoded_result = match self.format.next_packet() {
                Ok(packet) => self.decode_next(&packet),
                Err(errors::Error::IoError(e)) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        Err(SampleDecodeError::EndReached)
                    } else {
                        Err(SampleDecodeError::SkippablePacket)
                    }
                }
                Err(e) => Err(symph_err![e]),
            };

            match decoded_result {
                Ok(raw_buf) => buffer.copy_from_audio_buffer_ref(&raw_buf, remainder),
                Err(SampleDecodeError::EndReached) => {
                    break;
                }
                Err(SampleDecodeError::SkippablePacket) => {
                    continue;
                }
                Err(SampleDecodeError::ResetRequired) => {
                    return Err(SampleLoadError::ResetRequired);
                }
                Err(SampleDecodeError::LoadError(e)) => {
                    return Err(e);
                }
            };
        }

        Ok(buffer.written() - already_written + remainder.written())
    }

    fn reset_decoder(&mut self) {
        self.decoder.reset()
    }
}

/// Describes the reading capabilities of a sample reader
///
/// - Can read a buffer worth of content
/// - Can be issued for subsets of the buffer for incremental consumption
///
/// The reader is required to be able to produce `n` samples of content when requested,
/// but it is allowed to store a `multiple of n` samples internally.
///
/// The reader is not responsible of self issuing itself of further buffer reads when the
/// end of the internal buffer is reached. This is the responsibility of the caller.
pub trait SampleReader {
    /// Read the a buffer worth of content
    fn read_sync(&mut self) -> Result<(), SampleLoadError>;

    /// Issue the next slice of samples for both channels
    fn next_slice(&mut self) -> Result<(&[f32], &[f32]), SampleLoadError>;

    /// A number between 0 and 1 indicating the percentage of the internal buffer that has been consumed
    ///
    /// This value can be used to determine when to issue a new buffer read.
    fn percentage_consumed(&self) -> f32;
}

/// A reader which loads the full content of a sample into memory.
///
/// You should call `read` only once, since it will load the full content of the sample.
///
/// Sample issuing will eventually round robin.
///
/// The total capacity will be a multiple of the desired input buffer size
pub struct FullReader {
    reader: Reader,
    current_buffer: StereoBuffer,
}

impl FullReader {
    pub fn new(
        path: PathBuf,
        meta_opts: MetadataOptions,
        fmt_opts: FormatOptions,
        dec_opts: DecoderOptions,
    ) -> Result<Self, SampleLoadError> {
        let reader = Reader::new(path, meta_opts, fmt_opts, dec_opts)?;
        return Ok(Self {
            reader,
            current_buffer: StereoBuffer::new(0), // TODO find a way to compute the duration of the audio file
        });
    }
}
