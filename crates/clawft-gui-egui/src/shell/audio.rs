//! Tiny procedural "crab scuttle" — a handful of short percussive clicks.
//!
//! Compiled only when the `audio` feature is enabled so builds on hosts
//! without alsa-dev headers stay green. When disabled, `play_scuttle`
//! is a no-op.

#[cfg(feature = "audio")]
pub fn play_scuttle() {
    std::thread::Builder::new()
        .name("clawft-gui-scuttle".into())
        .spawn(scuttle_thread)
        .ok();
}

#[cfg(not(feature = "audio"))]
pub fn play_scuttle() {}

#[cfg(feature = "audio")]
fn scuttle_thread() {
    use rodio::source::{SineWave, Source};
    use std::time::Duration;

    let Ok((_stream, handle)) = rodio::OutputStream::try_default() else {
        return;
    };
    // Sequence of little chitter clicks — 5 short transients with
    // decaying envelope and random-ish pitch. All derived from a base
    // frequency to stay small and dependency-light.
    let sink = match rodio::Sink::try_new(&handle) {
        Ok(s) => s,
        Err(_) => return,
    };
    let bursts: &[(f32, u64)] = &[
        (2400.0, 28),
        (2900.0, 22),
        (2100.0, 34),
        (3200.0, 18),
        (2600.0, 26),
    ];
    for (freq, dur_ms) in bursts {
        let tone = SineWave::new(*freq)
            .take_duration(Duration::from_millis(*dur_ms))
            .amplify(0.08)
            .fade_in(Duration::from_millis(2));
        sink.append(tone);
        // tiny gap between clicks
        sink.append(
            SineWave::new(60.0)
                .take_duration(Duration::from_millis(35))
                .amplify(0.0),
        );
    }
    sink.sleep_until_end();
}
