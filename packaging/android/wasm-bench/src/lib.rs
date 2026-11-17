//! A wasm mod for timing wasm mods on a phone. On every server tick it does the same amount of
//! ordinary mod work: integer math, sorting a vector, filling and reading a map. `/modprofile start`
//! then `/modprofile report` give how long its `server_tick` takes on this phone's engine
//! (Cranelift on 64-bit, Pulley on 32-bit). Every ten seconds it says in the chat the checksum of
//! that tick's work, which is the same on any engine for the same tick: a wrong one means the
//! engine computed something else.
use mod_api::{Hooks, ServerHost, ServerTickEvent, Text};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct Mod;

static TICKS: AtomicU64 = AtomicU64::new(0);

/// The next value of a xorshift stream.
fn next(x: &mut u64) -> u64 {
    *x ^= *x << 13;
    *x ^= *x >> 7;
    *x ^= *x << 17;
    *x
}

/// One tick's work, from `seed`; its checksum.
fn work(seed: u64) -> u64 {
    let mut x = seed | 1;
    // Integer math: the stream folded into an FNV-1a hash.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for _ in 0..20_000 {
        hash = (hash ^ (next(&mut x) & 0xff)).wrapping_mul(0x0100_0000_01b3);
    }
    // Memory: a vector filled from the stream, then sorted.
    let mut values: Vec<u32> = (0..2048u32).map(|i| (next(&mut x) as u32) ^ i).collect();
    values.sort_unstable();
    hash ^= u64::from(values[values.len() / 2]);
    // A map: every fourth value in, then every eighth looked up.
    let mut map = BTreeMap::new();
    for (i, key) in values.iter().step_by(4).enumerate() {
        map.insert(*key, i as u32);
    }
    let found: u64 = values
        .iter()
        .step_by(8)
        .filter_map(|key| map.get(key))
        .map(|&i| u64::from(i))
        .sum();
    hash ^ found
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, mod_api::ServerTickPhase::ServerTickEnd { .. }) {
            return;
        }
        let ticks = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
        let checksum = work(ticks);
        if ticks % 200 == 0 {
            host.messages().broadcast(Text::literal(format!(
                "wasm bench: tick {ticks}, checksum {checksum:016x}"
            )));
        }
    }
}
