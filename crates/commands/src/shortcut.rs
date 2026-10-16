//! Keyboard shortcuts: modifier sets, key identity, and their display labels.
//!
//! Everything here is pure data and arithmetic so it can be unit-tested on any
//! host. The registry is the single source of truth for what a shortcut *means*;
//! the shell maps a key press to one of these and the menu renders the label.

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

    /// Control and Alt.
    pub const CTRL_ALT: Self = Self {
        ctrl: true,
        shift: false,
        alt: true,
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

    /// Whether any modifier is held.
    pub const fn is_empty(self) -> bool {
        !self.ctrl && !self.shift && !self.alt
    }
}

impl fmt::Display for Modifiers {
    /// Renders the modifier prefix (`Ctrl+Shift+Alt+`).
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
    /// The label shown for this key in an accelerator string.
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
    fn shortcut_labels_match_the_menus() {
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
            (Shortcut::new(Modifiers::CTRL, Key::Plus), "Ctrl+Plus"),
            (Shortcut::new(Modifiers::CTRL, Key::Minus), "Ctrl+Minus"),
            (Shortcut::new(Modifiers::CTRL, Key::Digit(0)), "Ctrl+0"),
            (
                Shortcut::new(Modifiers::CTRL_ALT, Key::Letter('s')),
                "Ctrl+Alt+S",
            ),
        ];
        for (shortcut, expected) in cases {
            assert_eq!(shortcut.to_string(), expected, "shortcut label");
        }
    }

    #[test]
    fn modifiers_report_emptiness() {
        assert!(Modifiers::NONE.is_empty());
        assert!(!Modifiers::CTRL.is_empty());
    }
}
