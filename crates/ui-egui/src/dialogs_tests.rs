//! Dialogs: the shared frame (Esc, Enter, multi-line text), Paragraph opening on the paragraph's
//! values, and Preferences changing nothing until OK or Apply.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

use super::Dialog;
use crate::{Services, SlideApp};

fn harness(dialog: &str) -> Harness<'static, SlideApp> {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    app.dialog = Some(Dialog::new(dialog));
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h
}

fn key(h: &mut Harness<'static, SlideApp>, key: egui::Key, modifiers: egui::Modifiers) {
    h.event(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
    h.event(egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers });
    h.run_steps(3);
}

#[test]
fn escape_closes_and_the_title_names_the_dialog() {
    let mut h = harness("table");
    assert!(h.query_by_label("Insert Table").is_some());
    key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().dialog.is_none());
}

#[test]
fn enter_is_a_new_line_in_a_comment_and_command_enter_posts_it() {
    let mut h = harness("comment");
    h.state_mut().dialog.as_mut().unwrap().fields.insert("text".into(), "Looks good".into());
    h.run_steps(2);
    key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(h.state().dialog.is_some(), "Enter alone doesn't post");
    key(&mut h, egui::Key::Enter, egui::Modifiers::COMMAND);
    assert!(h.state().dialog.is_none(), "⌘Enter posts");
}

#[test]
fn paragraph_opens_on_the_paragraphs_values_and_applies_them() {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    let id = app.run("insert.textBox", json!({"rect": [50, 50, 300, 80], "text": "Hello"})).unwrap()["id"].as_u64().unwrap();
    app.run("selection.select", json!({"ids": [id]})).unwrap();
    app.run("format.paragraph", json!({"spaceBefore": 12, "lineSpacing": 1.5})).unwrap();
    app.dialog = Some(Dialog::new("paragraph"));
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    let f = |h: &Harness<'static, SlideApp>, k: &str| h.state().dialog.as_ref().unwrap().fields.get(k).cloned().unwrap_or_default();
    assert_eq!(f(&h, "before"), "12");
    assert_eq!(f(&h, "spacing"), "onehalf");
    h.state_mut().dialog.as_mut().unwrap().fields.insert("after".into(), "6".into());
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let st = h.state_mut().session.execute("format.state", &json!({})).unwrap();
    assert_eq!(st["spaceAfter"], 6.0);
    assert_eq!(st["spaceBefore"], 12.0, "kept");
}

#[test]
fn preferences_change_nothing_until_ok_or_apply() {
    let mut h = harness("preferences");
    assert!(h.query_by_label("Editing").is_some(), "sections at the left");
    h.get_by_label("Editing").click();
    h.run_steps(3);
    let guides = h.state().session.prefs.smart_guides;
    h.get_by_label("Show smart guides").click();
    h.run_steps(3);
    assert_eq!(h.state().session.prefs.smart_guides, guides, "staged, not applied");
    h.get_by_label("Apply").click();
    h.run_steps(3);
    assert_eq!(h.state().session.prefs.smart_guides, !guides, "Apply applies");
    assert!(h.state().dialog.is_some(), "and keeps the dialog");
    h.get_by_label("Show smart guides").click();
    h.run_steps(3);
    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    assert_eq!(h.state().session.prefs.smart_guides, !guides, "Cancel drops the unapplied change");
}
