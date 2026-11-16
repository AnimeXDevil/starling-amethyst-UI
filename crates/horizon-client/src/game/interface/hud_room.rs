//! The HUD's `Room` this frame (`docs/ui-kit.md`): what vanilla's HUD shows that Horizon's own
//! overlays keep clear of. `hud` empties it each frame and `hud_layers` fills it in before the
//! world's HUD draws, into `ui.room`; the controller notice, splitscreen's join strip and the
//! radial menu read it. With it, the frame's `motion::Clock` (`hud_clock`), and whether an
//! overlay holds the view (`overlay_holds_view`), which what vanilla draws there gives way to.
use super::{CREATIVE_SHIFT, Game, Screen};
use horizon_client::hud::HudBatch;
use horizon_client::hud_kit::motion::Clock;
use horizon_client::hud_kit::room::{Keep, Room};
use horizon_client::{boss_overlay, hud_layout, hud_overlays};

/// The HUD works in local safe-area coordinates. Move its vertices and item
/// scissors back into the full frame after drawing it.
pub(super) struct HudArea {
    full: [f32; 2],
    origin: [f32; 2],
    first_vertex: usize,
    first_blend: usize,
}

impl HudArea {
    pub(super) fn begin(batch: &mut HudBatch, [left, top, right, bottom]: [f32; 4]) -> Self {
        let area = Self {
            full: [batch.width, batch.height],
            origin: [left, top],
            first_vertex: batch.vertices.len(),
            first_blend: batch.blends.len(),
        };
        batch.width -= left + right;
        batch.height -= top + bottom;
        area
    }

    pub(super) fn finish(self, batch: &mut HudBatch) {
        let [x, y] = self.origin;
        [batch.width, batch.height] = self.full;
        if self.origin == [0.0; 2] {
            return;
        }
        for vertex in &mut batch.vertices[self.first_vertex..] {
            vertex.pos[0] += x;
            vertex.pos[1] += y;
        }
        for (_, blend) in &mut batch.blends[self.first_blend..] {
            use horizon_client::hud::Blend;
            if let Blend::Depth(rect) | Blend::DepthGlint(rect) = blend {
                rect[0] += (x * batch.scale) as u32;
                rect[1] += (y * batch.scale) as u32;
            }
        }
    }
}

impl Game<'_> {
    pub(super) fn draw_performance_overlays(&mut self, batch: &mut HudBatch) {
        let margins = self.hud_safe_insets(batch);
        let area = HudArea::begin(batch, margins);
        self.frame_graph
            .draw(self.settings.horizon.overlay, batch, &self.gui.atlas);
        self.draw_stats_overlay(batch);
        area.finish(batch);
    }

    pub(super) fn draw_hud_crosshair(
        &mut self,
        batch: &mut HudBatch,
        spectator: bool,
        indicator: u8,
        strength: f32,
        delay: f32,
    ) {
        let (wi, hi) = (batch.width as i32, batch.height as i32);
        // Hud.extractCrosshair / extractItemHotbar / extractPlayerHealth /
        // extractVehicleHealth and the experience bar, laid out by hud_layout.
        // Combat feel Legacy hides the attack indicator unless the player keeps it.
        // The 3D crosshair (F3) takes the 2D one's place.
        // Hud.extractCrosshair: a spectator only sees it over something to open.
        if self.camera_type.is_first_person()
            && (!spectator || self.spectator_crosshair())
            && !self.debug_crosshair()
            && !self.touch_hides_crosshair()
            && !self.overlay_holds_view()
        {
            let attack = (indicator == 1).then_some(strength);
            let full = attack.is_some_and(|a| {
                hud_layout::full_attack_indicator(a, delay, self.crosshair_living())
            });
            let draws = hud_layout::crosshair(wi, hi, attack, full);
            // Under one creative window it hides; Bedrock Inventory has a gap at the middle.
            if self.screen != Screen::Creative || CREATIVE_SHIFT.get() == 0.0 {
                self.gui_layer("minecraft:crosshair", batch, |g, batch| {
                    // The custom crosshair takes the sprite's place (the first draw); the
                    // attack indicator stays.
                    if g.custom_crosshair() {
                        g.draw_custom_crosshair(batch, wi, hi);
                        hud_layout::paint(batch, &g.gui.atlas, &draws[1..]);
                    } else {
                        hud_layout::paint(batch, &g.gui.atlas, &draws);
                    }
                });
            }
        }
    }

    pub(super) fn hud_safe_insets(&self, batch: &HudBatch) -> [f32; 4] {
        #[cfg(target_os = "android")]
        {
            use horizon_client::android_display::{DisplayOption as D, safe_zone};
            safe_zone(
                [batch.width, batch.height],
                [
                    D::SafeHorizontal.value(&self.settings),
                    D::SafeVertical.value(&self.settings),
                ],
                crate::android::safe_insets()
                    .map(|pixels| (pixels as f32 / batch.scale.max(1.0)).ceil()),
            )
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = batch;
            [0.0; 4]
        }
    }

    #[cfg(target_os = "android")]
    pub(super) fn draw_safe_zone(&self, batch: &mut HudBatch) {
        let [left, top, right, bottom] = self.hud_safe_insets(batch);
        let [right, bottom] = [batch.width - right - 2.0, batch.height - bottom - 2.0];
        for (x, y, dx, dy) in [
            (left, top, 1.0, 1.0),
            (right, top, -1.0, 1.0),
            (left, bottom, 1.0, -1.0),
            (right, bottom, -1.0, -1.0),
        ] {
            for [x, y, w, h] in [
                [x.min(x + 12.0 * dx), y, 14.0, 2.0],
                [x, y.min(y + 12.0 * dy), 2.0, 14.0],
            ] {
                batch.fill(&self.gui.atlas, x + 1.0, y + 1.0, w, h, 0xff162026);
                batch.fill(&self.gui.atlas, x, y, w, h, u32::MAX);
            }
        }
    }
    /// The HUD's clock this frame (`hud_kit::motion`): still, so overlays show their motion
    /// settled, while the game is paused, frame-stepped (`HORIZON_STILL`, a screenshot
    /// sequence) or a repro recording or replay runs.
    pub(super) fn hud_clock(&self) -> Clock {
        let still = self.pause
            || self.clock.still()
            || self.screenshot_sequence.is_some()
            || self.repro.recording_active()
            || self.repro.replaying();
        Clock::new(std::time::Instant::now(), still)
    }

    /// What the HUD shows on a `w` x `h` GUI px view this frame, as keep-outs: the boss bars
    /// while the HUD shows (`show_hud`: not F1, not Replay Studio), as wide as the bar or its
    /// widest name.
    pub(super) fn hud_room(&self, w: f32, h: f32, show_hud: bool) -> Room {
        let mut room = Room::new(w, h);
        if !show_hud {
            return room;
        }
        let (wi, hi) = (w as i32, h as i32);
        let bars = self.boss_overlay.bars(boss_overlay::now_millis());
        let drawn = hud_overlays::boss_bars_drawn(hi, bars.len());
        if drawn > 0 {
            let font = &self.gui.atlas.font;
            let half = bars[..drawn]
                .iter()
                .map(|b| (hud_layout::width(font, &b.name.resolve_legacy()) + 1) / 2)
                .fold(hud_overlays::BOSS_BAR_WIDTH / 2, i32::max);
            let bottom = hud_overlays::boss_bars_bottom(hi, bars.len());
            room.add(
                Keep::BossBars,
                [
                    (wi / 2 - half) as f32,
                    0.0,
                    (2 * half) as f32,
                    bottom as f32,
                ],
            );
        }
        room
    }

    /// The status rows above the hotbar in a `w` x `h` GUI px view, from what they draw this
    /// frame (`draws`): a keep-out from their highest row's top down, as wide as the hotbar. Called
    /// as they are laid out, after `hud_room`, so the overlays drawn over the HUD see them.
    pub(super) fn keep_status_rows(&mut self, draws: &[hud_layout::GuiDraw], w: i32, h: i32) {
        if let Some(top) = hud_layout::draws_top(draws).filter(|top| *top < h) {
            let half = hud_overlays::HOTBAR_HALF_WIDTH;
            self.ui.room.add(
                Keep::StatusRows,
                [
                    (w / 2 - half) as f32,
                    top as f32,
                    (2 * half) as f32,
                    (h - top) as f32,
                ],
            );
        }
    }

    /// An overlay holds the middle of the view (the radial menu, once it shows: a quick tap of
    /// its opener draws nothing, so nothing blinks). The 2D and F3 crosshairs, the chat's lines,
    /// the toasts and the first-person hands give way to it.
    pub(super) fn overlay_holds_view(&self) -> bool {
        self.pad_radial_shown()
    }

    /// ToastManager.extractRenderState: F1 hides the toasts (`hidden`), and so does an overlay
    /// holding the view, as the radial menu's page tabs run across the top where the toasts are.
    /// They go on timing out meanwhile, as under F1.
    pub(super) fn draw_every_toast(&mut self, batch: &mut HudBatch, hidden: bool) {
        let hidden = hidden || self.overlay_holds_view();
        self.draw_toasts(batch, hidden);
        self.draw_tutorial_toasts(batch, hidden);
        self.draw_pack_toasts(batch, hidden);
        self.draw_music_toast(batch, hidden);
    }

    /// The hands drawn in first person this frame: none while an overlay holds the view, as the
    /// radial menu's tiles sit where they are.
    pub(super) fn hands_shown(&self) -> [bool; 2] {
        if self.overlay_holds_view() {
            [false; 2]
        } else {
            self.hands_to_render()
        }
    }
}
