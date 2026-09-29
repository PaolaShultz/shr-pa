//! Opt-in offline timing evidence, never opens a device. Includes all EQ sections
//! and compressor, and simultaneous filter/delay/gain/limiter transactions.
use shr_pa::{
    config::{Config, EqKind, GEQ_HZ},
    dsp::{Engine, Prepared},
};
use std::time::Instant;
fn main() {
    let seconds: usize = std::env::args()
        .nth(1)
        .unwrap_or("30".into())
        .parse()
        .expect("seconds");
    assert!((1..=600).contains(&seconds));
    let mut c = Config::default();
    c.geq.enabled = true;
    c.compressor.enabled = true;
    c.compressor.threshold_db = -30.;
    c.input_delay_ms = 100.;
    c.geq.db = std::array::from_fn(|_| std::array::from_fn(|i| if i % 2 == 0 { -1. } else { 1. }));
    for es in &mut c.input_eq {
        for (i, e) in es.iter_mut().enumerate() {
            e.hz = GEQ_HZ[i * 4];
            e.db = if i % 2 == 0 { -1. } else { 1. };
        }
    }
    for b in &mut c.bands {
        b.delay_ms = 10.;
        for (i, e) in b.eq.iter_mut().enumerate() {
            e.hz = 80. * 2_f64.powi(i as i32);
            e.db = if i % 2 == 0 { -1. } else { 1. };
            e.kind = if i == 0 {
                EqKind::LowShelf
            } else if i == 7 {
                EqKind::HighShelf
            } else {
                EqKind::Bell
            };
        }
    }
    let mut alternate = c;
    for gains in &mut alternate.geq.db {
        for g in gains {
            *g = -*g;
        }
    }
    for es in &mut alternate.input_eq {
        for e in es {
            e.db = -e.db;
        }
    }
    for b in &mut alternate.bands {
        b.delay_ms = 3.;
        b.gain_db = -3.;
        b.limiter_db = -10.;
        for e in &mut b.eq {
            e.db = -e.db;
        }
    }
    alternate.input_delay_ms = 30.;
    alternate.compressor.ratio = 8.;
    let changes = [
        Prepared::new(c, false).unwrap(),
        Prepared::new(alternate, false).unwrap(),
    ];
    let blocks = seconds * 48000 / 128;
    let mut times = Vec::with_capacity(blocks);
    let mut e = Engine::new(c).unwrap();
    e.set_mutes([false; 6]);
    let mut g = shr_pa::offline::Generator::new(
        shr_pa::offline::Signal::Noise,
        48000,
        (blocks * 128) as u64,
    )
    .unwrap();
    let mut input = [[0.; 2]; 128];
    let mut output = [[0.; 6]; 128];
    let mut transitions = 0;
    for n in 0..blocks {
        g.fill(&mut input);
        let start = Instant::now();
        if n % 16 == 0 {
            e.apply(changes[(n / 16) % 2]).unwrap();
            transitions += 1;
        }
        e.render(&input, &mut output).unwrap();
        times.push(start.elapsed().as_secs_f64() * 1e6);
        assert!(!e.faulted());
        std::hint::black_box(&output);
    }
    times.sort_by(f64::total_cmp);
    println!(
        "126 EQ sections + LR24 + compressor + 8 max-capacity delays; {seconds}s offline, {blocks} blocks, {transitions} transactions; mean {:.2} us, p99 {:.2} us, max {:.2} us; period 2666.67 us",
        times.iter().sum::<f64>() / times.len() as f64,
        times[times.len() * 99 / 100],
        times.last().unwrap()
    );
}
