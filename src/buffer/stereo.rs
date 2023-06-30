use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Layout, Signal};

use super::{mono::MonoBuffer, utils::uniform_audio_buffer, Buffer, BufferLayout};

/// Stereo channels
pub enum Channel {
    Left = 0,
    Right = 1,
}

pub struct StereoBuffer {
    pub left: MonoBuffer,
    pub right: MonoBuffer,
}

impl StereoBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            left: MonoBuffer::new(capacity),
            right: MonoBuffer::new(capacity),
        }
    }

    pub fn slice(&self, start: usize, len: usize) -> (&[f32], &[f32]) {
        (self.left.slice(start, len), self.right.slice(start, len))
    }

    pub fn slice_mut(&mut self, start: usize, len: usize) -> (&mut [f32], &mut [f32]) {
        (
            self.left.slice_mut(start, len),
            self.right.slice_mut(start, len),
        )
    }

    pub fn as_slice(&self) -> (&[f32], &[f32]) {
        (self.left.as_slice(), self.right.as_slice())
    }

    pub fn as_slice_mut(&mut self) -> (&mut [f32], &mut [f32]) {
        (self.left.as_slice_mut(), self.right.as_slice_mut())
    }

    /// Same as append_slice, but independent for each channel.
    pub fn append_slices(&mut self, left: &[f32], right: &[f32]) {
        debug_assert_eq!(self.left.capacity(), self.right.capacity());
        debug_assert_eq!(self.left.cursor(), self.right.cursor());

        self.left.append_slice(left);
        self.right.append_slice(right);
    }

    /// Append a slice to all channels, and fill overflow with unappendable content.
    pub fn append_slices_overflow(
        &mut self,
        left: &[f32],
        right: &[f32],
        overflow: &mut StereoBuffer,
    ) {
        debug_assert_eq!(self.left.capacity(), self.right.capacity());
        debug_assert_eq!(self.left.cursor(), self.right.cursor());
        debug_assert_eq!(overflow.left.capacity(), overflow.right.capacity());
        debug_assert_eq!(overflow.left.cursor(), overflow.right.cursor());

        self.left
            .append_slice_overflow(left, &mut overflow.left);
        self.right
            .append_slice_overflow(right, &mut overflow.right);
    }

    pub fn append_audio_buffer(&mut self, buffer: &AudioBuffer<f32>, overflow: &mut StereoBuffer) {
        debug_assert_eq!(self.left.capacity(), self.right.capacity());
        debug_assert_eq!(self.left.cursor(), self.right.cursor());
        debug_assert_eq!(overflow.left.capacity(), overflow.right.capacity());
        debug_assert_eq!(overflow.left.cursor(), overflow.right.cursor());

        let spec = buffer.spec();

        if spec.channels == Layout::Mono.into_channels() {
            let mono_buf = buffer.chan(0);
            self.append_slices_overflow(mono_buf, mono_buf, overflow);
            return;
        }

        if spec.channels == Layout::Stereo.into_channels() {
            self.append_slices_overflow(buffer.chan(0), buffer.chan(1), overflow);
            return;
        }

        unimplemented!("Only mono and stereo audio buffers are supported")
    }

    pub fn append_audio_buffer_ref(
        &mut self,
        buffer: &AudioBufferRef,
        overflow: &mut StereoBuffer,
    ) {
        match buffer {
            AudioBufferRef::F32(input) => self.append_audio_buffer(input, overflow),
            AudioBufferRef::U8(input) => {
                self.append_audio_buffer(&uniform_audio_buffer(input), overflow)
            }
            AudioBufferRef::U16(input) => {
                self.append_audio_buffer(&uniform_audio_buffer(input), overflow)
            }
            AudioBufferRef::U24(input) => {
                self.append_audio_buffer(&uniform_audio_buffer(input), overflow)
            }
            AudioBufferRef::U32(input) => {
                self.append_audio_buffer(&uniform_audio_buffer(input), overflow)
            }
            AudioBufferRef::S8(input) => {
                self.append_audio_buffer(&uniform_audio_buffer(input), overflow)
            }
            AudioBufferRef::S16(input) => {
                self.append_audio_buffer(&uniform_audio_buffer(input), overflow)
            }
            AudioBufferRef::S24(input) => {
                self.append_audio_buffer(&uniform_audio_buffer(input), overflow)
            }
            AudioBufferRef::S32(input) => {
                self.append_audio_buffer(&uniform_audio_buffer(input), overflow)
            }
            AudioBufferRef::F64(input) => {
                self.append_audio_buffer(&uniform_audio_buffer(input), overflow)
            }
        }
    }
}

impl Buffer for StereoBuffer {
    fn append_slice(&mut self, slice: &[f32]) {
        debug_assert_eq!(self.left.capacity(), self.right.capacity());
        debug_assert_eq!(self.left.cursor(), self.right.cursor());
        self.left.append_slice(slice);
        self.right.append_slice(slice);
    }

    fn layout(&self) -> BufferLayout {
        BufferLayout::Stereo
    }

    fn channel_capacity(&self) -> usize {
        self.left.channel_capacity()
    }

    fn cursor(&self) -> usize {
        debug_assert_eq!(self.left.cursor(), self.right.cursor());
        self.left.cursor()
    }

    fn clear_cursor(&mut self) {
        self.left.clear_cursor();
        self.right.clear_cursor();
    }

    fn reserve(&mut self, additional: usize) -> usize {
        let reserved = self.left.reserve(additional);
        self.right.reserve_exact(reserved);
        return reserved;
    }

    fn reserve_exact(&mut self, additional: usize) {
        self.left.reserve_exact(additional);
        self.right.reserve_exact(additional);
    }

    fn trim(&mut self) {
        self.left.trim();
        self.right.trim();
    }

    fn align_to(&mut self, alignment: usize) -> usize {
        let left_add = self.left.align_to(alignment);
        let right_add = self.right.align_to(alignment);

        debug_assert_eq!(left_add, right_add);

        return left_add;
    }

    fn pad_silence(&mut self) {
        self.left.pad_silence();
        self.right.pad_silence();
    }

    fn _0() -> Self {
        Self::new(0)
    }
}
