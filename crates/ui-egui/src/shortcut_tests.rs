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

// ----- The slide list --------------------------------------------------------------------

fn slides(h: &Harness<'static, SlideApp>) -> Vec<u32> {
    h.state().session.active().unwrap().doc.slides.iter().map(|s| s.id.0).collect()
}

/// Four slides, the first current, and the slide list holding the keyboard.
fn slide_list() -> Harness<'static, SlideApp> {
    let (mut h, _) = harness();
    for _ in 0..3 {
        h.state_mut().run("slide.new", json!({})).unwrap();
    }
    h.state_mut().run("slide.go", json!({"index": 0})).unwrap();
    h.state_mut().slides_focus = true;
    h.run_steps(2);
    h
}

#[test]
fn a_click_on_a_thumbnail_gives_the_slide_list_the_keyboard_and_one_elsewhere_takes_it() {
    let (mut h, _) = harness();
    let r = h.state().thumbs_rect.expect("the slide list is shown");
    let click = |h: &mut Harness<'static, SlideApp>, p: egui::Pos2| {
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
            h.run_steps(1);
        }
        h.run_steps(1);
    };
    click(&mut h, r.min + egui::vec2(r.width() / 2.0, 60.0));
    assert!(h.state().slides_focus, "the thumbnail took the keyboard");
    let slide = h.state().slide_rect.unwrap();
    click(&mut h, slide.left_bottom() + egui::vec2(10.0, -10.0));
    assert!(!h.state().slides_focus, "the slide took it back");
}

#[test]
fn slide_list_keys_duplicate_delete_and_hide_slides() {
    let mut h = slide_list();
    press(&mut h, Key::D, CMD);
    assert_eq!(slides(&h).len(), 5, "⌘D duplicates the slide");
    press(&mut h, Key::Delete, Modifiers::NONE);
    assert_eq!(slides(&h).len(), 4, "Delete removes it");
    press(&mut h, Key::H, CMD | Modifiers::SHIFT);
    let d = h.state().session.active().unwrap();
    assert!(d.doc.slides[d.selection.slide].hidden, "⇧⌘H hides it");
}

#[test]
fn slide_list_keys_extend_the_selection_and_move_slides() {
    let mut h = slide_list();
    let before = slides(&h);
    press(&mut h, Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(h.state().session.active().unwrap().selection.slides.len(), 2, "⇧↓ selects the next slide too");
    press(&mut h, Key::ArrowDown, CMD);
    assert_eq!(slides(&h), vec![before[2], before[0], before[1], before[3]], "⌘↓ moves both down one place");
    press(&mut h, Key::ArrowDown, CMD | Modifiers::SHIFT);
    assert_eq!(slides(&h), vec![before[2], before[3], before[0], before[1]], "⇧⌘↓ to the end, in order");
    assert_eq!(h.state().session.active().unwrap().selection.slides.len(), 2, "still selected");
}

#[test]
fn delete_on_an_empty_selection_leaves_the_slide_alone() {
    let mut h = slide_list();
    h.state_mut().slides_focus = false;
    press(&mut h, Key::Delete, Modifiers::NONE);
    press(&mut h, Key::D, CMD);
    assert_eq!(slides(&h).len(), 4, "the canvas has nothing selected to delete or duplicate");
}

#[test]
fn moving_slides_keeps_their_order() {
    use crate::thumbs::moves;
    assert_eq!(moves(&[1, 2], 5, false, false), vec![(1, 0), (2, 1)]);
    assert_eq!(moves(&[0, 2], 5, false, false), vec![(2, 1)], "the first can't move up; the next closes up");
    assert_eq!(moves(&[1, 3], 5, true, true), vec![(3, 4), (1, 3)]);
    assert_eq!(moves(&[2], 3, true, false), vec![], "already last");
}
