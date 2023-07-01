# Audio reader

The goal is to decode various audio formats in an uniform way. Currently all readers retain samples in the form of `StereoBuffer`s, however specialized `MonoBuffer` readers may be planned. All inner buffers are of type `f32` and are normalized to the range `[-1.0, 1.0]`.

The readers can be issued for slices of a given **host_buffer_size** incrementally, the purpose being easy integration with **audio plugins** and **real-time audio processing**.

Currently there is only a `SyncFullReader` which reads the entire file into memory in one go. This is not ideal for large files, but it is a good starting point for writing tests.

## TODO

- [ ] Fix decoding `delay` and `padding` not being accounted for
- [ ] Add `SyncIncrementalReader` which issues synchronous reads when the buffer is exhausted.
- [ ] Add `IncrementalReader` which supports asynchronous reads when the buffer is exhausted.
- [ ] Add cursor management
- [ ] Add a `ReaderBuilder` which follows some euristic to determine the best reader for a given file.
- [ ] Add a `SampleReader` which also caches some parts of the samples in memory to eliviate processing delay.
