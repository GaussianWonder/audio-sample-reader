use super::{utils::buffer_with_size, Buffer, BufferLayout};

/// Opinionated buffer for inner operations
///
/// Values exceeding the capacity of samples_written are considered to be uninitialized and non deterministic.
pub struct MonoBuffer {
    pub buf: Vec<f32>,
    channel_size: usize,
    /// The location from which copy and swap occurs in this buffer
    samples_written: usize,
}

impl<Idx> std::ops::Index<Idx> for MonoBuffer
where
    Idx: std::slice::SliceIndex<[f32]>,
{
    type Output = Idx::Output;

    fn index(&self, index: Idx) -> &Self::Output {
        &self.buf[index]
    }
}

impl<Idx> std::ops::IndexMut<Idx> for MonoBuffer
where
    Idx: std::slice::SliceIndex<[f32]>,
{
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        &mut self.buf[index]
    }
}

impl MonoBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: buffer_with_size(capacity),
            channel_size: capacity,
            samples_written: 0,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, f32> {
        self.buf.iter()
    }

    fn recalculate_len(&mut self) {
        unsafe {
            self.buf.set_len(self.buf.capacity());
        }
        self.channel_size = self.buf.len();
    }

    pub fn slice(&self, start: usize, len: usize) -> &[f32] {
        &self.buf[start..start + len]
    }

    pub fn slice_mut(&mut self, start: usize, len: usize) -> &mut [f32] {
        &mut self.buf[start..start + len]
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.buf
    }

    pub fn as_slice_mut(&mut self) -> &mut [f32] {
        &mut self.buf
    }

    /// Append a slice to all channels, and fill overflow with unappendable content.
    pub fn append_slice_overflow(&mut self, slice: &[f32], overflow: &mut MonoBuffer) {
        let sample_count = slice.len();
        if sample_count == 0 {
            return;
        }

        let sample_overflow = self.overflow_on(sample_count);
        let samples_to_write = sample_count - sample_overflow;

        self.append_slice(&slice[..samples_to_write]);

        if sample_overflow > 0 {
            let overflow_end = samples_to_write + sample_overflow;
            overflow.append_slice(&slice[samples_to_write..overflow_end]);
        }
    }

    // TODO append audio buffer & audio buffer ref
}

impl Buffer for MonoBuffer {
    fn append_slice(&mut self, slice: &[f32]) {
        let cursor = self.cursor();
        let end = cursor + slice.len();
        self.buf[cursor..end].copy_from_slice(slice);
        self.samples_written += slice.len();
    }

    fn layout(&self) -> BufferLayout {
        BufferLayout::Mono
    }

    fn channel_capacity(&self) -> usize {
        self.channel_size
    }

    fn cursor(&self) -> usize {
        self.samples_written
    }

    fn clear_cursor(&mut self) {
        self.samples_written = 0
    }

    fn reserve(&mut self, additional: usize) -> usize {
        let old_capacity = self.buf.capacity();
        self.buf.reserve(additional);
        self.recalculate_len();
        return self.buf.capacity() - old_capacity;
    }

    fn reserve_exact(&mut self, additional: usize) {
        self.buf.reserve_exact(additional);
        self.recalculate_len();
    }

    fn trim(&mut self) {
        if self.capacity_left() == 0 {
            return;
        }

        self.buf.shrink_to_fit();
        self.buf.resize(self.samples_written, 0f32);
        self.channel_size = self.buf.len();
    }

    fn align_to(&mut self, alignment: usize) -> usize {
        if self.channel_size % alignment == 0 {
            return 0;
        }

        let additional = alignment - self.channel_size % alignment;
        self.reserve_exact(additional);
        return additional;
    }

    fn pad_silence(&mut self) {
        if self.capacity_left() == 0 {
            return;
        }

        self.buf[self.samples_written..].fill(0f32);
        self.samples_written = self.channel_size;
    }

    fn _0() -> Self {
        Self::new(0)
    }
}
