//! Qt key events -> libghostty key identities.

use libghostty_vt::key::{Key, Mods};

pub const QT_SHIFT: u32 = 0x0200_0000;
pub const QT_CTRL: u32 = 0x0400_0000;
pub const QT_ALT: u32 = 0x0800_0000;
pub const QT_META: u32 = 0x1000_0000;
pub const QT_KEYPAD: u32 = 0x2000_0000;

pub fn mods(qt: u32) -> Mods {
    let mut m = Mods::empty();
    if qt & QT_SHIFT != 0 {
        m |= Mods::SHIFT;
    }
    if qt & QT_CTRL != 0 {
        m |= Mods::CTRL;
    }
    if qt & QT_ALT != 0 {
        m |= Mods::ALT;
    }
    if qt & QT_META != 0 {
        m |= Mods::SUPER;
    }
    m
}

/// A modifier key's side and modifier bits. Qt reports Shift, Ctrl, Alt and
/// Meta without a side; the native key tells: XKB keysyms (Shift_R 0xffe2,
/// Control_R 0xffe4, Alt_R 0xffea, Super_R 0xffec, Meta_R 0xffe8), evdev
/// codes + 8 (62, 105, 108, 134) or Windows scan codes (0x36, 0x11d,
/// 0x138, 0x15c). Its own bit is set on the press and cleared on release.
pub fn modifier_key(k: Key, mut mods: Mods, release: bool, vk: u32, scan: u32) -> (Key, Mods) {
    let (left, right, bit, side) = match k {
        Key::ShiftLeft | Key::ShiftRight => (Key::ShiftLeft, Key::ShiftRight, Mods::SHIFT, Mods::SHIFT_SIDE),
        Key::ControlLeft | Key::ControlRight => (Key::ControlLeft, Key::ControlRight, Mods::CTRL, Mods::CTRL_SIDE),
        Key::AltLeft | Key::AltRight => (Key::AltLeft, Key::AltRight, Mods::ALT, Mods::ALT_SIDE),
        Key::MetaLeft | Key::MetaRight => (Key::MetaLeft, Key::MetaRight, Mods::SUPER, Mods::SUPER_SIDE),
        _ => return (k, mods),
    };
    let is_right = k == right
        || matches!(vk, 0xffe2 | 0xffe4 | 0xffe8 | 0xffea | 0xffec | 0xfe03)
        || (vk < 0xfe00 && matches!(scan, 62 | 105 | 108 | 134 | 0x36 | 0x11d | 0x138 | 0x15c));
    if release {
        mods.remove(bit | side);
    } else {
        mods.insert(bit);
        mods.set(side, is_right);
    }
    (if is_right { right } else { left }, mods)
}

/// The physical key and its unshifted codepoint (US layout) for a Qt key.
pub fn key(qt_key: i32, qt_mods: u32) -> Option<(Key, char)> {
    let keypad = qt_mods & QT_KEYPAD != 0;
    let k = qt_key as u32;
    if keypad {
        let kp = match k {
            0x30..=0x39 => Some(match k - 0x30 {
                0 => Key::Numpad0,
                1 => Key::Numpad1,
                2 => Key::Numpad2,
                3 => Key::Numpad3,
                4 => Key::Numpad4,
                5 => Key::Numpad5,
                6 => Key::Numpad6,
                7 => Key::Numpad7,
                8 => Key::Numpad8,
                _ => Key::Numpad9,
            }),
            0x2b => Some(Key::NumpadAdd),
            0x2d => Some(Key::NumpadSubtract),
            0x2a => Some(Key::NumpadMultiply),
            0x2f => Some(Key::NumpadDivide),
            0x2e => Some(Key::NumpadDecimal),
            0x3d => Some(Key::NumpadEqual),
            0x0100_0005 => Some(Key::NumpadEnter),
            _ => None,
        };
        if let Some(kp) = kp {
            let c = char::from_u32(k).filter(|c| c.is_ascii_graphic()).unwrap_or('\0');
            return Some((kp, c));
        }
    }
    let letter = |c: u8| -> Key {
        use Key::*;
        [A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z][(c - b'A') as usize]
    };
    Some(match k {
        0x41..=0x5a => (letter(k as u8), (k as u8 + 32) as char),
        0x30 | 0x29 => (Key::Digit0, '0'),
        0x31 | 0x21 => (Key::Digit1, '1'),
        0x32 | 0x40 => (Key::Digit2, '2'),
        0x33 | 0x23 => (Key::Digit3, '3'),
        0x34 | 0x24 => (Key::Digit4, '4'),
        0x35 | 0x25 => (Key::Digit5, '5'),
        0x36 | 0x5e => (Key::Digit6, '6'),
        0x37 | 0x26 => (Key::Digit7, '7'),
        0x38 | 0x2a => (Key::Digit8, '8'),
        0x39 | 0x28 => (Key::Digit9, '9'),
        0x20 => (Key::Space, ' '),
        0x2d | 0x5f => (Key::Minus, '-'),
        0x3d | 0x2b => (Key::Equal, '='),
        0x5b | 0x7b => (Key::BracketLeft, '['),
        0x5d | 0x7d => (Key::BracketRight, ']'),
        0x5c | 0x7c => (Key::Backslash, '\\'),
        0x3b | 0x3a => (Key::Semicolon, ';'),
        0x27 | 0x22 => (Key::Quote, '\''),
        0x2c | 0x3c => (Key::Comma, ','),
        0x2e | 0x3e => (Key::Period, '.'),
        0x2f | 0x3f => (Key::Slash, '/'),
        0x60 | 0x7e => (Key::Backquote, '`'),
        0x0100_0000 => (Key::Escape, '\0'),
        0x0100_0001 | 0x0100_0002 => (Key::Tab, '\0'),
        0x0100_0003 => (Key::Backspace, '\0'),
        0x0100_0004 | 0x0100_0005 => (Key::Enter, '\0'),
        0x0100_0006 => (Key::Insert, '\0'),
        0x0100_0007 => (Key::Delete, '\0'),
        0x0100_0008 => (Key::Pause, '\0'),
        0x0100_0009 => (Key::PrintScreen, '\0'),
        0x0100_0010 => (Key::Home, '\0'),
        0x0100_0011 => (Key::End, '\0'),
        0x0100_0012 => (Key::ArrowLeft, '\0'),
        0x0100_0013 => (Key::ArrowUp, '\0'),
        0x0100_0014 => (Key::ArrowRight, '\0'),
        0x0100_0015 => (Key::ArrowDown, '\0'),
        0x0100_0016 => (Key::PageUp, '\0'),
        0x0100_0017 => (Key::PageDown, '\0'),
        0x0100_0020 => (Key::ShiftLeft, '\0'),
        0x0100_0021 => (Key::ControlLeft, '\0'),
        0x0100_0022 => (Key::MetaLeft, '\0'),
        0x0100_0023 => (Key::AltLeft, '\0'),
        0x0100_1103 => (Key::AltRight, '\0'),
        0x0100_0053 => (Key::MetaLeft, '\0'),
        0x0100_0054 => (Key::MetaRight, '\0'),
        0x0100_0024 => (Key::CapsLock, '\0'),
        0x0100_0025 => (Key::NumLock, '\0'),
        0x0100_0026 => (Key::ScrollLock, '\0'),
        0x0100_0055 => (Key::ContextMenu, '\0'),
        0x0100_0030..=0x0100_0047 => {
            use Key::*;
            let f = [F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24];
            (f[(k - 0x0100_0030) as usize], '\0')
        }
        _ => return None,
    })
}
