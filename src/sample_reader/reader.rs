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
            PathBuf::from(""),
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
}
