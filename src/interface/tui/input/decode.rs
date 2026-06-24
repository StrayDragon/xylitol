//! Input layer 1: decode crossterm events into normalized InputKey.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};

/// Normalized input key, independent of terminal quirks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputKey {
    pub(crate) code: KeyCode,
    pub(crate) modifiers: KeyModifiers,
}

/// Decoded user action from a crossterm event.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum DecodedInput {
    /// A key press.
    Key(InputKey),
    /// Mouse button press.
    MousePress {
        button: MouseButton,
        row: u16,
        col: u16,
    },
    /// Mouse drag.
    MouseDrag { row: u16, col: u16 },
    /// Mouse button release.
    MouseRelease { row: u16, col: u16 },
    /// Mouse scroll.
    MouseScroll { up: bool, row: u16, col: u16 },
    /// Terminal resize.
    Resize { cols: u16, rows: u16 },
    /// Unknown / unhandled.
    Unknown,
}

/// Decode a crossterm event into a DecodedInput.
pub(crate) fn decode(event: Event) -> DecodedInput {
    match event {
        Event::Key(KeyEvent {
            code, modifiers, ..
        }) => DecodedInput::Key(InputKey { code, modifiers }),
        Event::Mouse(mouse_event) => match mouse_event.kind {
            MouseEventKind::Down(button) => DecodedInput::MousePress {
                button,
                row: mouse_event.row,
                col: mouse_event.column,
            },
            MouseEventKind::Drag(button) => DecodedInput::MouseDrag {
                row: mouse_event.row,
                col: mouse_event.column,
            },
            MouseEventKind::Up(button) => DecodedInput::MouseRelease {
                row: mouse_event.row,
                col: mouse_event.column,
            },
            MouseEventKind::ScrollDown => DecodedInput::MouseScroll {
                up: false,
                row: mouse_event.row,
                col: mouse_event.column,
            },
            MouseEventKind::ScrollUp => DecodedInput::MouseScroll {
                up: true,
                row: mouse_event.row,
                col: mouse_event.column,
            },
            _ => DecodedInput::Unknown,
        },
        Event::Resize(cols, rows) => DecodedInput::Resize { cols, rows },
        _ => DecodedInput::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_decode_key_press() {
        let event = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        match decode(event) {
            DecodedInput::Key(key) => {
                assert_eq!(key.code, KeyCode::Char('a'));
                assert_eq!(key.modifiers, KeyModifiers::NONE);
            }
            _ => panic!("expected Key"),
        }
    }

    #[test]
    fn test_decode_ctrl_c() {
        let event = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        match decode(event) {
            DecodedInput::Key(key) => {
                assert_eq!(key.code, KeyCode::Char('c'));
                assert_eq!(key.modifiers, KeyModifiers::CONTROL);
            }
            _ => panic!("expected Key"),
        }
    }

    #[test]
    fn test_decode_enter() {
        let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        match decode(event) {
            DecodedInput::Key(key) => {
                assert_eq!(key.code, KeyCode::Enter);
            }
            _ => panic!("expected Key"),
        }
    }

    #[test]
    fn test_decode_tab() {
        let event = Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        match decode(event) {
            DecodedInput::Key(key) => {
                assert_eq!(key.code, KeyCode::Tab);
            }
            _ => panic!("expected Key"),
        }
    }

    #[test]
    fn test_decode_escape() {
        let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        match decode(event) {
            DecodedInput::Key(key) => {
                assert_eq!(key.code, KeyCode::Esc);
            }
            _ => panic!("expected Key"),
        }
    }

    #[test]
    fn test_decode_resize() {
        let event = Event::Resize(120, 40);
        match decode(event) {
            DecodedInput::Resize { cols, rows } => {
                assert_eq!(cols, 120);
                assert_eq!(rows, 40);
            }
            _ => panic!("expected Resize"),
        }
    }
}
