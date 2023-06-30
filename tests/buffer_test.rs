use audio_reader::{
    buffer::{Buffer, StereoBuffer},
    buffer_with_size,
};

fn a_test_vec(len: usize) -> Vec<f32> {
    let mut vec = Vec::with_capacity(len);
    for i in 1..(len + 1) {
        vec.push(i as f32);
    }
    vec
}

fn b_test_vec(len: usize) -> Vec<f32> {
    let mut vec = a_test_vec(len);
    vec.reverse();
    vec
}

#[test]
fn create_buffer() {
    let buffer = buffer_with_size(10);
    assert_eq!(buffer.capacity(), 10);
    assert_eq!(buffer.len(), 10);
}

#[test]
fn create_stereo_buffer() {
    let buffer = StereoBuffer::new(10);
    assert_eq!(buffer.capacity(), 20);
    assert_eq!(buffer.left.capacity(), buffer.right.capacity());
    assert_eq!(buffer.left.capacity(), 10);
    assert_eq!(buffer.left.buf.len(), buffer.right.buf.len());
    assert_eq!(buffer.left.buf.len(), 10);
}

#[test]
fn pad_empty_buffer_with_silence() {
    let mut buffer = StereoBuffer::new(10);
    buffer.pad_silence();
    assert!(buffer.left.iter().all(|&x| x == 0f32));
    assert!(buffer.right.iter().all(|&x| x == 0f32));
}

#[test]
fn copy_slices_into_buffer() {
    let mut buffer = StereoBuffer::new(10);
    buffer.append_slices(a_test_vec(10).as_slice(), b_test_vec(10).as_slice());
    assert_eq!(buffer.left.buf, a_test_vec(10));
    assert_eq!(buffer.right.buf, b_test_vec(10));
    assert_eq!(buffer.capacity_left(), 0);
}

#[test]
fn copy_mono_slice_with_overflow() {
    let mut buffer = StereoBuffer::new(10);
    let mut overflow = StereoBuffer::new(10);
    buffer.append_slices_overflow(
        a_test_vec(20).as_slice(),
        a_test_vec(20).as_slice(),
        &mut overflow,
    );

    assert_eq!(buffer.left.buf, buffer.right.buf);
    assert_eq!(buffer.left.buf, a_test_vec(10));

    assert!(overflow.has_content(), "There should be overflow!");

    assert_eq!(overflow.left.buf, overflow.right.buf);
    assert_eq!(overflow.left.buf, a_test_vec(20)[10..]);
    assert_eq!(overflow.capacity(), 20);
    assert_eq!(overflow.capacity_left(), 0);
}

#[test]
fn copy_stereo_slice_with_overflow() {
    let mut buffer = StereoBuffer::new(10);
    let mut overflow = StereoBuffer::new(10);

    buffer.append_slices_overflow(
        a_test_vec(20).as_slice(),
        b_test_vec(20).as_slice(),
        &mut overflow,
    );

    assert_eq!(buffer.left.buf, a_test_vec(10));
    assert_eq!(buffer.right.buf, b_test_vec(20)[..10]);

    assert!(overflow.has_content(), "There should be overflow!");

    assert_eq!(overflow.left.buf, a_test_vec(20)[10..]);
    assert_eq!(overflow.right.buf, b_test_vec(20)[10..]);
    assert_eq!(overflow.capacity(), 20);
    assert_eq!(overflow.capacity_left(), 0);
}

#[test]
fn copy_consecutive() {
    let mut buffer = StereoBuffer::new(20);

    buffer.append_slices_overflow(
        a_test_vec(10).as_slice(),
        &b_test_vec(20)[..10],
        &mut StereoBuffer::_0(),
    );
    assert_eq!(buffer.capacity_left(), 10);
    assert_eq!(buffer.overflow_on(20), 10);
    buffer.append_slices_overflow(
        &a_test_vec(20)[10..],
        &b_test_vec(20)[10..],
        &mut StereoBuffer::_0(),
    );

    assert_eq!(buffer.left.buf, a_test_vec(20));
    assert_eq!(buffer.right.buf, b_test_vec(20));
    assert_eq!(buffer.capacity_left(), 0);
}

#[test]
fn pad_partial_buffer_with_silence() {
    let mut buffer = StereoBuffer::new(20);
    buffer.append_slices_overflow(&a_test_vec(10), &b_test_vec(10), &mut StereoBuffer::_0());
    buffer.pad_silence();
    assert_eq!(buffer.capacity_left(), 0);
    assert!(buffer.left[10..].iter().all(|&x| x == 0f32));
    assert!(buffer.right[10..].iter().all(|&x| x == 0f32));
}

#[test]
fn reserve_extra_samples() {
    let mut buffer = StereoBuffer::new(10);
    buffer.reserve(90);

    assert_eq!(
        buffer.capacity(),
        buffer.right.capacity() + buffer.left.capacity()
    );
    assert_eq!(buffer.left.capacity(), buffer.right.capacity());
    assert_eq!(buffer.left.buf.len(), buffer.right.buf.len());
}

#[test]
fn trim_buffer() {
    let mut buffer = StereoBuffer::new(10);
    buffer.pad_silence();
    buffer.reserve_exact(90);

    assert_eq!(buffer.capacity(), 2 * 100);
    assert_eq!(buffer.left.capacity(), buffer.right.capacity());
    assert_eq!(buffer.left.buf.len(), buffer.right.buf.len());

    buffer.trim();

    assert_eq!(buffer.capacity(), 2 * 10);
    assert_eq!(buffer.left.capacity(), buffer.right.capacity());
    assert_eq!(buffer.left.buf.len(), buffer.right.buf.len());
}
