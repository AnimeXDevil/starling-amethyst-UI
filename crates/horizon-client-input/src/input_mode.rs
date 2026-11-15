//! Which input the player plays with, as Bedrock's input modes: the touch controls, the chat's
//! bar and the on-screen keyboard show only while it is the touch screen. Where the touch screen
//! is the platform's own input (Android), the keyboard, a mouse or a controller put them away and
//! the next touch brings them back; elsewhere (a touch laptop) they stay from the first touch on,
//! the keyboard and mouse beside them. And what Android's window system leaves out about the
//! keyboard: which keys are a physical keyboard's, and the modifiers held.
use std::collections::HashSet;
use winit::event::MouseButton;
use winit::keyboard::{KeyCode, ModifiersState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    Touch,
    Pad,
    KeyboardMouse,
}

impl InputMode {
    /// What the game starts in: the touch screen where it is the platform's own input.
    pub fn first(touch_platform: bool) -> Self {
        if touch_platform {
            InputMode::Touch
        } else {
            InputMode::KeyboardMouse
        }
    }

    /// The mode after the player used `input`.
    pub fn after(self, input: InputMode, touch_platform: bool) -> Self {
        if self == InputMode::Touch && !touch_platform {
            InputMode::Touch
        } else {
            input
        }
    }
}

/// `KeyCharacterMap.VIRTUAL_KEYBOARD`: the device of keys the system makes up (the on-screen
/// keyboard's, the navigation bar's Back).
const VIRTUAL_KEYBOARD: i32 = -1;

/// Whether an Android key press is the player typing on a physical keyboard: one of a keyboard's
/// keys (letters, digits, punctuation, Enter, Escape, the arrows, modifiers, F-keys, the
/// keypad), from a real device that is not a controller. The system's keys (volume, Back, Home,
/// media), a controller's and the on-screen keyboard's (no scan code) are not.
pub fn android_keyboard(source: u32, device: i32, scan_code: i32, keycode: i32) -> bool {
    let typing = matches!(
        keycode,
        7..=16 | 19..=22 | 29..=62 | 66..=77 | 81 | 92 | 93 | 111..=124 | 131..=163
    );
    typing
        && device != VIRTUAL_KEYBOARD
        && scan_code != 0
        && !crate::gamepad::android::from_pad(source)
}

/// The modifiers the held keys make: Android's window system reports none (winit sends no
/// `ModifiersChanged` there).
pub fn held_modifiers(held: &HashSet<KeyCode>) -> ModifiersState {
    let mut modifiers = ModifiersState::empty();
    for (keys, modifier) in [
        (
            [KeyCode::ShiftLeft, KeyCode::ShiftRight],
            ModifiersState::SHIFT,
        ),
        (
            [KeyCode::ControlLeft, KeyCode::ControlRight],
            ModifiersState::CONTROL,
        ),
        ([KeyCode::AltLeft, KeyCode::AltRight], ModifiersState::ALT),
        (
            [KeyCode::SuperLeft, KeyCode::SuperRight],
            ModifiersState::SUPER,
        ),
    ] {
        if keys.iter().any(|k| held.contains(k)) {
            modifiers |= modifier;
        }
    }
    modifiers
}

/// How far a mouse Android has not captured went in one event: its `AXIS_RELATIVE_X/Y`
/// (`relative`, the event's history summed in), or, for a mouse whose events carry none (some
/// phones' Bluetooth and OTG mice), how far its pointer went since the event before (`before` to
/// `at`), as Horizon Android 0.7.8 reads it. With only the first, the view stood still or moved
/// in jerks on those mice while the game held the pointer and Android had not captured it.
pub fn free_mouse_motion(
    relative: (f32, f32),
    before: Option<(f32, f32)>,
    at: (f32, f32),
) -> (f32, f32) {
    if relative != (0.0, 0.0) {
        return relative;
    }
    before.map_or((0.0, 0.0), |(x, y)| (at.0 - x, at.1 - y))
}

/// The bits of Android's `MotionEvent.getButtonState()` and the mouse button each is: primary,
/// secondary, tertiary, back and forward. Back and forward are the side buttons, Java's
/// "mouse 4" and "mouse 5".
const ANDROID_MOUSE_BUTTONS: [(u32, MouseButton); 5] = [
    (1, MouseButton::Left),
    (2, MouseButton::Right),
    (4, MouseButton::Middle),
    (8, MouseButton::Back),
    (16, MouseButton::Forward),
];

/// The buttons an Android mouse holds. A side button comes twice: in the motion events' button
/// state, and as the Back or Forward key (`KEYCODE_BACK` 4, `KEYCODE_FORWARD` 125) Android makes
/// of it; some mice may send only one of the two. A button is held while either says so, and its
/// press and release count once. Before, the side buttons were left to Android as its Back and
/// Forward keys, so they could not be bound to anything in the game.
#[derive(Debug, Default)]
pub struct AndroidMouseButtons {
    motion: u32,
    keys: u32,
    held: u32,
}

impl AndroidMouseButtons {
    pub const fn new() -> Self {
        Self {
            motion: 0,
            keys: 0,
            held: 0,
        }
    }

    /// A motion event's button state: the presses and releases it brings.
    pub fn motion(&mut self, state: u32) -> Vec<(MouseButton, bool)> {
        self.motion = state;
        self.update()
    }

    /// A key from the mouse: its presses and releases when it is a side button's, `None` for
    /// another key.
    pub fn key(&mut self, code: i32, down: bool) -> Option<Vec<(MouseButton, bool)>> {
        let bit = match code {
            4 => 8,
            125 => 16,
            _ => return None,
        };
        if down {
            self.keys |= bit;
        } else {
            self.keys &= !bit;
        }
        Some(self.update())
    }

    fn update(&mut self) -> Vec<(MouseButton, bool)> {
        let now = self.motion | self.keys;
        let news = ANDROID_MOUSE_BUTTONS
            .iter()
            .filter(|(bit, _)| (self.held ^ now) & bit != 0)
            .map(|&(bit, button)| (button, now & bit != 0))
            .collect();
        self.held = now;
        news
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A side button counts once whether the mouse sends it as the motion's button state, as the
    /// Back or Forward key, or both (in either order); the other buttons come only in the motion.
    #[test]
    fn an_android_mouse_side_button_counts_once_however_it_comes() {
        let mut buttons = AndroidMouseButtons::new();
        // Both: Android's key first, then the motion carrying the bit.
        assert_eq!(buttons.key(4, true), Some(vec![(MouseButton::Back, true)]));
        assert_eq!(buttons.motion(8), vec![]);
        assert_eq!(buttons.motion(8 | 1), vec![(MouseButton::Left, true)]);
        assert_eq!(buttons.motion(1), vec![]);
        assert_eq!(
            buttons.key(4, false),
            Some(vec![(MouseButton::Back, false)])
        );
        // Only the motion's state.
        assert_eq!(buttons.motion(1 | 16), vec![(MouseButton::Forward, true)]);
        assert_eq!(
            buttons.motion(0),
            vec![(MouseButton::Left, false), (MouseButton::Forward, false)]
        );
        // Only the key, held across moves that carry no bit.
        assert_eq!(
            buttons.key(125, true),
            Some(vec![(MouseButton::Forward, true)])
        );
        assert_eq!(buttons.motion(0), vec![]);
        assert_eq!(buttons.key(125, true), Some(vec![]), "a repeat is no news");
        assert_eq!(
            buttons.key(125, false),
            Some(vec![(MouseButton::Forward, false)])
        );
        // Another key from the mouse is not a button.
        assert_eq!(buttons.key(66, true), None);
    }

    #[test]
    fn a_free_mouse_moves_by_its_relative_axes_or_else_by_its_pointer() {
        // A mouse that reports how far it went: that, whatever its pointer did.
        assert_eq!(
            free_mouse_motion((3.0, -2.0), Some((10.0, 10.0)), (50.0, 50.0)),
            (3.0, -2.0)
        );
        // One that reports nothing there: the pointer's own move since the event before.
        assert_eq!(
            free_mouse_motion((0.0, 0.0), Some((10.0, 20.0)), (14.0, 17.0)),
            (4.0, -3.0)
        );
        // Its first event has nothing to measure from.
        assert_eq!(
            free_mouse_motion((0.0, 0.0), None, (14.0, 17.0)),
            (0.0, 0.0)
        );
    }

    #[test]
    fn a_phone_follows_the_input_used() {
        let mut mode = InputMode::first(true);
        assert_eq!(mode, InputMode::Touch);
        for input in [
            InputMode::Pad,
            InputMode::KeyboardMouse,
            InputMode::Touch,
            InputMode::KeyboardMouse,
            InputMode::Pad,
            InputMode::Touch,
        ] {
            mode = mode.after(input, true);
            assert_eq!(mode, input);
        }
    }

    #[test]
    fn a_computer_keeps_the_touch_controls_once_touched() {
        let mut mode = InputMode::first(false);
        assert_eq!(mode, InputMode::KeyboardMouse);
        mode = mode.after(InputMode::Pad, false);
        assert_eq!(mode, InputMode::Pad);
        mode = mode.after(InputMode::Touch, false);
        for input in [InputMode::KeyboardMouse, InputMode::Pad] {
            mode = mode.after(input, false);
            assert_eq!(mode, InputMode::Touch);
        }
    }

    #[test]
    fn only_a_physical_keyboards_keys_count() {
        const KEYBOARD: u32 = 0x0301;
        // W, Space, Escape, the left arrow, F3, left shift, keypad 5.
        for code in [51, 62, 111, 21, 133, 59, 149] {
            assert!(android_keyboard(KEYBOARD, 4, 30, code), "{code}");
        }
        // Back, volume up, Home, media play/pause, a pad's A.
        for code in [4, 24, 3, 85, 96] {
            assert!(!android_keyboard(KEYBOARD, 4, 30, code), "{code}");
        }
        // The on-screen keyboard's Enter: the virtual device, or no scan code.
        assert!(!android_keyboard(KEYBOARD, VIRTUAL_KEYBOARD, 28, 66));
        assert!(!android_keyboard(KEYBOARD, 4, 0, 66));
        // A controller's D-pad.
        assert!(!android_keyboard(0x0501, 6, 544, 19));
    }

    #[test]
    fn held_keys_make_the_modifiers() {
        let held = HashSet::from([KeyCode::ShiftRight, KeyCode::ControlLeft, KeyCode::KeyA]);
        assert_eq!(
            held_modifiers(&held),
            ModifiersState::SHIFT | ModifiersState::CONTROL
        );
        assert_eq!(held_modifiers(&HashSet::new()), ModifiersState::empty());
        let held = HashSet::from([KeyCode::AltLeft, KeyCode::SuperRight]);
        assert_eq!(
            held_modifiers(&held),
            ModifiersState::ALT | ModifiersState::SUPER
        );
    }
}
