use std::f32::consts::TAU;
use std::sync::Arc;

use super::{SAMPLE_RATE, ambience::XorShift64, encode_wav};

fn render(duration: f32, mut sample: impl FnMut(f32) -> f32) -> Arc<[u8]> {
    let count = (duration * SAMPLE_RATE as f32) as usize;
    encode_wav(
        count,
        (0..count).map(|i| sample(i as f32 / SAMPLE_RATE as f32)),
    )
}

pub fn step(seed: u64, duration: f32, pitch: f32, gain: f32) -> Arc<[u8]> {
    let mut rng = XorShift64::new(seed);
    let mut low = 0.0;
    render(duration, |t| {
        let white = rng.next_f32() * 2.0 - 1.0;
        low += (white - low) * 0.18;
        let phase = TAU * pitch * t * (1.0 - t / duration * 0.33);
        let impact = phase.sin() * (-t / 0.032).exp();
        let grit = (white * 0.30 + low * 1.55) * (1.0 - t / duration).powi(2);
        let envelope = (t / 0.006).min(1.0) * (1.0 - t / duration);
        (impact * 0.58 + grit) * gain * envelope
    })
}

pub fn door(enter: bool) -> Arc<[u8]> {
    let mut rng = XorShift64::new(0xD002);
    let mut low = 0.0;
    render(0.65, |t| {
        let white = rng.next_f32() * 2.0 - 1.0;
        low += (white - low) * 0.24;
        let knock = |start: f32, pitch: f32, decay: f32| {
            let elapsed: f32 = t - start;
            if elapsed < 0.0 {
                return 0.0;
            }
            let phase = TAU * pitch * elapsed;
            let tone = phase.sin() * 0.5 + (phase * 2.57).sin() * 0.3 + low;
            tone * (elapsed * 300.0).min(1.0) * (-elapsed * decay).exp()
        };
        (knock(0.0, 670.0, 58.0) * 0.28
            + knock(0.08, 240.0, 24.0) * 0.39
            + knock(if enter { 0.37 } else { 0.46 }, 95.0, 26.0) * 0.62)
            * (1.0 - t / 0.65)
    })
}

pub fn engine() -> Arc<[u8]> {
    render(1.0, |t| {
        let phase = TAU * 48.0 * t;
        let firing = 0.62 * phase.sin() + 0.31 * (phase * 2.0).sin() + 0.16 * (phase * 4.0).sin();
        let rumble = 0.38 * (TAU * 24.0 * t).sin() + 0.15 * (TAU * 137.0 * t).sin();
        let rattle = (TAU * 397.0 * t).sin() * phase.cos().powi(4) * 0.12;
        (firing + rumble + rattle) * (0.32 + 0.06 * (TAU * 5.0 * t).sin())
    })
}
