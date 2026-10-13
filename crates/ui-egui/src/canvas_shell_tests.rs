//! The canvas shell through real egui input: switching layouts, what each one shows, the
//! inspector tabs and the toolbar's tools.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::{Session, ToolKind};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::{Services, Shell, SlideApp};

fn harness(shell: Shell) -> Harness<'static, SlideApp> {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    app.run("view.shell", json!({"shell": shell.id()})).unwrap();
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    // The app keeps asking for frames, so step instead of run().
    h.run_steps(3);
    h
}

#[test]
fn view_shell_switches_layouts_and_rejects_unknown_ones() {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    assert_eq!(app.ui.shell, Shell::Classic, "the ribbon stays the default");
    assert_eq!(app.run("view.shell", json!({"shell": "canvas"})).unwrap()["shell"], "canvas");
    assert_eq!(app.run("view.shell", json!({})).unwrap()["shell"], "classic", "no shell toggles");
    assert_eq!(app.run("view.shell", json!({})).unwrap()["shell"], "canvas");
    assert!(app.run("view.shell", json!({"shell": "figma"})).is_err());
    assert_eq!(app.ui.shell, Shell::Canvas, "a bad name changes nothing");
}

#[test]
fn the_layout_choice_is_kept_with_the_ui_state() {
    let ui = crate::UiState { shell: Shell::Canvas, ..Default::default() };
    let back: crate::UiState = serde_json::from_value(serde_json::to_value(&ui).unwrap()).unwrap();
    assert_eq!(back.shell, Shell::Canvas);
    let old: crate::UiState = serde_json::from_value(json!({"brightness": "dark"})).unwrap();
    assert_eq!(old.shell, Shell::Classic, "settings saved before the canvas shell open in the ribbon");
}

#[test]
fn the_canvas_shell_replaces_the_ribbon_with_sidebar_inspector_and_toolbar() {
    let h = harness(Shell::Canvas);
    assert!(h.query_by_label("File").is_none(), "no ribbon tabs");
    for label in ["Layers", "Design", "Animate", "Export", "Select (Esc)", "Text box"] {
        assert!(h.query_by_label(label).is_some(), "`{label}` is shown");
    }
    assert!(h.state().slide_rect.is_some(), "the slide is on the canvas");

    let h = harness(Shell::Classic);
    assert!(h.query_by_label("File").is_some(), "the ribbon is back");
    assert!(h.query_by_label("Layers").is_none());
}

#[test]
fn switching_layouts_leaves_the_presentation_alone() {
    let mut h = harness(Shell::Canvas);
    let before = h.state().session.active().unwrap().doc.clone();
    h.state_mut().run("view.shell", json!({"shell": "classic"})).unwrap();
    h.run_steps(2);
    h.state_mut().run("view.shell", json!({"shell": "canvas"})).unwrap();
    h.run_steps(2);
    let st = h.state().session.active().unwrap();
    assert!(std::sync::Arc::ptr_eq(&before, &st.doc), "no edit is made");
    assert!(!st.is_dirty());
}

#[test]
fn inspector_tabs_switch_between_design_and_animate() {
    let mut h = harness(Shell::Canvas);
    assert_eq!(h.state().ui.inspector_tab, "design");
    h.get_by_label("Animate").click();
    h.run_steps(2);
    assert_eq!(h.state().ui.inspector_tab, "animate");
    h.get_by_label("Design").click();
    h.run_steps(2);
    assert_eq!(h.state().ui.inspector_tab, "design");
}

#[test]
fn toolbar_picks_the_drawing_tool() {
    let mut h = harness(Shell::Canvas);
    h.get_by_label("Text box").click();
    h.run_steps(2);
    assert_eq!(h.state().session.tool.kind, ToolKind::TextBox);
    h.get_by_label("Select (Esc)").click();
    h.run_steps(2);
    assert_eq!(h.state().session.tool.kind, ToolKind::Select);
}

#[test]
fn the_canvas_shell_draws_in_dark_mode_and_with_nothing_open() {
    let mut h = harness(Shell::Canvas);
    h.state_mut().run("view.dark", json!({"on": true})).unwrap();
    h.run_steps(3);
    assert!(h.query_by_label("Layers").is_some());
    h.state_mut().run("app.closePresentation", json!({})).unwrap();
    h.run_steps(3);
    assert!(h.state().session.active().is_none());
}
