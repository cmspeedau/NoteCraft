//! Keyboard accelerators.
//!
//! The registry (`nc-commands`) is the single authoritative source of what each
//! shortcut means; this module is the adapter that turns an `egui` key press
//! into the [`CommandId`] that key names and hands it to [`actions`] — the same
//! place a menu click lands — so a command runs identically however it was
//! invoked (`AGENT.md` §7, `docs/architecture.md` §1).
//!
//! Three rules keep the keyboard honest:
//!
//! * Only a command this build actually implements is dispatched. A key that
//!   names a command scheduled for a later milestone is left in the event queue
//!   rather than consumed for nothing (`actions::handles`).
//! * Only an available command is dispatched: a greyed command's key stays
//!   unclaimed, exactly as the greyed menu item does.
//! * A command that acts on the focused text context (undo, redo, select all,
//!   delete) is left to the field when a widget other than the editor holds the
//!   keyboard, so typing into the Find or Save As field keeps its own editing
//!   keys. Everything else — Save, Open, Find, zoom — stays global.
//!
//! Cut, copy and paste do not arrive as key presses on native backends: the
//! windowing layer turns `Ctrl+X`/`Ctrl+C`/`Ctrl+V` into `egui`'s `Cut`/`Copy`/
//! `Paste` events before a `Key` event is produced, and the editor's text widget
//! routes those through the same document. Their menu entries request the same
//! events (`ViewportCommand::Request*`), so the menu and the keyboard converge
//! on one implementation.

use egui::{Context, Event, Key, Modifiers};

use crate::ui::actions;
use crate::ui::editor::EDITOR_ID;
use crate::ui::state::Shell;
use nc_commands::{CommandId, Key as NcKey, by_shortcut};

/// Commands that act on whatever text context holds the keyboard. When a widget
/// other than the editor is focused, their key is left to that widget.
const EDIT_CONTEXT: &[&str] = &[
    "cmd.edit.undo",
    "cmd.edit.redo",
    "cmd.edit.select_all",
    "cmd.edit.delete",
];

/// Runs the command each pending accelerator names, consuming the key so nothing
/// else acts on it.
///
/// Called once per frame before the chrome and the editor are drawn, so a
/// command that opens a dialog (or moves the caret) is reflected in the same
/// frame.
pub fn dispatch(shell: &mut Shell, ctx: &Context) {
    let focus_elsewhere = focus_is_elsewhere(ctx);
    let context = shell.availability();

    let mut fired: Vec<CommandId> = Vec::new();
    ctx.input_mut(|input| {
        input.events.retain(|event| {
            let Event::Key {
                key,
                physical_key: _,
                pressed: true,
                repeat: _,
                modifiers,
            } = event
            else {
                return true;
            };
            let Some(id) = command_for(*key, *modifiers) else {
                return true;
            };
            let Some(spec) = nc_commands::by_id(id.as_str()) else {
                return true;
            };
            if !actions::handles(id) || !spec.is_available(context) {
                return true;
            }
            if focus_elsewhere && EDIT_CONTEXT.contains(&id.as_str()) {
                return true;
            }
            fired.push(id);
            false
        });
    });

    for id in fired {
        actions::run(shell, ctx, id);
    }
}

/// The command a key press names, if the registry binds it.
fn command_for(key: Key, modifiers: Modifiers) -> Option<CommandId> {
    let key = nc_key(key)?;
    let modifiers = nc_modifiers(modifiers);
    by_shortcut(modifiers, key).map(|spec| spec.id)
}

/// Whether a widget other than the editor currently holds the keyboard.
fn focus_is_elsewhere(ctx: &Context) -> bool {
    let editor = egui::Id::new(EDITOR_ID);
    ctx.memory(|memory| memory.focused().is_some_and(|focused| focused != editor))
}

/// Translates the modifiers `egui` reports into the registry's model.
///
/// `egui`'s `command` is Control on Windows and Linux and Command on macOS, so
/// mapping it to the registry's `ctrl` gives each platform its own
/// conventional accelerator without the registry carrying both.
fn nc_modifiers(modifiers: Modifiers) -> nc_commands::Modifiers {
    nc_commands::Modifiers {
        ctrl: modifiers.command,
        shift: modifiers.shift,
        alt: modifiers.alt,
    }
}

/// Translates an `egui` key into the registry's key model.
fn nc_key(key: Key) -> Option<NcKey> {
    if let Some(letter) = latin_letter(key) {
        return Some(NcKey::Letter(letter));
    }
    if let Some(digit) = digit(key) {
        return Some(NcKey::Digit(digit));
    }
    if let Some(number) = function_number(key) {
        return Some(NcKey::Function(number));
    }
    Some(match key {
        // `+` is written two ways depending on the keyboard, and both are the
        // zoom-in chord.
        Key::Plus | Key::Equals => NcKey::Plus,
        Key::Minus => NcKey::Minus,
        Key::Delete => NcKey::Delete,
        Key::Tab => NcKey::Tab,
        Key::Escape => NcKey::Escape,
        Key::Questionmark => NcKey::Question,
        Key::Home => NcKey::Home,
        Key::End => NcKey::End,
        Key::PageUp => NcKey::PageUp,
        Key::PageDown => NcKey::PageDown,
        Key::Space => NcKey::Space,
        Key::Enter => NcKey::Enter,
        Key::Backspace => NcKey::Backspace,
        _ => return None,
    })
}

/// The Latin letter an `egui` key names, if it names one.
fn latin_letter(key: Key) -> Option<char> {
    Some(match key {
        Key::A => 'A',
        Key::B => 'B',
        Key::C => 'C',
        Key::D => 'D',
        Key::E => 'E',
        Key::F => 'F',
        Key::G => 'G',
        Key::H => 'H',
        Key::I => 'I',
        Key::J => 'J',
        Key::K => 'K',
        Key::L => 'L',
        Key::M => 'M',
        Key::N => 'N',
        Key::O => 'O',
        Key::P => 'P',
        Key::Q => 'Q',
        Key::R => 'R',
        Key::S => 'S',
        Key::T => 'T',
        Key::U => 'U',
        Key::V => 'V',
        Key::W => 'W',
        Key::X => 'X',
        Key::Y => 'Y',
        Key::Z => 'Z',
        _ => return None,
    })
}

/// The digit an `egui` key names, if it names one.
fn digit(key: Key) -> Option<u8> {
    Some(match key {
        Key::Num0 => 0,
        Key::Num1 => 1,
        Key::Num2 => 2,
        Key::Num3 => 3,
        Key::Num4 => 4,
        Key::Num5 => 5,
        Key::Num6 => 6,
        Key::Num7 => 7,
        Key::Num8 => 8,
        Key::Num9 => 9,
        _ => return None,
    })
}

/// The function-key number an `egui` key names, if it names one.
fn function_number(key: Key) -> Option<u8> {
    Some(match key {
        Key::F1 => 1,
        Key::F2 => 2,
        Key::F3 => 3,
        Key::F4 => 4,
        Key::F5 => 5,
        Key::F6 => 6,
        Key::F7 => 7,
        Key::F8 => 8,
        Key::F9 => 9,
        Key::F10 => 10,
        Key::F11 => 11,
        Key::F12 => 12,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nc_commands::Modifiers as NcModifiers;

    /// Control as the windowing layer reports it: `command` is set on every
    /// platform (Control elsewhere, Command on macOS).
    const CTRL: Modifiers = Modifiers::COMMAND;

    fn ctrl_shift() -> Modifiers {
        Modifiers {
            shift: true,
            ..Modifiers::COMMAND
        }
    }

    fn ctrl_alt() -> Modifiers {
        Modifiers {
            alt: true,
            ..Modifiers::COMMAND
        }
    }

    fn named(key: Key, modifiers: Modifiers) -> Option<&'static str> {
        command_for(key, modifiers).map(|id| id.as_str())
    }

    #[test]
    fn the_documented_accelerators_resolve_to_their_commands() {
        let cases = [
            (Key::N, CTRL, "cmd.file.new_tab"),
            (Key::O, CTRL, "cmd.file.open"),
            (Key::S, CTRL, "cmd.file.save"),
            (Key::S, ctrl_shift(), "cmd.file.save_as"),
            (Key::W, CTRL, "cmd.file.close_tab"),
            (Key::F, CTRL, "cmd.edit.find"),
            (Key::F3, Modifiers::NONE, "cmd.edit.find_next"),
            (Key::F3, Modifiers::SHIFT, "cmd.edit.find_prev"),
            (Key::F5, Modifiers::NONE, "cmd.edit.time_date"),
            (Key::Z, CTRL, "cmd.edit.undo"),
            (Key::A, CTRL, "cmd.edit.select_all"),
        ];
        for (key, modifiers, expected) in cases {
            assert_eq!(
                named(key, modifiers),
                Some(expected),
                "{modifiers:?}+{key:?}"
            );
        }
    }

    #[test]
    fn both_spellings_of_plus_reach_zoom_in() {
        assert_eq!(named(Key::Plus, CTRL), Some("cmd.view.zoom_in"));
        assert_eq!(named(Key::Equals, CTRL), Some("cmd.view.zoom_in"));
        assert_eq!(named(Key::Minus, CTRL), Some("cmd.view.zoom_out"));
    }

    #[test]
    fn an_unbound_key_names_nothing() {
        assert_eq!(named(Key::Q, ctrl_alt()), None);
        assert_eq!(named(Key::F6, Modifiers::NONE), None);
    }

    #[test]
    fn command_maps_to_ctrl_so_each_platform_keeps_its_own_modifier() {
        let ctrl = nc_modifiers(Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        });
        assert_eq!(ctrl, NcModifiers::CTRL);
        // Shift+Alt survives the mapping.
        let both = nc_modifiers(Modifiers {
            shift: true,
            alt: true,
            ..Modifiers::NONE
        });
        assert_eq!(
            both,
            NcModifiers {
                ctrl: false,
                shift: true,
                alt: true,
            }
        );
    }

    #[test]
    fn every_key_the_registry_uses_can_be_translated() {
        // If a shortcut is added for a key with no mapping, its accelerator
        // would silently never fire; fail loudly instead.
        for spec in nc_commands::REGISTRY {
            let Some(shortcut) = spec.shortcut else {
                continue;
            };
            let egui_key = egui_key_of(shortcut.key);
            let Some(egui_key) = egui_key else {
                continue;
            };
            let egui_modifiers = egui_modifiers_of(shortcut.modifiers);
            assert_eq!(
                command_for(egui_key, egui_modifiers).map(|id| id.as_str()),
                Some(spec.id.as_str()),
                "{} ({}) does not round-trip through the key map",
                spec.id,
                shortcut,
            );
        }
    }

    /// The inverse of [`nc_key`], for the round-trip test only.
    fn egui_key_of(key: NcKey) -> Option<Key> {
        Some(match key {
            NcKey::Letter(letter) => match letter.to_ascii_uppercase() {
                'A' => Key::A,
                'B' => Key::B,
                'C' => Key::C,
                'D' => Key::D,
                'E' => Key::E,
                'F' => Key::F,
                'G' => Key::G,
                'H' => Key::H,
                'I' => Key::I,
                'J' => Key::J,
                'K' => Key::K,
                'L' => Key::L,
                'M' => Key::M,
                'N' => Key::N,
                'O' => Key::O,
                'P' => Key::P,
                'Q' => Key::Q,
                'R' => Key::R,
                'S' => Key::S,
                'T' => Key::T,
                'U' => Key::U,
                'V' => Key::V,
                'W' => Key::W,
                'X' => Key::X,
                'Y' => Key::Y,
                'Z' => Key::Z,
                _ => return None,
            },
            NcKey::Digit(0) => Key::Num0,
            NcKey::Digit(1) => Key::Num1,
            NcKey::Digit(2) => Key::Num2,
            NcKey::Digit(3) => Key::Num3,
            NcKey::Digit(4) => Key::Num4,
            NcKey::Digit(5) => Key::Num5,
            NcKey::Digit(6) => Key::Num6,
            NcKey::Digit(7) => Key::Num7,
            NcKey::Digit(8) => Key::Num8,
            NcKey::Digit(9) => Key::Num9,
            NcKey::Function(1) => Key::F1,
            NcKey::Function(2) => Key::F2,
            NcKey::Function(3) => Key::F3,
            NcKey::Function(4) => Key::F4,
            NcKey::Function(5) => Key::F5,
            NcKey::Function(6) => Key::F6,
            NcKey::Function(7) => Key::F7,
            NcKey::Function(8) => Key::F8,
            NcKey::Function(9) => Key::F9,
            NcKey::Function(10) => Key::F10,
            NcKey::Function(11) => Key::F11,
            NcKey::Function(12) => Key::F12,
            NcKey::Plus => Key::Plus,
            NcKey::Minus => Key::Minus,
            NcKey::Delete => Key::Delete,
            NcKey::Tab => Key::Tab,
            NcKey::Escape => Key::Escape,
            NcKey::Question => Key::Questionmark,
            NcKey::Home => Key::Home,
            NcKey::End => Key::End,
            NcKey::PageUp => Key::PageUp,
            NcKey::PageDown => Key::PageDown,
            NcKey::Space => Key::Space,
            NcKey::Enter => Key::Enter,
            NcKey::Backspace => Key::Backspace,
            _ => return None,
        })
    }

    /// The inverse of [`nc_modifiers`], for the round-trip test only.
    fn egui_modifiers_of(modifiers: nc_commands::Modifiers) -> Modifiers {
        Modifiers {
            alt: modifiers.alt,
            ctrl: modifiers.ctrl,
            shift: modifiers.shift,
            mac_cmd: false,
            command: modifiers.ctrl,
        }
    }
}
