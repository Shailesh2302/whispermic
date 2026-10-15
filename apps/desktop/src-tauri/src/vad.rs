//! Energy + zero-crossing VAD gate (~0.2ms per 30ms frame).
//! Drops silence/breath before STT so we save ~60% compute and cut latency.
//! Tune `threshold` (RMS) per mic in Settings if needed.

pub struct Vad {
    /// RMS energy threshold above which frame counts as speech.
    pub threshold: f32,
    /// Frames of hangover to keep gate open after speech ends (smooths word gaps).
    hangover_frames: usize,
    hangover_left: usize,
}

impl Vad {
    pub fn new(threshold: f32) -> Self {
        Self { threshold, hangover_frames: 8, hangover_left: 0 }
    }

    /// 480 samples @16kHz = 30ms. Returns true if speech (or within hangover).
    pub fn is_speech(&mut self, frame: &[f32]) -> bool {
        if frame.is_empty() {
            return self.hangover_left > 0;
        }
        let mut sum = 0.0f32;
        let mut zc = 0usize;
        let mut prev = frame[0];
        for &s in frame {
            sum += s * s;
            if (s >= 0.0) != (prev >= 0.0) {
                zc += 1;
            }
            prev = s;
        }
        let rms = (sum / frame.len() as f32).sqrt();
        // voiced speech: enough energy + not pure hiss (zc guard kills fan noise)
        let voiced = rms > self.threshold && zc < frame.len() / 2;
        if voiced {
            self.hangover_left = self.hangover_frames;
            true
        } else if self.hangover_left > 0 {
            self.hangover_left -= 1;
            true
        } else {
            false
        }
    }

    /// Raw loudness 0..1 for the overlay level meter.
    /// (Used by future UI wiring; kept out of the hot path on purpose.)
    #[allow(dead_code)]
    pub fn level(frame: &[f32]) -> f32 {
        if frame.is_empty() {
            return 0.0;
        }
        let sum: f32 = frame.iter().map(|s| s * s).sum();
        (sum / frame.len() as f32).sqrt().clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_is_not_speech() {
        let mut v = Vad::new(0.02);
        assert!(!v.is_speech(&vec![0.0; 480]));
    }
    #[test]
    fn loud_sine_is_speech() {
        let mut v = Vad::new(0.02);
        let f: Vec<f32> = (0..480).map(|i| (i as f32 * 0.1).sin() * 0.3).collect();
        assert!(v.is_speech(&f));
    }
}
