//! A wasm mod to check that a phone runs wasm mods: the server thread's ticks and the game
//! thread's, counted by hooks each thread calls, said in the chat every ten seconds.
use mod_api::{ClientHost, ClientTickPostEvent, Hooks, ServerHost, ServerTickEvent, Text};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct Mod;

static SERVER_TICKS: AtomicU64 = AtomicU64::new(0);
static CLIENT_TICKS: AtomicU64 = AtomicU64::new(0);

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, mod_api::ServerTickPhase::ServerTickEnd { .. }) {
            return;
        }
        let ticks = SERVER_TICKS.fetch_add(1, Ordering::Relaxed) + 1;
        if ticks % 200 == 0 {
            let client = CLIENT_TICKS.load(Ordering::Relaxed);
            host.messages().broadcast(Text::literal(format!(
                "wasm probe: {ticks} server ticks, {client} game ticks"
            )));
        }
    }

    fn client_tick_post(&self, _: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        CLIENT_TICKS.fetch_add(1, Ordering::Relaxed);
    }
}
