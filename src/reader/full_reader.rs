use std::{mem::size_of, path::PathBuf};

use symphonia::core::{codecs::DecoderOptions, formats::FormatOptions, meta::MetadataOptions};

use super::{error::SampleLoadError, Reader, ReadingProjection, SampleReader};
use crate::buffer::{stereo::StereoBuffer, Buffer};

/// A reader which loads the full content of a sample into memory.
///
/// You should call `read` only once, since it will load the full content of the sample.
///
/// Sample issuing will eventually round robin.
///
/// The total capacity will be a multiple of the host buffer length.
pub struct SyncFullReader {
    pub buffer: StereoBuffer,
    reader: Reader,
    /// Reading cursor, not to be confused with the buffer cursor used for writing
    cursor: usize,
    host_buffer_len: usize,
}

impl SyncFullReader {
    pub fn new(
        path: PathBuf,
        host_buffer_len: usize,
        meta_opts: MetadataOptions,
        fmt_opts: FormatOptions,
        dec_opts: DecoderOptions,
    ) -> Result<Self, SampleLoadError> {
        let reader = Reader::new(path, meta_opts, fmt_opts, dec_opts)?;
        // exact sample count or 1MB worth of samples for 2 f32 channels
        let estimated_size = reader
            .meta
            .n_samples
            .unwrap_or((1 * 1024 * 1024) as u64 / (size_of::<f32>() * 2) as u64);

        return Ok(Self {
            reader,
            buffer: StereoBuffer::new(estimated_size as usize),
            cursor: 0,
            host_buffer_len,
        });
    }
}

impl SampleReader for SyncFullReader {
    fn read_sync(&mut self) -> Result<(), SampleLoadError> {
        let mut remainder = StereoBuffer::_0();
        let known_sample_count = self.reader.meta.n_samples.is_some();
        let mut samples_per_packet: usize =
            self.reader.meta.max_samples_per_packet.unwrap_or(0) as usize;
        let mut allocate = false;

        loop {
            match self.reader.next_packet(&mut self.buffer, &mut remainder)? {
                ReadingProjection::EndReached => break,
                ReadingProjection::SamplesRead(size) => {
                    if known_sample_count {
                        continue;
                    }
                    samples_per_packet = std::cmp::max(samples_per_packet, size);
                    if samples_per_packet >= self.buffer.capacity_left() {
                        allocate = true;
                    }
                }
            }

            if allocate {
                // Double the size of the buffer and continue
                self.buffer.reserve(self.buffer.capacity());
                allocate = false;
            }
        }

        self.buffer.trim();
        self.buffer.align_to(self.host_buffer_len);
        self.buffer.pad_silence();

        Ok(())
    }

    fn next_slice(&mut self) -> (&[f32], &[f32]) {
        let slices = self.buffer.slice(self.cursor, self.host_buffer_len);
        self.cursor += self.host_buffer_len;
        if self.cursor > self.buffer.channel_capacity() {
            self.cursor = 0;
        }
        slices
    }

    fn percentage_consumed(&self) -> f32 {
        self.cursor as f32 / self.buffer.capacity() as f32
    }
}
