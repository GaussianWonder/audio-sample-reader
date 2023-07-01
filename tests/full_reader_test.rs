use std::path::PathBuf;

use audio_reader::{Buffer, SampleReader, StereoBuffer, SyncFullReader};
use hound::{self, WavSpec};
use std::f32::consts::PI;
use std::i16;

const ACCEPTABLE_FLOAT_ERROR: f32 = 0.0001; // Used when converting between int and float
const ACCEPTABLE_ERROR: f32 = 0.000000000001; // Used when expecting identical values

const INT_MONO_SINE: &str = "assets/int_mono_sine.wav";
const FLOAT_STEREO_SINE: &str = "assets/float_stereo_sine.wav";

const SAMPLE_RATE: u32 = 44100; // will be used as a buffer size too (1second buffers)
const HOST_BUFFER_SIZE: usize = 1024;

const MONO_INT: WavSpec = WavSpec {
    channels: 1,
    sample_rate: SAMPLE_RATE,
    bits_per_sample: 16,
    sample_format: hound::SampleFormat::Int,
};

const STEREO_FLOAT: WavSpec = WavSpec {
    channels: 2,
    sample_rate: SAMPLE_RATE,
    bits_per_sample: 32,
    sample_format: hound::SampleFormat::Float,
};

fn sine_sample(t: f32) -> f32 {
    (t * 440.0 * 2.0 * PI).sin()
}

fn sine_float_samples() -> Vec<f32> {
    (0..SAMPLE_RATE)
        .map(|x| x as f32 / SAMPLE_RATE as f32)
        .map(sine_sample)
        .collect()
}

fn sine_int_samples() -> Vec<i16> {
    let amplitude = i16::MAX as f32;
    sine_float_samples()
        .iter()
        .map(|x| (x * amplitude) as i16)
        .collect()
}

fn mono_int_sine() {
    let mut writer = hound::WavWriter::create(INT_MONO_SINE, MONO_INT).unwrap();

    for t in sine_int_samples() {
        writer.write_sample(t).unwrap();
    }

    writer.finalize().unwrap();
}

fn stereo_float_sine() {
    let mut writer = hound::WavWriter::create(FLOAT_STEREO_SINE, STEREO_FLOAT).unwrap();

    for t in sine_float_samples() {
        writer.write_sample(t).unwrap();
        writer.write_sample(t).unwrap();
    }

    writer.finalize().unwrap();
}

fn channel_error(left: &[f32], right: &[f32]) -> Vec<f32> {
    left.iter()
        .zip(right)
        .map(|(a, b)| f32::abs(a - b))
        .collect()
}

fn is_error_smaller_than(left: &[f32], right: &[f32], threshold: f32) -> bool {
    channel_error(left, right).iter().all(|x| *x < threshold)
}

fn is_silence(buf: &[f32]) -> bool {
    buf.iter().all(|x| *x == 0.0)
}

fn assert_integrity(
    left_target: &[f32],
    right_target: &[f32],
    buffer: &StereoBuffer,
    error_threshold: f32,
) {
    assert!(is_error_smaller_than(
        &buffer.left.buf[..left_target.len()],
        left_target,
        error_threshold,
    ));
    assert!(is_error_smaller_than(
        &buffer.right.buf[..right_target.len()],
        right_target,
        error_threshold,
    ));

    let buf_len_diff = buffer.channel_capacity() - left_target.len();
    if buf_len_diff > 0 {
        assert!(is_silence(&buffer.left.buf[left_target.len()..]));
        assert!(is_silence(&buffer.right.buf[right_target.len()..],));
    }
}

#[test]
fn read_mono_int_wav() {
    mono_int_sine();

    let mut cmp_reader = hound::WavReader::open(INT_MONO_SINE).unwrap();

    assert_eq!(cmp_reader.spec(), MONO_INT);

    let samples: Vec<i16> = cmp_reader.samples().map(|x| x.unwrap()).collect();

    assert_eq!(samples, sine_int_samples());

    let mut reader = SyncFullReader::new(
        PathBuf::from(INT_MONO_SINE),
        HOST_BUFFER_SIZE,
        Default::default(),
        Default::default(),
        Default::default(),
    )
    .unwrap();

    reader.read_sync().unwrap();
    assert_eq!(reader.buffer.capacity() % HOST_BUFFER_SIZE, 0);

    let pregen_sine = sine_float_samples();

    assert_integrity(
        &pregen_sine,
        &pregen_sine,
        &reader.buffer,
        ACCEPTABLE_FLOAT_ERROR,
    );
}

#[test]
fn read_stereo_float_wav() {
    stereo_float_sine();

    let mut cmp_reader = hound::WavReader::open(FLOAT_STEREO_SINE).unwrap();

    assert_eq!(cmp_reader.spec(), STEREO_FLOAT);

    let samples: Vec<f32> = cmp_reader.samples().map(|x| x.unwrap()).collect();
    let left_samples: Vec<f32> = samples.iter().step_by(2).cloned().collect();
    let right_samples: Vec<f32> = samples.iter().skip(1).step_by(2).cloned().collect();

    let pregen_sine = sine_float_samples();

    assert_eq!(left_samples, pregen_sine);
    assert_eq!(right_samples, pregen_sine);

    let mut reader = SyncFullReader::new(
        PathBuf::from(FLOAT_STEREO_SINE),
        HOST_BUFFER_SIZE,
        Default::default(),
        Default::default(),
        Default::default(),
    )
    .unwrap();

    reader.read_sync().unwrap();
    assert_eq!(reader.buffer.capacity() % HOST_BUFFER_SIZE, 0);

    assert_integrity(&pregen_sine, &pregen_sine, &reader.buffer, ACCEPTABLE_ERROR);
}
