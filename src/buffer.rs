use symphonia::core::{
    audio::{AudioBuffer, AudioBufferRef, Layout, Signal, SignalSpec},
    conv::IntoSample,
    sample::Sample,
};

/// Opinionated buffer for inner operations
///
/// Values exceeding the capacity of samples_written are considered to be uninitialized and non deterministic.
pub struct StereoBuffer {
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    channel_size: usize,
    /// The location from which copy and swap occurs in this buffer
    samples_written: usize,
}

impl StereoBuffer {
    /// Unasable buffer, panics when it is used
    pub fn _0() -> Self {
        Self {
            left: vec![],
            right: vec![],
            channel_size: 0,
            samples_written: 0,
        }
    }

    pub fn new(channel_size: usize) -> Self {
        Self {
            left: StereoBuffer::buffer_with_size(channel_size),
            right: StereoBuffer::buffer_with_size(channel_size),
            channel_size,
            samples_written: 0,
        }
    }

    pub fn has_content(&self) -> bool {
        return self.samples_written > 0;
    }

    pub fn written(&self) -> usize {
        return self.samples_written;
    }

    pub fn total_capacity(&self) -> usize {
        self.channel_size * 2
    }

    /// The remaining capacity that can be used for copying into this buffer for one channel
    pub fn capacity_left(&self) -> usize {
        self.channel_size - self.samples_written
    }

    /// Calculate the overflow if the cursor was to advance by the given unit
    pub fn overflow_on(&self, unit: usize) -> usize {
        let capacity = self.capacity_left();
        if unit <= capacity {
            return 0;
        }
        return unit - capacity;
    }

    /// This doesn't clear the buffer content, the capacity remains the same,
    /// the values are never changed, but further operations on this buffer will start from the beginning
    pub fn clear_cursor(&mut self) {
        self.samples_written = 0;
    }

    /// Create a new buffer with the given size.
    ///
    /// The contents of the buffer are indexable from the very beginning, however the content is uninitialized.
    pub fn buffer_with_size(size: usize) -> Vec<f32> {
        let mut buffer = Vec::<f32>::with_capacity(size);
        unsafe {
            buffer.set_len(size);
        }
        buffer
    }

    /// Get an immutable slice of the left and right channels
    ///
    /// # Panics
    ///
    /// Panics if the offset + size is greater than the buffer size
    pub fn slice(&self, offset: usize, size: usize) -> (&[f32], &[f32]) {
        (
            &self.left[offset..offset + size],
            &self.right[offset..offset + size],
        )
    }

    /// Get a mutable slice of the left and right channels
    ///
    /// # Panics
    ///
    /// Panics if the offset + size is greater than the buffer size
    pub fn slice_mut(&mut self, offset: usize, size: usize) -> (&mut [f32], &mut [f32]) {
        (
            &mut self.left[offset..offset + size],
            &mut self.right[offset..offset + size],
        )
    }

    /// Copy slices of samples into the buffer's left and right channels
    ///
    /// # Panics
    ///
    /// Panics if the input buffers are differnt length or if there is no room to copy into.
    pub fn copy_from_slices(&mut self, left: &[f32], right: &[f32]) {
        assert!(
            left.len() == right.len(),
            "Left and right channels must be the same length"
        );

        let sample_count = left.len();

        assert!(
            self.capacity_left() >= sample_count,
            "Buffer too large to copy from"
        );

        let end_slice = self.samples_written + sample_count;

        // Memcpy the buffer into a slice of the left and right channels
        self.left[self.samples_written..end_slice].copy_from_slice(left);
        self.right[self.samples_written..end_slice].copy_from_slice(right);

        // Commit the samples written
        self.samples_written += sample_count;
    }

    /// Similar to copy_from_slices, but only for one channel.
    ///
    /// It does not panic when copying to internal buffers,
    /// instead it tries to swap into the given remainder buffer, which can panic if not big enough.
    ///
    /// The remainder buffer is not cleared, but it is filled when remainder samples exists
    pub fn copy_slice_mono(&mut self, buf: &[f32], remainder: &mut StereoBuffer) {
        let sample_count = buf.len();
        if sample_count == 0 {
            return;
        }

        let sample_overflow = self.overflow_on(sample_count);
        let samples_to_write = sample_count - sample_overflow;
        let end_slice = self.samples_written + samples_to_write;

        // Memcpy the buffer into a slice of the left and right channels
        self.left[self.samples_written..end_slice]
            .copy_from_slice(buf[..samples_to_write].as_ref());
        self.right[self.samples_written..end_slice]
            .copy_from_slice(buf[..samples_to_write].as_ref());

        // Commit the samples written
        self.samples_written += samples_to_write;

        // Fill in the buffer overflow if applicable
        if sample_overflow > 0 {
            let overflow_end = samples_to_write + sample_overflow;
            remainder.copy_from_slices(
                &buf[samples_to_write..overflow_end],
                &buf[samples_to_write..overflow_end],
            );
        }
    }

    /// Similar to copy_from_slices, but it does not panic, instead it fills an overflow buffer
    pub fn copy_slice_stereo(&mut self, left: &[f32], right: &[f32], remainder: &mut StereoBuffer) {
        assert!(
            left.len() == right.len(),
            "Left and right channels must be the same length"
        );

        let sample_count = left.len();

        if sample_count == 0 {
            return;
        }

        let sample_overflow = self.overflow_on(sample_count);
        let samples_to_write = sample_count - sample_overflow;
        let end_slice = self.samples_written + samples_to_write;

        // Memcpy the buffers into the according slices of the left and right channels
        self.left[self.samples_written..end_slice]
            .copy_from_slice(left[..samples_to_write].as_ref());
        self.right[self.samples_written..end_slice]
            .copy_from_slice(right[..samples_to_write].as_ref());

        // Commit the samples written
        self.samples_written += samples_to_write;

        // Construct the buffer overflow if applicable
        if sample_overflow > 0 {
            let overflow_end = samples_to_write + sample_overflow;
            remainder.copy_from_slices(
                &left[samples_to_write..overflow_end],
                &right[samples_to_write..overflow_end],
            );
        }
    }

    /// Swap the contents of this buffer with another, starting from the beginning of this buffer's copy_cursor
    ///
    /// This is useful for filling in a buffer with remainder samples from a previous decoding iteration
    ///
    /// # Panics
    ///
    /// Panics if the input buffer size is greater than the remainder of this buffer can hold
    pub fn swap_with_buffer(&mut self, buffer: &mut StereoBuffer) {
        assert!(
            self.capacity_left() >= buffer.channel_size,
            "Input buffer size must be less than the remainder of this buffer"
        );

        let end_slice = self.samples_written + buffer.channel_size;

        // Swap the buffer into a slice of the left and right channels
        self.left[self.samples_written..end_slice].swap_with_slice(&mut buffer.left[..]);
        self.right[self.samples_written..end_slice].swap_with_slice(&mut buffer.right[..]);

        // Commit the samples written
        self.samples_written += buffer.channel_size;
    }

    /// Swap the contents of this buffer with Vec<Vec<f32>; 2> (one for each channel), starting from the beginning of this buffer's copy_cursor
    ///
    /// This is useful for filling in a buffer with resampled contents form another buffer
    ///
    /// # Panics
    ///
    /// Panics if the input buffer sizes differ or if the capacity_left is not enough to swap the input buffer
    pub fn swap_with_vec(&mut self, buffer: &mut Vec<Vec<f32>>) {
        assert!(buffer.len() == 2, "Input buffer must have 2 channels");

        assert!(
            buffer[0].len() == buffer[1].len(),
            "Left and right channels must be the same length",
        );

        let input_size = buffer[0].len();

        assert!(
            self.capacity_left() >= input_size,
            "Input buffer size must be less than the remainder of this buffer"
        );

        let end_slice = self.samples_written + input_size;

        // Swap the buffer into a slice of the left and right channels
        self.left[self.samples_written..end_slice].swap_with_slice(buffer[0].as_mut_slice());
        self.right[self.samples_written..end_slice].swap_with_slice(buffer[1].as_mut_slice());

        // Commit the samples written
        self.samples_written += input_size;
    }

    /// Copy the contents of another buffer into this buffer, including the copy_cursor
    ///
    /// # Panics
    ///
    /// Panics if the buffer sizes differ
    pub fn copy_from_buffer(&mut self, buffer: &StereoBuffer) {
        assert!(
            self.channel_size == buffer.channel_size,
            "Buffer sizes must be equal"
        );

        self.samples_written = buffer.samples_written;
        self.left.copy_from_slice(&buffer.left[..]);
        self.right.copy_from_slice(&buffer.right[..]);
    }

    /// Copy from an AudioBuffer into the left and right channels of this buffer.
    ///
    /// Remainder samples are returned if the buffer is full.
    ///
    /// Behavior differs based on SignalSpec:
    /// - Mono copies the buffer into both channels
    /// - Stereo copies each channel into the left and right channels accordingly
    /// - Multichannel averages all channels into one and copies that into the left and right channels
    ///
    /// # Panics
    ///
    /// Panics if the channel sizes differ.
    pub fn copy_from_audio_buffer(
        &mut self,
        buffer: &AudioBuffer<f32>,
        remainder: &mut StereoBuffer,
    ) {
        let spec = buffer.spec();

        if spec.channels == Layout::Mono.into_channels() {
            return self.copy_slice_mono(buffer.chan(0), remainder);
        }

        if spec.channels == Layout::Stereo.into_channels() {
            return self.copy_slice_stereo(buffer.chan(0), buffer.chan(1), remainder);
        }

        // Average all channels into one and copy that into the left and right channels
        let mut mono = Self::buffer_with_size(buffer.frames());
        if mono.len() == 0 {
            return;
        }

        let audio_planes = buffer.planes();
        let channels = audio_planes.planes();
        for channel in channels {
            for (i, sample) in channel.iter().enumerate() {
                mono[i] += *sample;
            }
        }

        let f32_chan_cnt = channels.len() as f32;
        for sample in mono.iter_mut() {
            *sample /= f32_chan_cnt;
        }

        return self.copy_slice_mono(mono.as_slice(), remainder);
    }

    /// Given any AudioBuffer<S> copy in a unified way the contents to the internal buffer and return the overflow
    ///
    /// * Currently this always results in a copy before calling copy_from_audio_buffer
    ///
    /// TODO instead of converting the whole buffer immutably and applying copy_from_audio_buffer
    /// TODO try iterating over the samples, converting each one to the correct type individually
    /// TODO and increment the samples_written by 1 for each sample
    /// TODO this function needs to take into account mono and stereo AudioBuffer signals similar to how copy_from_audio_buffer does it
    fn copy_from_any_audio_buffer<S>(
        &mut self,
        input: &AudioBuffer<S>,
        remainder: &mut StereoBuffer,
    ) where
        S: Sample + IntoSample<f32>,
    {
        let spec = *input.spec();
        let mut converted = AudioBuffer::<f32>::new(input.capacity() as u64, spec);
        input.convert(&mut converted);
        self.copy_from_audio_buffer(&converted, remainder)
    }

    /// Given an AudioBufferRef of any S sample type, copy the contents to the internal buffer and return the overflow
    pub fn copy_from_audio_buffer_ref(
        &mut self,
        buffer: &AudioBufferRef<'_>,
        remainder: &mut StereoBuffer,
    ) {
        match buffer {
            AudioBufferRef::F32(input) => self.copy_from_audio_buffer(input, remainder),
            AudioBufferRef::U8(input) => self.copy_from_any_audio_buffer(input, remainder),
            AudioBufferRef::U16(input) => self.copy_from_any_audio_buffer(input, remainder),
            AudioBufferRef::U24(input) => self.copy_from_any_audio_buffer(input, remainder),
            AudioBufferRef::U32(input) => self.copy_from_any_audio_buffer(input, remainder),
            AudioBufferRef::S8(input) => self.copy_from_any_audio_buffer(input, remainder),
            AudioBufferRef::S16(input) => self.copy_from_any_audio_buffer(input, remainder),
            AudioBufferRef::S24(input) => self.copy_from_any_audio_buffer(input, remainder),
            AudioBufferRef::S32(input) => self.copy_from_any_audio_buffer(input, remainder),
            AudioBufferRef::F64(input) => self.copy_from_any_audio_buffer(input, remainder),
        }
    }

    /// Pad the remainder of the buffer with silence
    pub fn pad_silence(&mut self) {
        if self.samples_written == self.channel_size {
            return;
        }
        self.left[self.samples_written..].fill(0f32);
        self.right[self.samples_written..].fill(0f32);
        self.samples_written = self.channel_size;
    }
}
