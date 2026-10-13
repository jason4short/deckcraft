//! Text styles: read from the master, applied as formatting PPTX stores, in one undo step.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use serde_json::json;

use super::*;
use crate::{Services, SlideApp};

fn app_with_box() -> (SlideApp, u64) {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    let id = app.session.execute("shape.insert", &json!({"preset": "rect", "rect": [100, 100, 300, 100], "text": "Hello"})).unwrap()["id"]
        .as_u64()
        .unwrap();
    app.session.execute("selection.select", &json!({"ids": [id]})).unwrap();
    (app, id)
}

#[test]
fn the_default_master_has_title_and_body_styles() {
    let (app, _) = app_with_box();
    let doc = app.session.active().unwrap().doc.clone();
    let st = styles(&doc);
    let names: Vec<_> = st.iter().map(|s| s.name).collect();
    assert_eq!(names, ["Title", "Body", "Body 2", "Body 3", "Plain text"]);
    let (title, body) = (&st[0], &st[1]);
    assert!(title.size() > body.size(), "a title is bigger than body text");
    assert!(st[2].size() <= body.size() && st[3].size() <= st[2].size(), "deeper body levels aren't bigger");
}

#[test]
fn applying_a_style_formats_the_text_in_one_undo_step() {
    let (mut app, id) = app_with_box();
    let doc = app.session.active().unwrap().doc.clone();
    let title = styles(&doc).into_iter().next().unwrap();
    let undo0 = app.session.active().unwrap().history.undo.len();
    apply(&mut app, "Text Style: Title", commands(&title));
    let st = app.session.active().unwrap();
    assert_eq!(st.history.undo.len(), undo0 + 1, "one undo step");
    assert_eq!(st.history.undo.last().unwrap().label, "Text Style: Title");
    let state = app.session.execute("format.state", &json!({})).unwrap();
    assert_eq!(state["size"], title.size());
    assert_eq!(matching(&styles(&doc), &state).map(|s| s.name), Some("Title"), "the menu shows the style just applied");
    if let Some(f) = &title.run.font {
        assert_eq!(state["fontRef"], f.as_str(), "a theme font stays a reference (+mj-lt)");
    }
    app.run("edit.undo", json!({})).unwrap();
    let text = app.session.execute("text.get", &json!({"id": id})).unwrap();
    assert_eq!(text["text"], "Hello");
    let state = app.session.execute("format.state", &json!({})).unwrap();
    assert_ne!(state["size"], title.size(), "undo takes the whole style off");
}

#[test]
fn a_styled_deck_round_trips_through_pptx() {
    let (mut app, id) = app_with_box();
    let doc = app.session.active().unwrap().doc.clone();
    let body2 = styles(&doc).into_iter().nth(2).unwrap();
    apply(&mut app, "Text Style: Body 2", commands(&body2));
    let before = app.session.active().unwrap().shape(deckcraft_model::ShapeId(id as u32)).unwrap().text.clone();
    let data = app.session.execute("file.saveBytes", &json!({"format": "pptx"})).unwrap()["data"].clone();
    app.session.execute("file.openBytes", &json!({"name": "styled.pptx", "data": data})).unwrap();
    let back = app.session.active().unwrap().doc.clone();
    let run = |b: &Option<deckcraft_model::TextBody>| b.as_ref().unwrap().paragraphs[0].runs[0].props.clone();
    let shape = back.slides[0].shapes.iter().find(|s| s.text.as_ref().is_some_and(|t| t.text() == "Hello")).unwrap();
    let (a, b) = (run(&before), run(&shape.text));
    assert_eq!((a.font, a.size, a.bold, a.italic), (b.font, b.size, b.bold, b.italic));
}

#[test]
fn custom_formatting_matches_no_style() {
    let (mut app, _) = app_with_box();
    app.run("format.size", json!({"size": 37.0})).unwrap();
    let doc = app.session.active().unwrap().doc.clone();
    let state = app.session.execute("format.state", &json!({})).unwrap();
    assert!(matching(&styles(&doc), &state).is_none());
}

#[test]
fn an_untouched_title_placeholder_shows_the_title_style() {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    let title = app.session.active().unwrap().current_slide().unwrap().shapes[0].id.0;
    app.session.execute("selection.select", &json!({"ids": [title]})).unwrap();
    let doc = app.session.active().unwrap().doc.clone();
    let state = app.session.execute("format.state", &json!({})).unwrap();
    assert_eq!(matching(&styles(&doc), &state).map(|s| s.name), Some("Title"), "{state}");
}
