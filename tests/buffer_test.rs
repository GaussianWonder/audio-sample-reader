use audio_reader::buffer::StereoBuffer;

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
    let buffer = StereoBuffer::buffer_with_size(10);
    assert_eq!(buffer.capacity(), 10);
    assert_eq!(buffer.len(), 10);
}

#[test]
fn create_stereo_buffer() {
    let buffer = StereoBuffer::new(10);
    assert_eq!(buffer.total_capacity(), 20);
    assert_eq!(buffer.left.capacity(), buffer.right.capacity());
    assert_eq!(buffer.left.capacity(), 10);
    assert_eq!(buffer.left.len(), buffer.right.len());
    assert_eq!(buffer.left.len(), 10);
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
    buffer.copy_from_slices(a_test_vec(10).as_slice(), b_test_vec(10).as_slice());
    assert_eq!(buffer.left, a_test_vec(10));
    assert_eq!(buffer.right, b_test_vec(10));
    assert_eq!(buffer.capacity_left(), 0);
}

#[test]
fn copy_buffer_into_buffer() {
    let mut a_buffer = StereoBuffer::new(10);
    let mut b_buffer = StereoBuffer::new(10);
    a_buffer.copy_from_slices(a_test_vec(10).as_slice(), b_test_vec(10).as_slice());
    b_buffer.copy_from_buffer(&a_buffer);
    assert_eq!(a_buffer.left, b_buffer.left);
    assert_eq!(a_buffer.right, b_buffer.right);
    assert_eq!(a_buffer.capacity_left(), b_buffer.capacity_left());
}

#[test]
fn copy_mono_slice_with_overflow() {
    let mut buffer = StereoBuffer::new(10);
    let overflow = buffer.copy_slice_mono(a_test_vec(20).as_slice());

    assert_eq!(buffer.left, buffer.right);
    assert_eq!(buffer.left, a_test_vec(10));

    assert!(overflow.is_some(), "There should be overflow!");

    if let Some(overflow) = overflow {
        assert_eq!(overflow.left, overflow.right);
        assert_eq!(overflow.left, a_test_vec(20)[10..]);
        assert_eq!(overflow.total_capacity(), 20);
        assert_eq!(overflow.capacity_left(), 0);
    }
}

#[test]
fn copy_stereo_slice_with_overflow() {
    let mut buffer = StereoBuffer::new(10);
    let overflow = buffer.copy_slice_stereo(a_test_vec(20).as_slice(), b_test_vec(20).as_slice());

    assert_eq!(buffer.left, a_test_vec(10));
    assert_eq!(buffer.right, b_test_vec(20)[..10]);

    assert!(overflow.is_some(), "There should be overflow!");

    if let Some(overflow) = overflow {
        assert_eq!(overflow.left, a_test_vec(20)[10..]);
        assert_eq!(overflow.right, b_test_vec(20)[10..]);
        assert_eq!(overflow.total_capacity(), 20);
        assert_eq!(overflow.capacity_left(), 0);
    }
}

#[test]
fn swap_buffer_with_vec() {
    let mut buffer = StereoBuffer::new(20);
    buffer.copy_slice_stereo(&b_test_vec(20)[..10], a_test_vec(10).as_slice());

    let mut vec = vec![b_test_vec(10), a_test_vec(20)[10..].to_vec()];
    buffer.swap_with_vec(&mut vec);

    assert_eq!(buffer.capacity_left(), 0);
    assert_eq!(buffer.left, b_test_vec(20));
    assert_eq!(buffer.right, a_test_vec(20));
}

#[test]
fn swap_buffer_with_buffer() {
    let mut a_buffer = StereoBuffer::new(20);
    a_buffer.copy_slice_stereo(&b_test_vec(20)[..10], a_test_vec(10).as_slice());

    let mut b_buffer = StereoBuffer::new(10);
    b_buffer.copy_slice_stereo(b_test_vec(10).as_slice(), &a_test_vec(20)[10..]);

    a_buffer.swap_with_buffer(&mut b_buffer);

    assert_eq!(a_buffer.capacity_left(), 0);
    assert_eq!(a_buffer.left, b_test_vec(20));
    assert_eq!(a_buffer.right, a_test_vec(20));
}

#[test]
fn copy_consecutive() {
    let mut buffer = StereoBuffer::new(20);

    buffer.copy_slice_stereo(a_test_vec(10).as_slice(), &b_test_vec(20)[..10]);
    assert_eq!(buffer.capacity_left(), 10);
    assert_eq!(buffer.overflow_on(20), 10);
    buffer.copy_slice_stereo(&a_test_vec(20)[10..], &b_test_vec(20)[10..]);

    assert_eq!(buffer.left, a_test_vec(20));
    assert_eq!(buffer.right, b_test_vec(20));
    assert_eq!(buffer.capacity_left(), 0);
}

#[test]
fn pad_partial_buffer_with_silence() {
    let mut buffer = StereoBuffer::new(20);
    buffer.copy_slice_stereo(&a_test_vec(10), &b_test_vec(10));
    buffer.pad_silence();
    assert_eq!(buffer.capacity_left(), 0);
    assert!(buffer.left[10..].iter().all(|&x| x == 0f32));
    assert!(buffer.right[10..].iter().all(|&x| x == 0f32));
}
