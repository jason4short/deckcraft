//! Text styles for the canvas shell's inspector: Title, Body, Body 2, Body 3 and Plain text,
//! read from the deck's slide master (its title, body and other text styles, over the
//! presentation's default text style). Applying one writes its font, size, weight, slant, colour
//! and line spacing to the selected text with the ordinary formatting commands, as one undo step,
//! so a styled deck is a plain PPTX: theme font references (`+mj-lt`) stay references.

use deckcraft_model::text::{ListStyle, Spacing};
use deckcraft_model::{Fill, Presentation, RunProps};
use serde_json::{Value, json};

use crate::SlideApp;

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub name: &'static str,
    pub run: RunProps,
    pub line_spacing: Option<Spacing>,
}

impl TextStyle {
    pub fn size(&self) -> f64 {
        self.run.size.unwrap_or(18.0)
    }
    pub fn bold(&self) -> bool {
        self.run.bold.unwrap_or(false)
    }
}

/// The deck's text styles, from its first slide master.
pub fn styles(doc: &Presentation) -> Vec<TextStyle> {
    let Some(m) = doc.masters.first() else { return vec![] };
    let level = |list: &ListStyle, l: u8| {
        let mut run = list.level(l).map(|s| s.run.clone()).unwrap_or_default();
        let mut para = list.level(l).map(|s| s.para.clone()).unwrap_or_default();
        if let Some(d) = doc.default_text_style.level(l) {
            run.inherit(&d.run);
            para.inherit(&d.para);
        }
        (run, para.line_spacing)
    };
    [
        ("Title", &m.title_style, 0),
        ("Body", &m.body_style, 0),
        ("Body 2", &m.body_style, 1),
        ("Body 3", &m.body_style, 2),
        ("Plain text", &m.other_style, 0),
    ]
    .into_iter()
    .map(|(name, list, l)| {
        let (run, line_spacing) = level(list, l);
        TextStyle { name, run, line_spacing }
    })
    .collect()
}

/// The commands that give the selected text `style`.
pub fn commands(style: &TextStyle) -> Vec<(&'static str, Value)> {
    let r = &style.run;
    let mut v = vec![];
    if let Some(f) = &r.font {
        v.push(("format.font", json!({"family": f})));
    }
    v.push(("format.size", json!({"size": style.size()})));
    v.push(("format.bold", json!({"on": style.bold()})));
    v.push(("format.italic", json!({"on": r.italic.unwrap_or(false)})));
    if let Some(Fill::Solid { color }) = &r.fill {
        v.push(("format.color", json!({"color": crate::ribbon::cref_param(color)})));
    }
    match style.line_spacing {
        Some(Spacing::Pct(x)) => v.push(("format.lineSpacing", json!({"lines": x}))),
        Some(Spacing::Pts(x)) => v.push(("format.lineSpacing", json!({"pt": x}))),
        None => {}
    }
    v
}

/// The style the selection's formatting (`format.state`) matches: the same font, size and weight;
/// or else the only style in that font (a layout may enlarge a title, which is still a Title).
pub fn matching<'a>(styles: &'a [TextStyle], state: &Value) -> Option<&'a TextStyle> {
    let font = state.get("fontRef").and_then(Value::as_str);
    let size = state.get("size").and_then(Value::as_f64)?;
    let bold = state.get("bold").and_then(Value::as_bool).unwrap_or(false);
    let same_font = |s: &&TextStyle| s.run.font.is_none() || s.run.font.as_deref() == font;
    if let Some(s) = styles.iter().filter(same_font).find(|s| (s.size() - size).abs() < 0.01 && s.bold() == bold) {
        return Some(s);
    }
    let mut in_font = styles.iter().filter(|s| font.is_some() && s.run.font.as_deref() == font);
    match (in_font.next(), in_font.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    }
}

/// Run `cmds` as one undo step labelled `label` (the formatting commands each make their own).
pub fn apply(app: &mut SlideApp, label: &str, cmds: Vec<(&'static str, Value)>) {
    let before = app.session.active().map(|d| d.history.undo.len()).unwrap_or(0);
    for (id, p) in cmds {
        if let Err(e) = app.run(id, p) {
            app.set_status(e);
        }
    }
    let Some(st) = app.session.active_mut() else { return };
    let undo = &mut st.history.undo;
    if undo.len() > before + 1 {
        // Keep the first step (the document before the style), drop the ones after it.
        undo.truncate(before + 1);
        if let Some(e) = undo.last_mut() {
            e.label = label.to_string();
        }
    }
}

#[cfg(test)]
#[path = "textstyles_tests.rs"]
mod tests;
