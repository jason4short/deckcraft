//! Placeholders across the deck. A slide's placeholder takes its position and size from its
//! layout's until it's moved, when it keeps its own (PPTX `a:xfrm` on the slide), so a title
//! nudged on one slide drifts from the rest. These commands find the moved ones, put them back
//! on the layout's position, or make one slide's position the layout's for every slide.

use std::sync::Arc;

use deckcraft_model::resolve::{self, Ctx};
use deckcraft_model::{PhType, Presentation, Shape, ShapeId, Xfrm};
use serde_json::{Value, json};

use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "placeholder.moved",
            "Moved Placeholders",
            [],
            None,
            "{kind?: title|subTitle|body|dt|ftr|sldNum|… (a title and a title slide's title count as one)} → [{slide: index, id, kind}]: placeholders placed off their layout's position or size",
            has_doc,
            moved
        ),
        cmd!(
            "placeholder.reset",
            "Reset Placeholder Positions",
            [],
            None,
            "{kind?: (default: the selected placeholder's), slides?: [index] (default: every slide)} → {reset}: those placeholders take their layout's position and size again (text and formatting stay)",
            has_doc,
            reset
        ),
        cmd!(
            "placeholder.toLayout",
            "Use Position on All Slides",
            [],
            None,
            "{id?: a placeholder on the current slide (default: the selected one)} → {reset}: its position and size become its layout placeholder's, and every slide with that layout follows",
            has_slide,
            to_layout
        ),
    ]
}

/// Title and centred title are one kind here: the same job on different layouts.
fn same_kind(a: PhType, b: PhType) -> bool {
    a == b || (a.is_title() && b.is_title())
}

fn kind_param(s: &Session, p: &Value, cmd: &str) -> Result<PhType> {
    if let Some(k) = str_param(p, "kind") {
        let t = PhType::from_xml(k);
        if t.xml() != k {
            return Err(bad(cmd, format!("unknown placeholder kind `{k}`")));
        }
        return Ok(t);
    }
    let st = s.doc()?;
    st.selected_shapes().iter().find_map(|sh| sh.ph_type()).ok_or_else(|| bad(cmd, "give `kind` or select a placeholder"))
}

/// The position a placeholder inherits from its layout (or master), if it inherits one.
fn inherited(ctx: &Ctx, sh: &Shape) -> Option<Xfrm> {
    let (lp, mp) = resolve::parents(ctx, sh);
    lp.and_then(|s| s.xfrm).or_else(|| mp.and_then(|s| s.xfrm))
}

fn differs(a: &Xfrm, b: &Xfrm) -> bool {
    [a.x - b.x, a.y - b.y, a.w - b.w, a.h - b.h].iter().any(|d| d.abs() > 0.5) || (a.rot - b.rot).abs() > 0.01
}

/// `(slide, shape, kind)` for placeholders of `kind` (any, when None) on `slides` that have a
/// position of their own and a layout or master one to go back to; `only_moved` keeps the ones
/// whose own position differs from it.
fn placed(doc: &Presentation, slides: &[usize], kind: Option<PhType>, only_moved: bool) -> Vec<(usize, ShapeId, PhType)> {
    let mut out = vec![];
    for &i in slides {
        let Some(sl) = doc.slides.get(i) else { continue };
        let Some(ctx) = Ctx::for_slide(doc, sl) else { continue };
        for sh in &sl.shapes {
            let (Some(t), Some(own)) = (sh.ph_type(), sh.xfrm.as_ref()) else { continue };
            if kind.is_some_and(|k| !same_kind(k, t)) {
                continue;
            }
            let Some(from) = inherited(&ctx, sh) else { continue };
            if !only_moved || differs(own, &from) {
                out.push((i, sh.id, t));
            }
        }
    }
    out
}

fn moved(s: &mut Session, p: &Value) -> Result<Value> {
    let kind = match str_param(p, "kind") {
        Some(_) => Some(kind_param(s, p, "placeholder.moved")?),
        None => None,
    };
    let doc = &s.doc()?.doc;
    let all: Vec<usize> = (0..doc.slides.len()).collect();
    let v: Vec<Value> = placed(doc, &all, kind, true).into_iter().map(|(i, id, t)| json!({"slide": i, "id": id, "kind": t.xml()})).collect();
    Ok(Value::Array(v))
}

/// Clear the own position of `targets` (slide, shape) so they follow their layout again.
fn clear(doc: &mut Presentation, targets: &[(usize, ShapeId, PhType)]) {
    for (i, id, _) in targets {
        if let Some(sl) = doc.slides.get_mut(*i)
            && let Some(sh) = Arc::make_mut(sl).shapes.iter_mut().find(|sh| sh.id == *id)
        {
            sh.xfrm = None;
        }
    }
}

fn reset(s: &mut Session, p: &Value) -> Result<Value> {
    let kind = kind_param(s, p, "placeholder.reset")?;
    let st = s.doc()?;
    let slides: Vec<usize> = match p.get("slides").and_then(Value::as_array) {
        Some(a) => a.iter().filter_map(Value::as_u64).map(|i| i as usize).collect(),
        None => (0..st.doc.slides.len()).collect(),
    };
    let targets = placed(&st.doc, &slides, Some(kind), false);
    if targets.is_empty() {
        return Ok(json!({"reset": 0}));
    }
    s.edit(|doc, _| {
        clear(doc, &targets);
        Ok(json!({"reset": targets.len()}))
    })
}

fn to_layout(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "placeholder.toLayout";
    let st = s.doc()?;
    let doc = &st.doc;
    let id = match id_param(p, "id") {
        Some(id) => id,
        None => st.selected_shapes().iter().find(|sh| sh.ph.is_some()).map(|sh| sh.id).ok_or_else(|| bad(CMD, "select a placeholder"))?,
    };
    let slide = doc.slides.get(st.selection.slide).ok_or_else(|| bad(CMD, "no slide"))?;
    let sh = slide.shapes.iter().find(|sh| sh.id == id && sh.ph.is_some()).ok_or_else(|| bad(CMD, "not a placeholder on this slide"))?;
    let ctx = Ctx::for_slide(doc, slide).ok_or_else(|| bad(CMD, "no layout"))?;
    let lp = resolve::parents(&ctx, sh).0.ok_or_else(|| bad(CMD, "this placeholder has no place on its layout"))?;
    let (layout, target, x) = (slide.layout, lp.id, resolve::xfrm(&ctx, sh));
    // Every placeholder that follows the same layout placeholder.
    let mut followers = vec![];
    for (i, sl) in doc.slides.iter().enumerate().filter(|(_, sl)| sl.layout == layout) {
        let Some(ctx) = Ctx::for_slide(doc, sl) else { continue };
        for s in sl.shapes.iter().filter(|s| s.ph.is_some() && s.xfrm.is_some()) {
            if resolve::parents(&ctx, s).0.is_some_and(|l| l.id == target) {
                followers.push((i, s.id, s.ph_type().unwrap_or(PhType::Body)));
            }
        }
    }
    s.edit(|doc, _| {
        let lsh = doc
            .masters
            .iter_mut()
            .find_map(|m| {
                let has = m.layouts.iter().any(|l| l.id == layout);
                has.then(|| Arc::make_mut(m)).and_then(|m| m.layouts.iter_mut().find(|l| l.id == layout))
            })
            .and_then(|l| l.shapes.iter_mut().find(|s| s.id == target))
            .ok_or_else(|| bad(CMD, "the layout placeholder is gone"))?;
        lsh.xfrm = Some(x);
        clear(doc, &followers);
        Ok(json!({"reset": followers.len()}))
    })
}

#[cfg(test)]
#[path = "placeholder_tests.rs"]
mod tests;
