//! Windows key names and held modifiers.
#[cfg(windows)]
use exact_linux::Presenter;
#[cfg(windows)]
use exact_runner::DataSource;
use exact_runner::KeyModifiers;

#[cfg(windows)]
pub(super) fn key_names(
    physical: winit::keyboard::PhysicalKey,
    logical: &winit::keyboard::Key,
    text: Option<&str>,
) -> (String, String) {
    use winit::keyboard::{Key, PhysicalKey};
    let code = match physical {
        PhysicalKey::Code(code) => format!("{code:?}").replace("Super", "Meta"),
        PhysicalKey::Unidentified(_) => "Unidentified".into(),
    };
    let key = match logical {
        Key::Character(s) => s.to_string(),
        Key::Named(k) => format!("{k:?}"),
        _ => text.unwrap_or("Unidentified").to_owned(),
    };
    (code, key)
}

/// Outside ModifiersChanged, only clear releases consumed by a menu or focus change.
#[cfg(windows)]
#[allow(unsafe_code)]
pub(super) fn sync<D: DataSource>(
    presenter: &mut Presenter<D>,
    modifiers: Option<winit::keyboard::ModifiersState>,
) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    // GetKeyState follows this thread's input queue.
    let pressed = [
        [VK_LSHIFT, VK_RSHIFT],
        [VK_LCONTROL, VK_RCONTROL],
        [VK_LMENU, VK_RMENU],
        [VK_LWIN, VK_RWIN],
    ]
    .map(|pair| pair.map(|key| unsafe { GetKeyState(i32::from(key)) } < 0));
    reconcile(
        modifiers.map(|state| KeyModifiers {
            shift: state.shift_key(),
            ctrl: state.control_key(),
            alt: state.alt_key(),
            meta: state.super_key(),
        }),
        pressed,
        |code, down| presenter.hold_modifier(code, down),
    );
}

fn reconcile(
    modifiers: Option<KeyModifiers>,
    pressed: [[bool; 2]; 4],
    mut hold: impl FnMut(&str, bool),
) {
    let active = modifiers.unwrap_or_default();
    for ((codes, active), sides) in [
        (["ShiftLeft", "ShiftRight"], active.shift),
        (["ControlLeft", "ControlRight"], active.ctrl),
        (["AltLeft", "AltRight"], active.alt),
        (["MetaLeft", "MetaRight"], active.meta),
    ]
    .into_iter()
    .zip(pressed)
    {
        for (code, down) in codes.into_iter().zip(sides) {
            // winit filters AltGr's synthetic Control and Alt. Raw key state
            // identifies a side, but cannot introduce a modifier it filtered.
            if modifiers.is_some() {
                hold(code, active && down);
            } else if !down {
                hold(code, false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exact_linux::presenter::{clipboard::Clipboard, PainterChoice};
    use exact_linux::Presenter;
    use std::{cell::RefCell, rc::Rc};

    struct Memory(Rc<RefCell<String>>);
    impl Clipboard for Memory {
        fn read_text(&mut self) -> Result<Option<String>, String> {
            Ok(Some(self.0.borrow().clone()))
        }
        fn write_text(&mut self, text: &str) -> Result<(), String> {
            *self.0.borrow_mut() = text.into();
            Ok(())
        }
    }

    #[test]
    fn altgr_types_text_and_leaves_no_control_chord() {
        let plan = contract::compile(
            "component App\n  view\n    input testId=\"field\" value=\"name\" width=200 height=40\n",
        )
        .unwrap()
        .encode();
        let (mut p, warning) = Presenter::boot_with(
            &plan,
            (),
            (240., 80.),
            1.,
            Default::default(),
            PainterChoice::Cpu,
        )
        .unwrap();
        assert!(warning.is_none());
        let clipboard = Rc::new(RefCell::new("untouched".to_string()));
        p.set_clipboard(Box::new(Memory(clipboard.clone())));
        let k = p.host().kernel();
        let field = k.node_by_key(k.find_by_test_id("field")[0]).unwrap().id;
        p.tap(field).unwrap();
        p.hardware_key("End", "End", true, false);
        p.hardware_key("End", "End", false, false);

        // AltGr's synthetic Control press can first report Control held.
        reconcile(
            Some(KeyModifiers {
                ctrl: true,
                ..Default::default()
            }),
            [[false; 2], [true, false], [false; 2], [false; 2]],
            |code, down| p.hold_modifier(code, down),
        );
        // winit filters Control and Alt while right Alt is down.
        reconcile(
            Some(KeyModifiers::default()),
            [[false; 2], [true, false], [false, true], [false; 2]],
            |code, down| p.hold_modifier(code, down),
        );
        p.hardware_key("AltRight", "AltGraph", true, false);
        p.hardware_key("KeyQ", "@", true, false);
        p.hardware_key("KeyQ", "@", false, false);
        p.hardware_key("AltRight", "AltGraph", false, false);
        // No ModifiersChanged or Control release follows AltGr's release.
        for ch in "example.com".chars() {
            let code = if ch == '.' {
                "Period".into()
            } else {
                format!("Key{}", ch.to_ascii_uppercase())
            };
            let key = ch.to_string();
            p.hardware_key(&code, &key, true, false);
            p.hardware_key(&code, &key, false, false);
        }
        let value = p
            .access_tree()
            .0
            .into_iter()
            .find(|n| n.view == field)
            .unwrap()
            .value;
        assert_eq!(
            (value.as_str(), clipboard.borrow().as_str()),
            ("name@example.com", "untouched")
        );

        for side in 0..2 {
            let mut pressed = [[false; 2]; 4];
            pressed[1][side] = true;
            reconcile(
                Some(KeyModifiers {
                    ctrl: true,
                    ..Default::default()
                }),
                pressed,
                |code, down| p.hold_modifier(code, down),
            );
            for (code, key) in [("KeyA", "a"), ("KeyC", "c")] {
                p.hardware_key(code, key, true, false);
                p.hardware_key(code, key, false, false);
            }
            assert_eq!(clipboard.borrow().as_str(), "name@example.com");
            *clipboard.borrow_mut() = "untouched".into();
            // A native menu may consume the Control release.
            reconcile(None, [[false; 2]; 4], |code, down| {
                p.hold_modifier(code, down)
            });
            p.hardware_key("End", "End", true, false);
            p.hardware_key("End", "End", false, false);
        }
        p.hardware_key("KeyX", "x", true, false);
        p.hardware_key("KeyX", "x", false, false);
        let value = p
            .access_tree()
            .0
            .into_iter()
            .find(|n| n.view == field)
            .unwrap()
            .value;
        assert_eq!(value, "name@example.comx");
    }

    #[test]
    fn modifiers_keep_their_sides_and_recovery_only_releases() {
        let codes = [
            "ShiftLeft",
            "ShiftRight",
            "ControlLeft",
            "ControlRight",
            "AltLeft",
            "AltRight",
            "MetaLeft",
            "MetaRight",
        ];
        for (group, name) in ["Shift", "Control", "Alt", "Meta"].into_iter().enumerate() {
            for sides in [[true, false], [false, true], [true, true], [false, false]] {
                let mut pressed = [[true; 2]; 4];
                pressed[group] = sides;
                let mut held = Vec::new();
                reconcile(KeyModifiers::held(name), pressed, |code, down| {
                    held.push((code.to_owned(), down));
                });
                assert_eq!(held.len(), codes.len());
                for (index, (code, down)) in held.into_iter().enumerate() {
                    assert_eq!(code, codes[index]);
                    assert_eq!(
                        down,
                        index / 2 == group && sides[index % 2],
                        "{name} {code}"
                    );
                }
            }
        }
        let mut released = Vec::new();
        reconcile(
            None,
            [[false; 2], [true, false], [false, true], [false; 2]],
            |code, down| {
                assert!(!down, "raw state must not restore {code}");
                released.push(code.to_owned());
            },
        );
        assert_eq!(
            released,
            [
                "ShiftLeft",
                "ShiftRight",
                "ControlRight",
                "AltLeft",
                "MetaLeft",
                "MetaRight"
            ]
        );
    }
}
