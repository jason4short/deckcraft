//! ⌘V when the system clipboard holds no text: egui-winit then drops the press and only the V
//! key's release arrives (see `PasteKey`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use egui_kittest::Harness;
use serde_json::json;

use crate::{Services, SlideApp};

fn png() -> Vec<u8> {
    deckcraft_render::Image { width: 4, height: 4, pixels: vec![200; 64] }.to_png()
}

fn harness(services: Services) -> Harness<'static, SlideApp> {
    let app = SlideApp::new(Session::with_new(), services);
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    h
}

fn v(h: &mut Harness<'static, SlideApp>, pressed: bool, modifiers: egui::Modifiers) {
    h.event(egui::Event::Key { key: egui::Key::V, physical_key: None, pressed, repeat: false, modifiers });
    h.run_steps(2);
}

fn shapes(h: &Harness<'static, SlideApp>) -> usize {
    let d = h.state().session.active().unwrap();
    d.current_slide().unwrap().shapes.len()
}

fn pictures(h: &Harness<'static, SlideApp>) -> usize {
    let d = h.state().session.active().unwrap();
    d.current_slide().unwrap().shapes.iter().filter(|s| matches!(s.kind, deckcraft_model::ShapeKind::Picture { .. })).count()
}

#[test]
fn command_v_with_an_image_on_the_clipboard_inserts_a_picture() {
    let mut h = harness(Services { clipboard_image: Some(Box::new(|| Some(png()))), ..Default::default() });
    assert_eq!(pictures(&h), 0);
    v(&mut h, false, egui::Modifiers::COMMAND);
    assert_eq!(pictures(&h), 1, "the dropped ⌘V press still pastes the image");
}

#[test]
fn command_v_pastes_shapes_copied_here() {
    let mut h = harness(Services::default());
    h.state_mut().run("shape.insert", json!({"preset": "rect", "rect": [100, 100, 200, 100]})).unwrap();
    h.state_mut().session.execute("edit.copy", &json!({})).unwrap();
    let before = shapes(&h);
    v(&mut h, false, egui::Modifiers::COMMAND);
    assert_eq!(shapes(&h), before + 1, "a shape without text pastes");
}

#[test]
fn a_paste_event_or_a_typed_v_doesnt_paste_again() {
    let mut h = harness(Services { clipboard_image: Some(Box::new(|| Some(png()))), ..Default::default() });
    // ⌘V with text on the clipboard: a Paste event, then the release.
    h.event(egui::Event::Paste("hello".into()));
    h.run_steps(2);
    let after_text = shapes(&h);
    v(&mut h, false, egui::Modifiers::COMMAND);
    assert_eq!(shapes(&h), after_text, "the release after a Paste event is not a second paste");
    assert_eq!(pictures(&h), 0);
    // A V typed (press seen), or a release without ⌘ (an input method took the press).
    v(&mut h, true, egui::Modifiers::NONE);
    v(&mut h, false, egui::Modifiers::NONE);
    v(&mut h, false, egui::Modifiers::NONE);
    assert_eq!(pictures(&h), 0, "plain V is never a paste");
}
