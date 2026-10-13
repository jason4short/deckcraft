//! The Mac-style object shortcuts (`menus::mac_shortcut`) through real key events.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use deckcraft_model::{ShapeId, ShapeKind};
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use serde_json::json;

use crate::{Services, SlideApp};

const CMD: Modifiers = Modifiers::COMMAND;

fn harness() -> (Harness<'static, SlideApp>, Vec<ShapeId>) {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    let ids: Vec<ShapeId> = (0..3)
        .map(|i| {
            let r = app.run("shape.insert", json!({"preset": "rect", "rect": [100 + i * 120, 100, 100, 60]})).unwrap();
            ShapeId(r["id"].as_u64().unwrap() as u32)
        })
        .collect();
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    (h, ids)
}

fn select(h: &mut Harness<'static, SlideApp>, ids: &[ShapeId]) {
    h.state_mut().run("selection.select", json!({"ids": ids})).unwrap();
    h.run_steps(2);
}

fn press(h: &mut Harness<'static, SlideApp>, key: Key, modifiers: Modifiers) {
    h.event(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
    h.event(egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers });
    h.run_steps(2);
}

fn order(h: &Harness<'static, SlideApp>) -> Vec<ShapeId> {
    h.state().session.active().unwrap().current_slide().unwrap().shapes.iter().map(|s| s.id).collect()
}

fn get(h: &Harness<'static, SlideApp>, id: ShapeId) -> deckcraft_model::Shape {
    h.state().session.active().unwrap().shape(id).cloned().unwrap()
}

#[test]
fn command_g_groups_and_shift_command_g_ungroups() {
    let (mut h, ids) = harness();
    select(&mut h, &ids[..2]);
    let n = order(&h).len();
    press(&mut h, Key::G, CMD);
    assert_eq!(order(&h).len(), n - 1);
    assert!(matches!(h.state().session.active().unwrap().selected_shapes()[0].kind, ShapeKind::Group { .. }));
    press(&mut h, Key::G, CMD | Modifiers::SHIFT);
    assert_eq!(order(&h).len(), n);
}

#[test]
fn option_command_brackets_bring_to_front_and_send_to_back() {
    let (mut h, ids) = harness();
    select(&mut h, &ids[..1]);
    press(&mut h, Key::CloseBracket, CMD | Modifiers::ALT);
    assert_eq!(order(&h).last(), Some(&ids[0]));
    press(&mut h, Key::OpenBracket, CMD | Modifiers::ALT);
    assert_eq!(order(&h).first(), Some(&ids[0]));
}

#[test]
fn shift_command_l_locks_and_shift_command_h_hides_the_whole_selection() {
    let (mut h, ids) = harness();
    h.state_mut().run("shape.lock", json!({"ids": [ids[0]], "locked": true})).unwrap();
    select(&mut h, &ids[..2]);
    press(&mut h, Key::L, CMD | Modifiers::SHIFT);
    assert!(get(&h, ids[0]).locked && get(&h, ids[1]).locked, "one unlocked: all lock");
    press(&mut h, Key::L, CMD | Modifiers::SHIFT);
    assert!(!get(&h, ids[0]).locked && !get(&h, ids[1]).locked, "all locked: all unlock");
    press(&mut h, Key::H, CMD | Modifiers::SHIFT);
    assert!(get(&h, ids[0]).hidden && get(&h, ids[1]).hidden);
    press(&mut h, Key::H, CMD | Modifiers::SHIFT);
    assert!(!get(&h, ids[0]).hidden);
}

#[test]
fn shift_command_z_redoes() {
    let (mut h, ids) = harness();
    select(&mut h, &ids[..1]);
    press(&mut h, Key::D, CMD);
    let n = order(&h).len();
    press(&mut h, Key::Z, CMD);
    assert_eq!(order(&h).len(), n - 1, "⌘Z undoes the duplicate");
    press(&mut h, Key::Z, CMD | Modifiers::SHIFT);
    assert_eq!(order(&h).len(), n, "⇧⌘Z redoes it");
}
