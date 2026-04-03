use std::time::Instant;

pub struct LoopEvent {
    pub pad_index: usize,
    pub elapsed_ms: u64,
}

pub struct Loop {
    pub name: String,
    pub events: Vec<LoopEvent>,
}

pub struct LoopRecorder {
    pub loops: Vec<Loop>,
    is_recording: bool,
    start_time: Option<Instant>,
    current_events: Vec<LoopEvent>,
}

impl LoopRecorder {
    pub fn new() -> Self {
        LoopRecorder {
            loops: Vec::new(),
            is_recording: false,
            start_time: None,
            current_events: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.is_recording = true;
        self.start_time = Some(Instant::now());
        self.current_events = Vec::new();
    }

    pub fn stop(&mut self) {
        if !self.is_recording {
            return;
        }
        self.is_recording = false;
        let loop_num = self.loops.len() + 1;
        let name = format!("loop#{}", loop_num);
        let events = std::mem::take(&mut self.current_events);
        self.loops.push(Loop { name, events });
        self.start_time = None;
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    pub fn record_event(&mut self, pad_index: usize) {
        if !self.is_recording {
            return;
        }
        let elapsed_ms = self
            .start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        self.current_events.push(LoopEvent { pad_index, elapsed_ms });
    }
}
