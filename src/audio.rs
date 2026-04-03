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

    /// Create a new Sink for a pad. The caller holds the Sink to allow stopping/restarting.
    pub fn create_sink(&self) -> Option<Sink> {
        Sink::try_new(&self.stream_handle).ok()
    }

    /// Play audio data immediately into the given sink.
    pub fn play_into_sink(&self, sink: &Sink, data: Arc<Vec<u8>>) {
        sink.clear();
        let cursor = Cursor::new((*data).clone());
        if let Ok(source) = Decoder::new(cursor) {
            sink.append(source);
            sink.play();
        }
    }

    /// Play audio data in a fire-and-forget sink (for polyphony without restart semantics).
    pub fn play_sample(&self, data: Arc<Vec<u8>>) {
        let sink = match Sink::try_new(&self.stream_handle) {
            Ok(s) => s,
            Err(_) => return,
        };
        let cursor = Cursor::new((*data).clone());
        if let Ok(source) = Decoder::new(cursor) {
            sink.append(source);
            sink.detach();
        }
    }
}
