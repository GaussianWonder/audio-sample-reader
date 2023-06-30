mod mono;
mod stereo;
mod utils;

pub use mono::*;
pub use stereo::*;
pub use utils::*;

pub enum BufferLayout {
    Mono,
    Stereo,
}

pub trait Buffer {
    fn layout(&self) -> BufferLayout;
    fn channel_capacity(&self) -> usize;
    fn capacity(&self) -> usize {
        match self.layout() {
            BufferLayout::Mono => self.channel_capacity(),
            BufferLayout::Stereo => self.channel_capacity() * 2,
        }
    }
    /// Refers to the fill cursor of the buffer, from which copy and swap occurs
    fn cursor(&self) -> usize;
    fn clear_cursor(&mut self);

    /// Ops on memory management of the buffer
    fn reserve(&mut self, additional: usize) -> usize;
    fn reserve_exact(&mut self, additional: usize);
    fn trim(&mut self);
    fn align_to(&mut self, alignment: usize) -> usize;

    fn has_content(&self) -> bool {
        self.cursor() > 0
    }

    /// The remaining capacity that can be used for copying into this buffer for one channel.
    fn capacity_left(&self) -> usize {
        self.channel_capacity() - self.cursor()
    }

    /// Calculate the overflow if the cursor was to advance by the given unit.
    ///
    /// This refers to one channel.
    fn overflow_on(&self, unit: usize) -> usize {
        let capacity = self.capacity_left();
        if unit <= capacity {
            return 0;
        }
        return unit - capacity;
    }

    /// Append a slice to all channels.
    ///
    /// # Panics
    ///
    /// Panics if the slice is larger than the channel capacity.
    fn append_slice(&mut self, slice: &[f32]);

    /// Append slice, but reserve the exact ammount of space if needed
    ///
    /// It is reccomended to call trim and align after a series of calls to this method.
    fn append_slice_exact(&mut self, slice: &[f32]) {
        let overflow = self.overflow_on(slice.len());
        if overflow > 0 {
            self.reserve_exact(overflow);
        }
        self.append_slice(slice);
    }

    /// Append slice, but reserves ~double the capacity if needed
    ///
    /// Be sure to trim and align the buffer after a series of calls to this method.
    fn append_slice_safe(&mut self, slice: &[f32]) {
        let overflow = self.overflow_on(slice.len());
        if overflow > 0 {
            self.reserve(self.channel_capacity());
        }
        self.append_slice(slice);
    }

    /// All samples after the cursor are set to 0.
    fn pad_silence(&mut self);

    /// The null buffer, a presumably unusable buffer.
    ///
    /// Attepmpts to **write into** or **read from** this buffer will result in a panic unless allocating more space.
    fn _0() -> Self;
}
