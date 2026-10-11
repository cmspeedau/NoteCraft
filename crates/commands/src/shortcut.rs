//! Keyboard shortcuts: modifier sets, key identity, and Windows virtual-key
//! mapping.
//!
//! Everything here is pure data and arithmetic so it can be unit-tested on any
//! host. The virtual-key numbers are the Win32 `VK_*` values; they are kept in
//! this crate because the registry is the single source of truth for what a
//! shortcut *means*, and the native shell only translates a key press into one
//! of these numbers.

use std::fmt;

/// Modifier keys held down while a shortcut key is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    /// Control is held.
    pub ctrl: bool,
    /// Shift is held.
    pub shift: bool,
    /// Alt is held.
    pub alt: bool,
}

impl Modifiers {
    /// No modifiers held.
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
    };

    /// Control only.
    pub const CTRL: Self = Self {
        ctrl: true,
        shift: false,
        alt: false,
    };

    /// Control and Shift.
    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        shift: true,
        alt: false,
    };

    /// Shift only.
    pub const SHIFT: Self = Self {
        ctrl: false,
        shift: true,
        alt: false,
    };

    /// Alt only.
    pub const ALT: Self = Self {
        ctrl: false,
        shift: false,
        alt: true,
    };

    /// Returns these modifiers with Control added.
    pub const fn ctrl(self) -> Self {
        Self { ctrl: true, ..self }
    }

    /// Returns these modifiers with Shift added.
    pub const fn shift(self) -> Self {
        Self {
            shift: true,
            ..self
        }
    }

    /// Returns these modifiers with Alt added.
    pub const fn alt(self) -> Self {
        Self { alt: true, ..self }
    }

    /// Whether any modifier is held.
    pub const fn is_empty(self) -> bool {
        !self.ctrl && !self.shift && !self.alt
    }

    /// The Win32 `ACCEL.fVirt` / `FVIRTKEY|F*` flags for these modifiers.
    ///
    /// `FVIRTKEY` (0x01), `FSHIFT` (0x04), `FCONTROL` (0x08), `FALT` (0x10).
    /// A shortcut whose only modifier is Shift is expressed by the key itself
    /// in some layouts, so callers using this for an accelerator table must
    /// still set [`Modifiers::shift`] explicitly; the shell does.
    pub const fn windows_flags(self) -> u8 {
        let mut flags = 0x01; // FVIRTKEY
        if self.shift {
            flags |= 0x04; // FSHIFT
        }
        if self.ctrl {
            flags |= 0x08; // FCONTROL
        }
        if self.alt {
            flags |= 0x10; // FALT
        }
        flags
    }
}

impl fmt::Display for Modifiers {
    /// Renders the modifier prefix in Windows menu order (`Ctrl+Shift+Alt+`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        Ok(())
    }
}

/// A shortcut key, independent of modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A letter key, stored uppercase.
    Letter(char),
    /// A digit-row key (`0`–`9`).
    Digit(u8),
    /// A function key (`F1`–`F24`).
    Function(u8),
    /// The `+`/`=` key.
    Plus,
    /// The `-`/`_` key.
    Minus,
    /// The `Del` key.
    Delete,
    /// The `Tab` key.
    Tab,
    /// The `Esc` key.
    Escape,
    /// The `/`/`?` key.
    Question,
    /// The `Home` key.
    Home,
    /// The `End` key.
    End,
    /// The `PgUp` key.
    PageUp,
    /// The `PgDn` key.
    PageDown,
    /// The space bar.
    Space,
    /// The `Enter` key.
    Enter,
    /// The `Backspace` key.
    Backspace,
}

impl Key {
    /// The Win32 virtual-key code for this key.
    pub const fn virtual_key(self) -> u16 {
        match self {
            Key::Letter(letter) => {
                // VK_A (0x41) .. VK_Z (0x5A); letters are stored uppercase.
                let upper = letter.to_ascii_uppercase() as u32;
                0x41 + (upper - 'A' as u32) as u16
            }
            Key::Digit(digit) => 0x30 + digit as u16, // VK_0 (0x30)
            Key::Function(number) => 0x6F + number as u16, // VK_F1 (0x70)
            Key::Plus => 0xBB,                        // VK_OEM_PLUS
            Key::Minus => 0xBD,                       // VK_OEM_MINUS
            Key::Delete => 0x2E,                      // VK_DELETE
            Key::Tab => 0x09,                         // VK_TAB
            Key::Escape => 0x1B,                      // VK_ESCAPE
            Key::Question => 0xBF,                    // VK_OEM_2
            Key::Home => 0x24,                        // VK_HOME
            Key::End => 0x23,                         // VK_END
            Key::PageUp => 0x21,                      // VK_PRIOR
            Key::PageDown => 0x22,                    // VK_NEXT
            Key::Space => 0x20,                       // VK_SPACE
            Key::Enter => 0x0D,                       // VK_RETURN
            Key::Backspace => 0x08,                   // VK_BACK
        }
    }

    /// Whether a shift-modified press of this key produces `self`.
    ///
    /// Windows reports `VK_OEM_PLUS` for `=` and `+`, and `VK_OEM_2` for `/`
    /// and `?`; the registry stores the shifted identity, so a lookup for a
    /// Shift-modified press must fall back to the unshifted key before it
    /// fails.
    pub const fn unshifted(self) -> Option<Self> {
        match self {
            Key::Plus => Some(Key::Plus),
            Key::Question => Some(Key::Question),
            other => Some(other),
        }
    }

    /// The label Windows uses for this key in an accelerator string.
    pub const fn label(self) -> &'static str {
        match self {
            Key::Letter(_) => "",
            Key::Digit(_) => "",
            Key::Function(number) => match number {
                1 => "F1",
                2 => "F2",
                3 => "F3",
                4 => "F4",
                5 => "F5",
                6 => "F6",
                7 => "F7",
                8 => "F8",
                9 => "F9",
                10 => "F10",
                11 => "F11",
                12 => "F12",
                _ => "F?",
            },
            Key::Plus => "Plus",
            Key::Minus => "Minus",
            Key::Delete => "Del",
            Key::Tab => "Tab",
            Key::Escape => "Esc",
            Key::Question => "?",
            Key::Home => "Home",
            Key::End => "End",
            Key::PageUp => "PgUp",
            Key::PageDown => "PgDn",
            Key::Space => "Space",
            Key::Enter => "Enter",
            Key::Backspace => "Backspace",
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Letter(letter) => write!(f, "{}", letter.to_ascii_uppercase()),
            Key::Digit(digit) => write!(f, "{digit}"),
            other => f.write_str(other.label()),
        }
    }
}

/// A key plus the modifiers that must be held for it to fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    /// Modifiers that must be held.
    pub modifiers: Modifiers,
    /// The key that must be pressed.
    pub key: Key,
}

impl Shortcut {
    /// Builds a shortcut.
    pub const fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { modifiers, key }
    }
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.modifiers, self.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_keys_match_win32_constants() {
        assert_eq!(Key::Letter('n').virtual_key(), 0x4E);
        assert_eq!(Key::Letter('z').virtual_key(), 0x5A);
        assert_eq!(Key::Digit(0).virtual_key(), 0x30);
        assert_eq!(Key::Digit(9).virtual_key(), 0x39);
        assert_eq!(Key::Function(1).virtual_key(), 0x70);
        assert_eq!(Key::Function(12).virtual_key(), 0x7B);
        assert_eq!(Key::Plus.virtual_key(), 0xBB);
        assert_eq!(Key::Minus.virtual_key(), 0xBD);
        assert_eq!(Key::Escape.virtual_key(), 0x1B);
        assert_eq!(Key::Tab.virtual_key(), 0x09);
    }

    #[test]
    fn shortcut_labels_match_the_reference_matrix() {
        let cases = [
            (Shortcut::new(Modifiers::CTRL, Key::Letter('n')), "Ctrl+N"),
            (
                Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('s')),
                "Ctrl+Shift+S",
            ),
            (Shortcut::new(Modifiers::CTRL, Key::Letter('w')), "Ctrl+W"),
            (Shortcut::new(Modifiers::ALT, Key::Function(4)), "Alt+F4"),
            (
                Shortcut::new(Modifiers::SHIFT, Key::Function(3)),
                "Shift+F3",
            ),
            (Shortcut::new(Modifiers::CTRL, Key::Tab), "Ctrl+Tab"),
            (
                Shortcut::new(Modifiers::CTRL_SHIFT, Key::Tab),
                "Ctrl+Shift+Tab",
            ),
            (Shortcut::new(Modifiers::CTRL, Key::Plus), "Ctrl+Plus"),
            (Shortcut::new(Modifiers::CTRL, Key::Minus), "Ctrl+Minus"),
            (Shortcut::new(Modifiers::CTRL, Key::Digit(0)), "Ctrl+0"),
        ];
        for (shortcut, expected) in cases {
            assert_eq!(shortcut.to_string(), expected, "shortcut label");
        }
    }

    #[test]
    fn windows_flags_encode_modifiers() {
        assert_eq!(Modifiers::NONE.windows_flags(), 0x01);
        assert_eq!(Modifiers::CTRL.windows_flags(), 0x09);
        assert_eq!(Modifiers::CTRL_SHIFT.windows_flags(), 0x0D);
        assert_eq!(Modifiers::ALT.windows_flags(), 0x11);
    }
}
