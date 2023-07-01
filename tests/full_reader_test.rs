use hound::{self, WavSpec};
use std::f32::consts::PI;
use std::i16;
use std::path::PathBuf;
use std::process::Command;

use audio_reader::prelude::*;

const ACCEPTABLE_FLOAT_ERROR: f64 = 0.0001; // Used when converting between int and float
const ACCEPTABLE_ERROR: f64 = 0.000000000001; // Used when expecting identical values

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

/// Convert a file to a different format using ffmpeg
fn convert_audio_to(input: &'static str, ext: &'static str) -> PathBuf {
    let output = PathBuf::from(input).with_extension(ext);

    Command::new("ffmpeg")
        .arg("-i")
        .arg(input)
        .arg(output.to_str().unwrap())
        .output()
        .expect("Failed to execute ffmpeg command");

    return output;
}

/// Get a SyncFullReader for a given file
fn default_reader(path: PathBuf) -> SyncFullReader {
    SyncFullReader::new(
        path,
        HOST_BUFFER_SIZE,
        Default::default(),
        Default::default(),
        Default::default(),
    )
    .unwrap()
}

/// Get a sine sample from a given time
fn sine_sample(t: f32) -> f32 {
    (t * 440.0 * 2.0 * PI).sin()
}

/// Get a vector of sine samples
fn sine_float_samples() -> Vec<f32> {
    (0..SAMPLE_RATE)
        .map(|x| x as f32 / SAMPLE_RATE as f32)
        .map(sine_sample)
        .collect()
}

/// Get a vector of sine samples as i16
fn sine_int_samples() -> Vec<i16> {
    let amplitude = i16::MAX as f32;
    sine_float_samples()
        .iter()
        .map(|x| (x * amplitude) as i16)
        .collect()
}

/// Generate a sine wave (mono & 16bits) and write it to a file
fn mono_int_sine() {
    let mut writer = hound::WavWriter::create(INT_MONO_SINE, MONO_INT).unwrap();

    for t in sine_int_samples() {
        writer.write_sample(t).unwrap();
    }

    writer.finalize().unwrap();
}

/// Generate a sine wave (stereo & 32bits) and write it to a file
fn stereo_float_sine() {
    let mut writer = hound::WavWriter::create(FLOAT_STEREO_SINE, STEREO_FLOAT).unwrap();

    for t in sine_float_samples() {
        writer.write_sample(t).unwrap();
        writer.write_sample(t).unwrap();
    }

    writer.finalize().unwrap();
}

/// Get the euclidean distance between paired samples
fn channel_error(left: &[f32], right: &[f32]) -> Vec<f64> {
    left.iter()
        .zip(right)
        .map(|(a, b)| f64::abs(*a as f64 - *b as f64))
        .collect()
}

/// Get the first sample where the channel error is smaller than the threshold
fn error_smaller_than(left: &[f32], right: &[f32], threshold: f64) -> Option<usize> {
    channel_error(left, right)
        .iter()
        .position(|x| *x > threshold)
}

/// Split the buffer into chunks and assert that the error is smaller than the threshold
fn chunked_error_asssert(a: &[f32], b: &[f32], chunk_size: usize, error_threshold: f64) {
    a.chunks_exact(chunk_size)
        .zip(b.chunks_exact(chunk_size))
        .enumerate()
        .for_each(
            |(idx, (a, b))| match error_smaller_than(a, b, error_threshold) {
                Some(error_at) => {
                    println!(
                        "Error threshold exceeded at buffer idx {} at sample idx {} with error {}",
                        idx,
                        error_at,
                        f32::abs(a[idx] - b[idx])
                    );
                    let start_slice = std::cmp::max(0, error_at as i64 - 5) as usize;
                    let end_slice = std::cmp::min(a.len(), error_at + 5);

                    println!("A: {:?}", &b[start_slice..end_slice]);
                    println!("B: {:?}", &a[start_slice..end_slice]);
                    panic!("Error threshold exceeded");
                }
                None => {}
            },
        );
}

/// Check if a buffer is silence
fn is_perfect_silence(buf: &[f32]) -> bool {
    buf.iter().all(|x| *x == 0.0)
}

/// Check wether a buffer is closely related to silence or not
fn assert_silence(buf: &[f32]) {
    if is_perfect_silence(buf) {
        return;
    }
    println!("Buffer is not perfectly silent");
    let avg = buf.iter().fold(0.0f64, |acc, x| acc + f64::abs(*x as f64)) / buf.len() as f64;
    assert!(avg < ACCEPTABLE_ERROR, "Buffer is not silent, avg: {}", avg);
}

/// Assert that the buffer is identical to the target
fn assert_integrity(
    left_target: &[f32],
    right_target: &[f32],
    buffer: &StereoBuffer,
    error_threshold: f64,
) {
    chunked_error_asssert(
        &buffer.left.buf[..left_target.len()],
        left_target,
        HOST_BUFFER_SIZE,
        error_threshold,
    );

    chunked_error_asssert(
        &buffer.right.buf[..right_target.len()],
        right_target,
        HOST_BUFFER_SIZE,
        error_threshold,
    );

    let buf_len_diff = buffer.channel_capacity() - left_target.len();
    if buf_len_diff > 0 {
        assert_silence(&buffer.left.buf[left_target.len()..]);
        assert_silence(&buffer.right.buf[right_target.len()..]);
    }
}

#[test]
fn read_mono_int_wav() {
    mono_int_sine();

    let mut cmp_reader = hound::WavReader::open(INT_MONO_SINE).unwrap();

    assert_eq!(cmp_reader.spec(), MONO_INT);

    let samples: Vec<i16> = cmp_reader.samples().map(|x| x.unwrap()).collect();

    assert_eq!(samples, sine_int_samples());

    let mut reader = default_reader(PathBuf::from(INT_MONO_SINE));

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

    let mut reader = default_reader(PathBuf::from(FLOAT_STEREO_SINE));

    reader.read_sync().unwrap();
    assert_eq!(reader.buffer.capacity() % HOST_BUFFER_SIZE, 0);

    assert_integrity(&pregen_sine, &pregen_sine, &reader.buffer, ACCEPTABLE_ERROR);
}

#[test]
fn read_stereo_mp3() {
    // TODO fails because of padding and delay which is not handled yet
    stereo_float_sine();
    let path = convert_audio_to(FLOAT_STEREO_SINE, "mp3");

    let mut reader = default_reader(path);

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
fn read_stereo_flac() {
    stereo_float_sine();
    let path = convert_audio_to(FLOAT_STEREO_SINE, "flac");

    let mut reader = default_reader(path);

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
