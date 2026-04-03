use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;
use std::sync::Arc;

pub struct AudioEngine {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
}

impl AudioEngine {
    pub fn new() -> Option<Self> {
        let (stream, stream_handle) = OutputStream::try_default().ok()?;
        Some(AudioEngine {
            _stream: stream,
            stream_handle,
        })
    }

    /// Play audio data immediately. Supports polyphony by creating a new Sink per call.
    pub fn play_sample(&self, data: Arc<Vec<u8>>) {
        let sink = match Sink::try_new(&self.stream_handle) {
            Ok(s) => s,
            Err(_) => return,
        };
        let cursor = Cursor::new((*data).clone());
        if let Ok(source) = Decoder::new(cursor) {
            sink.append(source);
            sink.detach(); // let it play without blocking
        }
    }
}
