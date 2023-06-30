use symphonia::core::{
    audio::{AudioBuffer, AudioBufferRef},
    conv::IntoSample,
    sample::Sample,
};

/// Create a buffer with a given capacity and set its length to the same value.
pub fn buffer_with_size(size: usize) -> Vec<f32> {
    let mut buffer = Vec::<f32>::with_capacity(size);
    unsafe {
        buffer.set_len(size);
    }
    buffer
}

/// Convert any AudioBuffer<S> into an AudioBuffer<f32> by copying and converting each sample.
///
/// This 100% clones the buffer.
pub fn uniform_audio_buffer<S>(input: &AudioBuffer<S>) -> AudioBuffer<f32>
where
    S: Sample + IntoSample<f32>,
{
    let spec = *input.spec();
    let mut converted = AudioBuffer::<f32>::new(input.capacity() as u64, spec);
    input.convert(&mut converted);
    return converted;
}

/// This will 100% clone
pub fn convert_any_audio_buffer(buffer: &AudioBufferRef) -> AudioBuffer<f32> {
    match buffer {
        AudioBufferRef::F32(input) => uniform_audio_buffer(input),
        AudioBufferRef::U8(input) => uniform_audio_buffer(input),
        AudioBufferRef::U16(input) => uniform_audio_buffer(input),
        AudioBufferRef::U24(input) => uniform_audio_buffer(input),
        AudioBufferRef::U32(input) => uniform_audio_buffer(input),
        AudioBufferRef::S8(input) => uniform_audio_buffer(input),
        AudioBufferRef::S16(input) => uniform_audio_buffer(input),
        AudioBufferRef::S24(input) => uniform_audio_buffer(input),
        AudioBufferRef::S32(input) => uniform_audio_buffer(input),
        AudioBufferRef::F64(input) => uniform_audio_buffer(input),
    }
}
