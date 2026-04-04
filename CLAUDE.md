# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Rust desktop MPC2000-inspired pad sampler with a 4x4 grid UI, audio playback, microphone recording, and loop sequencing. Built with eframe/egui for the GUI, rodio for audio output, and cpal for microphone input.

## Commands

```bash
# Check types (fast, ~20s after first full compile)
cargo check

# Build
cargo build

# Run
cargo run
```

**System dependencies required:** `apt install build-essential pkg-config libasound2-dev`

There are no tests in this project currently.

## Architecture

All application logic lives in `src/main.rs` as `MpcApp`, with three supporting modules:

- **`src/audio.rs`** — `AudioEngine`: wraps a rodio `OutputStream` + per-pad `Sink` pool. `create_sink()` + `play_into_sink()` provide restart semantics; `play_sample()` is fire-and-forget polyphony.
- **`src/recording.rs`** — `RecordingEngine`: cpal-based mic capture using `Arc<AtomicBool>` for thread-safe recording state and `Arc<Mutex<Vec<f32>>>` for sample accumulation. `stop()` encodes captured samples to a 16-bit PCM WAV buffer via the internal `encode_wav()` function.
- **`src/loop_recorder.rs`** — `LoopRecorder`: records `(pad_index, elapsed_ms)` events using `std::time::Instant`. Completed loops stored as `Vec<Loop>` with sequential names (loop#1, loop#2, ...).

### Key MpcApp state

| Field | Purpose |
|---|---|
| `pad_samples: [Option<Arc<Vec<u8>>>; 16]` | WAV-encoded audio buffer per pad |
| `pad_sinks: Vec<Option<Sink>>` | Per-pad rodio sink for restart semantics |
| `pending_record_buffer` | Recorded WAV awaiting pad assignment |
| `pad_press_start: [Option<f64>; 16]` | Timestamps for 3-second long-press clear |
| `selected_loop: Option<usize>` | Currently selected loop in sidebar |
| `loop_playback: Option<LoopPlayback>` | Active single-loop playback state (loop_idx + start_instant + next_event_idx) |
| `available_mic_devices: Vec<String>` | Mic input device names populated at startup |
| `selected_mic_device: Option<String>` | Selected mic device (None = system default) |

### UI layout

`SidePanel::right("loop_sidebar")` must be added **before** `CentralPanel::default()` or it won't render. The pad grid uses `egui::Grid` with `Sense::click_and_drag()` (not `Sense::click()`) so that `is_pointer_button_down_on()` works for long-press detection.

### Keyboard mappings

| Keys | Function |
|---|---|
| Q W E / A S D F / Z X C V / 1 2 3 4 | Pads 1–3, 5–12, 13–16 (pad 4 has no key — R is taken) |
| R | Toggle mic recording |
| L | Toggle loop recording |
| P | Play selected loop |
| O | Stop selected loop |
| Space | Stop All |

### Current progress

US-001 through US-013 are complete (see `prd.json`). Next up:
- **US-014**: Multiple simultaneous loop playback with per-loop stop controls

## Important gotchas

- `cpal::Stream` is not `Send` on all platforms — keep it on the main thread
- `Sense::click_and_drag()` is required (not `Sense::click()`) to use `response.is_pointer_button_down_on()`
- `SidePanel::right(id)` must come before `CentralPanel::default()` in the update loop
- After a long-press completes, `long_press_complete` guard must prevent the click event from also triggering pad playback
- `self.selected_loop` index should be validated against `loop_recorder.loops.len()` before use (loops can be appended)
