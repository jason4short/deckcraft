//! Placeholders across the deck: finding the moved ones, putting them back on their layout's
//! position, and making one slide's position every slide's.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::json;

use crate::Session;
use crate::cmd::xfrm_of;

/// A title slide and three Title and Content slides.
fn deck() -> Session {
    let mut s = Session::with_new();
    for _ in 0..3 {
        s.execute("slide.new", &json!({})).unwrap();
    }
    s
}

/// The title placeholder's id on slide `i` (a title or a title slide's title).
fn title(s: &Session, i: usize) -> u32 {
    s.doc().unwrap().doc.slides[i].shapes.iter().find(|sh| sh.ph_type().is_some_and(|t| t.is_title())).unwrap().id.0
}

/// Where slide `i`'s title is drawn, and whether it has a position of its own.
fn title_at(s: &mut Session, i: usize) -> (f64, f64, bool) {
    let id = title(s, i);
    s.execute("slide.go", &json!({"index": i})).unwrap();
    let st = s.doc().unwrap();
    let sh = st.shape(deckcraft_model::ShapeId(id)).unwrap();
    let x = xfrm_of(&st.doc, &st.selection, sh);
    (x.x, x.y, sh.xfrm.is_some())
}

fn move_title(s: &mut Session, i: usize, x: f64) {
    let id = title(s, i);
    s.execute("slide.go", &json!({"index": i})).unwrap();
    s.execute("selection.select", &json!({"ids": [id]})).unwrap();
    s.execute("shape.move", &json!({"x": x})).unwrap();
}

#[test]
fn moved_lists_the_placeholders_off_their_layout() {
    let mut s = deck();
    assert_eq!(s.execute("placeholder.moved", &json!({})).unwrap(), json!([]), "a new deck follows its layouts");
    move_title(&mut s, 2, 40.0);
    let v = s.execute("placeholder.moved", &json!({})).unwrap();
    assert_eq!(v, json!([{"slide": 2, "id": title(&s, 2), "kind": "title"}]));
    assert_eq!(s.execute("placeholder.moved", &json!({"kind": "ftr"})).unwrap(), json!([]), "filtered by kind");
}

#[test]
fn reset_puts_every_title_back_on_its_layout() {
    let mut s = deck();
    let home = title_at(&mut s, 1);
    move_title(&mut s, 1, 40.0);
    move_title(&mut s, 3, 70.0);
    move_title(&mut s, 0, 10.0);
    let r = s.execute("placeholder.reset", &json!({"kind": "title"})).unwrap();
    assert_eq!(r["reset"], 3, "a title slide's title counts as a title");
    for i in 0..4 {
        assert!(!title_at(&mut s, i).2, "slide {i}'s title follows its layout");
    }
    assert_eq!(title_at(&mut s, 1), home);
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(title_at(&mut s, 3).0, 70.0, "one undo step brings them all back");
    assert!(s.execute("placeholder.reset", &json!({"kind": "nope"})).is_err());
}

#[test]
fn reset_can_keep_to_some_slides_and_takes_the_selected_kind() {
    let mut s = deck();
    move_title(&mut s, 1, 40.0);
    move_title(&mut s, 2, 40.0);
    // The moved title on slide 2 is selected.
    let r = s.execute("placeholder.reset", &json!({"slides": [1]})).unwrap();
    assert_eq!(r["reset"], 1);
    assert!(!title_at(&mut s, 1).2);
    assert!(title_at(&mut s, 2).2, "slide 3 wasn't asked for");
}

#[test]
fn to_layout_makes_one_slides_position_every_slides() {
    let mut s = deck();
    move_title(&mut s, 3, 70.0);
    move_title(&mut s, 2, 40.0);
    let r = s.execute("placeholder.toLayout", &json!({})).unwrap();
    assert_eq!(r["reset"], 2, "slides 3 and 4 had their own");
    for i in 1..4 {
        let (x, _, own) = title_at(&mut s, i);
        assert_eq!(x, 40.0, "slide {i}'s title at the new place");
        assert!(!own, "following the layout");
    }
    assert_ne!(title_at(&mut s, 0).0, 40.0, "the title slide has another layout");
    // New slides with that layout get it too.
    s.execute("slide.new", &json!({})).unwrap();
    assert_eq!(title_at(&mut s, 4).0, 40.0);
}
