//! Time compression without a change of pitch (WSOLA), used to work off a
//! backlog: after a Bluetooth output reopened, the sound that waited for it
//! plays slightly faster until it is back in step.
//!
//! Overlapping windows of the input are laid on the output closer together
//! than they were in the input. Each window starts where it matches the
//! natural continuation of the one before best, so the seams do not click.
//! Everything is allocated up front; nothing here blocks.

use std::f32::consts::PI;

use rtrb::Consumer;

use crate::BUS_CHANNELS;

/// Window length in frames (about 21 ms at 48 kHz).
const WINDOW: usize = 1024;
/// Output frames per window: half of it, so the windows sum to one.
const HOP: usize = WINDOW / 2;
/// How far a window may move to find a better match, in frames.
const SEARCH: usize = 256;
/// Capacity of the input buffer in frames.
const CAPACITY: usize = 4 * WINDOW;
/// Input frames a window needs, counted from its nominal start.
pub const NEEDS: usize = WINDOW + 2 * SEARCH;

pub struct Stretcher {
    window: Vec<f32>,
    /// Interleaved input, `CAPACITY` frames.
    input: Vec<f32>,
    input_len: usize,
    /// Nominal start of the next window in `input`, in frames.
    at: f64,
    /// Windowed second half of the previous window, still to be completed.
    tail: Vec<f32>,
    /// Raw second half of the previous window: what should come next.
    natural: Vec<f32>,
    started: bool,
    /// Finished output not yet taken, `2 * HOP` frames at most.
    queue: Vec<f32>,
    queue_pos: usize,
    queue_len: usize,
    /// After `finish`: input from here on is passed through unchanged.
    raw_from: Option<usize>,
}

impl Default for Stretcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Stretcher {
    pub fn new() -> Self {
        let window = (0..WINDOW)
            .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / WINDOW as f32).cos())
            .collect();
        Self {
            window,
            input: vec![0.0; CAPACITY * BUS_CHANNELS],
            input_len: 0,
            at: SEARCH as f64,
            tail: vec![0.0; HOP * BUS_CHANNELS],
            natural: vec![0.0; HOP * BUS_CHANNELS],
            started: false,
            queue: vec![0.0; 2 * HOP * BUS_CHANNELS],
            queue_pos: 0,
            queue_len: 0,
            raw_from: None,
        }
    }

    /// Forgets everything held.
    pub fn reset(&mut self) {
        self.input_len = 0;
        self.at = SEARCH as f64;
        self.started = false;
        self.queue_pos = 0;
        self.queue_len = 0;
        self.raw_from = None;
    }

    /// Whether `read_raw` still has frames of its own to give.
    pub fn has_raw(&self) -> bool {
        self.raw_from.is_some() || self.queue_len > self.queue_pos
    }

    /// Input and output frames held here, not yet played.
    pub fn pending(&self) -> usize {
        let queued = self.queue_len - self.queue_pos;
        match self.raw_from {
            Some(from) => queued + self.input_len.saturating_sub(from),
            None => queued + self.input_len.saturating_sub(self.at as usize),
        }
    }

    /// Moves what fits from the ring buffer into the input buffer.
    fn top_up(&mut self, consumer: &mut Consumer<f32>) {
        let room = CAPACITY - self.input_len;
        let frames = (consumer.slots() / BUS_CHANNELS).min(room);
        if frames == 0 {
            return;
        }
        let Ok(chunk) = consumer.read_chunk(frames * BUS_CHANNELS) else {
            return;
        };
        let (a, b) = chunk.as_slices();
        let start = self.input_len * BUS_CHANNELS;
        self.input[start..start + a.len()].copy_from_slice(a);
        self.input[start + a.len()..start + a.len() + b.len()].copy_from_slice(b);
        chunk.commit_all();
        self.input_len += frames;
    }

    /// Drops input before the earliest frame a window may still use.
    fn compact(&mut self) {
        let drop = (self.at as usize).saturating_sub(SEARCH);
        if drop == 0 {
            return;
        }
        self.input
            .copy_within(drop * BUS_CHANNELS..self.input_len * BUS_CHANNELS, 0);
        self.input_len -= drop;
        self.at -= drop as f64;
    }

    /// Start of the best matching window around the nominal one.
    fn best_start(&self) -> usize {
        let nominal = self.at as usize;
        if !self.started {
            return nominal;
        }
        let mut best = nominal;
        let mut best_score = f32::MIN;
        for start in (nominal - SEARCH..=nominal + SEARCH).step_by(2) {
            let mut dot = 0.0;
            let mut energy = 1e-9;
            for k in (0..HOP).step_by(2) {
                let c = &self.input[(start + k) * BUS_CHANNELS..];
                let n = &self.natural[k * BUS_CHANNELS..];
                let candidate = c[0] + c[1];
                dot += candidate * (n[0] + n[1]);
                energy += candidate * candidate;
            }
            let score = dot / energy.sqrt();
            if score > best_score {
                best_score = score;
                best = start;
            }
        }
        best
    }

    /// Lays the next window on the output. Needs `NEEDS` input frames.
    /// Returns where the window started in the input.
    fn step(&mut self, speed: f32, offset: usize) -> Option<usize> {
        if self.input_len < self.at as usize + SEARCH + WINDOW {
            return None;
        }
        let start = self.best_start();
        for k in 0..HOP {
            for ch in 0..BUS_CHANNELS {
                let x = self.input[(start + k) * BUS_CHANNELS + ch];
                let mixed = if self.started {
                    self.tail[k * BUS_CHANNELS + ch] + x * self.window[k]
                } else {
                    x
                };
                self.queue[(offset + k) * BUS_CHANNELS + ch] = mixed;
                let y = self.input[(start + HOP + k) * BUS_CHANNELS + ch];
                self.tail[k * BUS_CHANNELS + ch] = y * self.window[HOP + k];
                self.natural[k * BUS_CHANNELS + ch] = y;
            }
        }
        self.started = true;
        self.queue_pos = 0;
        self.queue_len = offset + HOP;
        self.at += HOP as f64 * speed as f64;
        Some(start)
    }

    /// Fills `out` (interleaved) with input played `speed` times as fast.
    /// Returns the frames written; fewer than asked only when the input ran
    /// dry.
    pub fn pull(&mut self, consumer: &mut Consumer<f32>, out: &mut [f32], speed: f32) -> usize {
        let frames = out.len() / BUS_CHANNELS;
        let mut done = 0;
        while done < frames {
            if self.queue_pos == self.queue_len {
                self.compact();
                self.top_up(consumer);
                if self.step(speed, 0).is_none() {
                    break;
                }
            }
            let n = (self.queue_len - self.queue_pos).min(frames - done);
            out[done * BUS_CHANNELS..(done + n) * BUS_CHANNELS].copy_from_slice(
                &self.queue[self.queue_pos * BUS_CHANNELS..(self.queue_pos + n) * BUS_CHANNELS],
            );
            self.queue_pos += n;
            done += n;
        }
        done
    }

    /// Ends the compression without a break: the next window is blended in
    /// and what follows is passed through unchanged by `read_raw`.
    pub fn finish(&mut self, consumer: &mut Consumer<f32>) {
        if !self.started {
            self.raw_from = Some(0);
            self.top_up(consumer);
            return;
        }
        self.compact();
        self.top_up(consumer);
        // Output not yet taken stays ahead of what is added now.
        let kept = self.queue_len - self.queue_pos;
        self.queue.copy_within(
            self.queue_pos * BUS_CHANNELS..self.queue_len * BUS_CHANNELS,
            0,
        );
        self.queue_pos = 0;
        self.queue_len = kept;
        let Some(start) = self.step(1.0, kept) else {
            self.raw_from = Some(self.at as usize);
            return;
        };
        self.raw_from = Some(start + HOP);
    }

    /// Takes frames in their original speed: first what is held here, then
    /// from the ring buffer. Returns the frames written.
    pub fn read_raw(&mut self, consumer: &mut Consumer<f32>, out: &mut [f32]) -> usize {
        let frames = out.len() / BUS_CHANNELS;
        let mut done = 0;
        let queued = (self.queue_len - self.queue_pos).min(frames);
        if queued > 0 {
            out[..queued * BUS_CHANNELS].copy_from_slice(
                &self.queue
                    [self.queue_pos * BUS_CHANNELS..(self.queue_pos + queued) * BUS_CHANNELS],
            );
            self.queue_pos += queued;
            done = queued;
        }
        if done < frames && self.queue_pos == self.queue_len {
            if let Some(from) = self.raw_from {
                let n = self.input_len.saturating_sub(from).min(frames - done);
                out[done * BUS_CHANNELS..(done + n) * BUS_CHANNELS]
                    .copy_from_slice(&self.input[from * BUS_CHANNELS..(from + n) * BUS_CHANNELS]);
                done += n;
                if from + n >= self.input_len {
                    self.reset();
                } else {
                    self.raw_from = Some(from + n);
                }
            }
        }
        if done < frames && !self.has_raw() {
            let n = (consumer.slots() / BUS_CHANNELS).min(frames - done);
            if n > 0 {
                if let Ok(chunk) = consumer.read_chunk(n * BUS_CHANNELS) {
                    let (a, b) = chunk.as_slices();
                    let at = done * BUS_CHANNELS;
                    out[at..at + a.len()].copy_from_slice(a);
                    out[at + a.len()..at + a.len() + b.len()].copy_from_slice(b);
                    chunk.commit_all();
                    done += n;
                }
            }
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtrb::RingBuffer;

    const RATE: f32 = 48_000.0;

    fn ring_with_sine(hz: f32, frames: usize) -> Consumer<f32> {
        let (mut producer, consumer) = RingBuffer::new(frames * BUS_CHANNELS);
        for n in 0..frames {
            let v = 0.5 * (2.0 * PI * hz * n as f32 / RATE).sin();
            producer.push(v).unwrap();
            producer.push(v).unwrap();
        }
        consumer
    }

    /// Rising zero crossings per second of the left channel.
    fn pitch(out: &[f32]) -> f32 {
        let crossings = out
            .chunks_exact(BUS_CHANNELS)
            .map(|f| f[0])
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        crossings as f32 / (out.len() / BUS_CHANNELS) as f32 * RATE
    }

    #[test]
    fn compresses_time_and_keeps_the_pitch() {
        let mut consumer = ring_with_sine(440.0, 48_000);
        let mut stretcher = Stretcher::new();
        let mut out = Vec::new();
        let mut block = vec![0.0; 128 * BUS_CHANNELS];
        loop {
            let n = stretcher.pull(&mut consumer, &mut block, 1.1);
            out.extend_from_slice(&block[..n * BUS_CHANNELS]);
            if n < 128 {
                break;
            }
        }
        let frames = out.len() / BUS_CHANNELS;
        // A second of input, played 1.1 times as fast, minus what stays in the buffers.
        assert!(
            (40_000..46_500).contains(&frames),
            "{frames} frames out of 48000"
        );
        let hz = pitch(&out);
        assert!((hz - 440.0).abs() < 8.0, "pitch {hz}");
        // Steady loudness, no dropouts at the seams.
        let loudest = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(loudest < 0.56, "peak {loudest}");
        let quiet_stretch = out[4_000..]
            .chunks(960)
            .any(|c| c.iter().fold(0.0f32, |m, v| m.max(v.abs())) < 0.35);
        assert!(!quiet_stretch);
    }

    #[test]
    fn finishing_continues_without_a_gap() {
        let mut consumer = ring_with_sine(440.0, 24_000);
        let mut stretcher = Stretcher::new();
        let mut out = Vec::new();
        let mut block = vec![0.0; 128 * BUS_CHANNELS];
        for _ in 0..40 {
            let n = stretcher.pull(&mut consumer, &mut block, 1.1);
            assert_eq!(n, 128);
            out.extend_from_slice(&block);
        }
        stretcher.finish(&mut consumer);
        while stretcher.has_raw() {
            let n = stretcher.read_raw(&mut consumer, &mut block);
            out.extend_from_slice(&block[..n * BUS_CHANNELS]);
            if n == 0 {
                break;
            }
        }
        let n = stretcher.read_raw(&mut consumer, &mut block);
        out.extend_from_slice(&block[..n * BUS_CHANNELS]);
        let hz = pitch(&out);
        assert!((hz - 440.0).abs() < 8.0, "pitch {hz}");
        let seam = out[2 * 5_120..]
            .chunks(480)
            .all(|c| c.iter().fold(0.0f32, |m, v| m.max(v.abs())) > 0.3);
        assert!(seam);
    }
}
