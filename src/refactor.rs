pub trait MonoBuffer {
    /// Capacity of the buffer.
    fn capacity(&self) -> usize;

    /// Refers to the fill cursor of the buffer, from which copy and swap occurs.
    fn cursor(&self) -> usize;
    /// Reset the cursor back to 0, following swap and copy operations occur from the start of the buffer.
    fn clear_cursor(&mut self);

    /// Reserve additional space for the buffer, this is not guaranteed to be exact.
    fn reserve(&mut self, additional: usize) -> usize;
    /// Reserve additional space for the buffer, this is guaranteed to be exact.
    fn reserve_exact(&mut self, additional: usize);
    /// Trim the buffer to the current used capacity.
    fn trim(&mut self);
    /// Align the buffer to the given alignment.
    /// 
    /// Useful when this buffer is feeding into some other buffer (i.e. host consuming samples). Pad silence after this operation when needed.
    fn align_to(&mut self, alignment: usize) -> usize;

    /// Wether or not this buffer has any usable content inside.
    fn has_content(&self) -> bool {
        self.cursor() > 0
    }

    /// The remaining capacity that can be used for copying into this buffer.
    fn capacity_left(&self) -> usize {
        self.capacity() - self.cursor()
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