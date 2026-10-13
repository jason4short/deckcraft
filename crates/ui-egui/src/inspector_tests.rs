//! The Design inspector: its sections for each kind of selection, and that its controls run the
//! commands they name.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use deckcraft_model::{Fill, ShapeId};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

use super::*;
use crate::{Services, Shell, SlideApp};

fn harness() -> Harness<'static, SlideApp> {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    app.run("view.shell", json!({"shell": "canvas"})).unwrap();
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 1000.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    assert_eq!(h.state().ui.shell, Shell::Canvas);
    h
}

/// A filled rectangle with text, selected.
fn select_box(h: &mut Harness<'static, SlideApp>) -> u32 {
    let app = h.state_mut();
    let id = app.session.execute("shape.insert", &json!({"preset": "rect", "rect": [100, 100, 200, 100], "text": "Hi"})).unwrap()["id"]
        .as_u64()
        .unwrap() as u32;
    app.session.execute("selection.select", &json!({"ids": [id]})).unwrap();
    app.session.execute("shape.fill", &json!({"color": "#3366CC"})).unwrap();
    h.run_steps(3);
    id
}

fn shape(app: &SlideApp, id: u32) -> deckcraft_model::Shape {
    app.session.active().unwrap().shape(ShapeId(id)).cloned().unwrap()
}

#[test]
fn nothing_selected_shows_the_slide_layout_and_background() {
    let h = harness();
    assert!(h.query_by_label("Slide").is_some());
    assert!(h.query_by_label("Background").is_some());
    assert!(h.query_by_label("Text").is_none());
}

#[test]
fn a_selected_shape_shows_align_text_link_fill_and_opacity() {
    let mut h = harness();
    select_box(&mut h);
    for label in ["Align left", "Text", "Bold (⌘B)", "Link", "Fill", "Opacity", "More options"] {
        assert!(h.query_by_label(label).is_some(), "`{label}` is shown");
    }
}

#[test]
fn text_buttons_format_the_selection() {
    let mut h = harness();
    select_box(&mut h);
    h.get_by_label("Bold (⌘B)").click();
    h.run_steps(3);
    let st = h.state_mut().session.execute("format.state", &json!({})).unwrap();
    assert_eq!(st["bold"], true);
    h.get_by_label("Numbering").click();
    h.run_steps(3);
    let st = h.state_mut().session.execute("format.state", &json!({})).unwrap();
    assert_eq!(st["numbering"], true);
}

#[test]
fn align_and_fill_buttons_run_their_commands() {
    let mut h = harness();
    let id = select_box(&mut h);
    h.get_by_label("Align left").click();
    h.run_steps(3);
    assert_eq!(shape(h.state(), id).xfrm.unwrap().x, 0.0, "aligned to the slide's left edge");
    h.get_by_label("No fill").click();
    h.run_steps(3);
    assert_eq!(shape(h.state(), id).fill, Some(Fill::None));
    h.get_by_label("Gradient").click();
    h.run_steps(3);
    assert!(matches!(shape(h.state(), id).fill, Some(Fill::Gradient(_))));
}

#[test]
fn opacity_reads_the_fill_or_the_picture() {
    let mut h = harness();
    let id = select_box(&mut h);
    let sc = deckcraft_model::Theme::default().colors;
    assert_eq!(opacity_of(&shape(h.state(), id), &sc), 1.0);
    h.state_mut().session.execute("shape.fill", &json!({"transparency": 0.25})).unwrap();
    assert!((opacity_of(&shape(h.state(), id), &sc) - 0.75).abs() < 0.01);
}

#[test]
fn weights_are_names_pptx_can_store() {
    // An unknown family offers the bold flag only.
    let (base, styles, cur) = weights("No Such Family", true);
    assert_eq!((base.as_str(), cur.as_str()), ("No Such Family", "Bold"));
    assert_eq!(styles, ["Regular", "Bold"]);
    // Every weight offered for a real family is Regular, Bold or a family name of its own.
    let db = FontDb::global();
    let family = db.families().into_iter().find(|f| db.styles(f).len() > 2).unwrap_or_else(|| "Inter".into());
    let (base, styles, _) = weights(&family, false);
    for s in &styles {
        assert!(s == "Regular" || s == "Bold" || db.has_family(&format!("{base} {s}")), "{base} {s}");
    }
    assert_eq!(weight_commands("Inter", "SemiBold")[0].1["family"], "Inter SemiBold");
    assert_eq!(weight_commands("Inter", "Bold")[1].1["on"], true);
}

#[test]
fn the_text_style_menu_names_the_style_and_applies_another() {
    let mut h = harness();
    let app = h.state_mut();
    let title = app.session.active().unwrap().current_slide().unwrap().shapes[0].id.0;
    // (An empty placeholder keeps its layout's formatting: the engine formats runs.)
    app.session.execute("text.set", &json!({"id": title, "text": "Pixel Grid"})).unwrap();
    app.session.execute("selection.select", &json!({"ids": [title]})).unwrap();
    h.run_steps(3);
    h.get_by_label("Text style: Title").click();
    h.run_steps(3);
    h.get_by_label("Body 3").click();
    h.run_steps(3);
    let st = h.state_mut().session.execute("format.state", &json!({})).unwrap();
    assert_eq!(st["size"], 20.0);
    assert_eq!(st["fontRef"], "+mn-lt");
    assert!(h.query_by_label("Text style: Body 3").is_some());
}
