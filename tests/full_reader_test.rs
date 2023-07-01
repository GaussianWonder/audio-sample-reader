mod common;

use common::*;

use std::i16;
use std::path::PathBuf;

use audio_reader::prelude::*;

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

/// Generate a .wav sine wave, convert it with ffmpeg and test the integrity of the result against the error_threshold
pub fn read_other_format(ext: &'static str, error_threshold: f64) {
    stereo_float_sine();
    let path = convert_audio_to(FLOAT_STEREO_SINE, ext);

    let mut reader = default_reader(path);

    reader.read_sync().unwrap();
    assert_eq!(reader.buffer.capacity() % HOST_BUFFER_SIZE, 0);
    assert_eq!(
        reader.buffer.left.capacity(),
        reader.buffer.right.capacity()
    );

    let pregen_sine = sine_float_samples();
    assert_integrity(&pregen_sine, &pregen_sine, &reader.buffer, error_threshold);
}

/// Tests the reading capabilities against an external reader
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

/// Tests reading capabilities against an external reader
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
    read_other_format("mp3", ACCEPTABLE_FLOAT_ERROR);
}

#[test]
fn read_stereo_flac() {
    read_other_format("flac", ACCEPTABLE_FLOAT_ERROR);
}

#[test]
fn read_stereo_ogg() {
    // TODO fails because of delay which is not handled yet
    read_other_format("ogg", ACCEPTABLE_FLOAT_ERROR);
}
