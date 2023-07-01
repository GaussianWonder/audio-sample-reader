pub mod error;
pub mod full_reader;
pub mod prepare;

use self::prepare::{prepare_sample_reader, ReaderMeta};
use crate::buffer::{stereo::StereoBuffer, Buffer};
use error::*;

use std::path::PathBuf;
use symphonia::core::{
    audio::AudioBufferRef,
    codecs::{Decoder, DecoderOptions},
    errors,
    formats::{FormatOptions, FormatReader, Packet, Track},
    meta::MetadataOptions,
};

macro_rules! symph_err {
    ( $x:expr ) => {{
        SampleDecodeError::LoadError(SampleLoadError::SymphoniaError($x))
    }};
}

/// A thing you receive after you read and decode a packet.
pub enum ReadingProjection {
    /// Samples read per channel.
    SamplesRead(usize),
    EndReached,
}

/// The thing that reads and decodes a sample.
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
        let (track, format, decoder, meta) =
            prepare_sample_reader(path, meta_opts, fmt_opts, dec_opts)?;

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
    ) -> Result<ReadingProjection, SampleLoadError> {
        let is_end: bool;
        let already_written = buffer.cursor();

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
                Ok(raw_buf) => buffer.append_audio_buffer_ref(&raw_buf, remainder),
                Err(SampleDecodeError::EndReached) => {
                    is_end = true;
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

        Ok(if is_end {
            ReadingProjection::EndReached
        } else {
            ReadingProjection::SamplesRead(buffer.cursor() - already_written + remainder.cursor())
        })
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
    /// Read a buffer worth of content
    fn read_sync(&mut self) -> Result<(), SampleLoadError>;

    /// Issue the next slice of samples for both channels
    fn next_slice(&mut self) -> (&[f32], &[f32]);

    /// Get the next sample from the requested channel
    // fn next_sample(&mut self) -> Result<f32, SampleLoadError>;

    /// A number between 0 and 1 indicating the percentage of the internal buffer that has been consumed
    ///
    /// This value can be used to determine when to issue a new buffer read.
    fn percentage_consumed(&self) -> f32;
}

pub mod prelude {
    pub use super::{
        error::{SampleDecodeError, SampleLoadError},
        full_reader::SyncFullReader,
        Reader, ReadingProjection, SampleReader,
    };
}
