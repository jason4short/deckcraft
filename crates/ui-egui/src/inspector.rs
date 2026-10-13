//! The canvas shell's Design inspector: compact Align, Text, Link, Fill and Opacity sections for
//! the selection, Layout and Background for the slide when nothing is selected, and the full
//! Format Shape pane under "More options". Every control runs an engine command, and each one
//! writes something PPTX already stores: font weights are offered only as names a font exposes
//! (Regular and Bold, or a family name such as "Inter SemiBold"), never as free axis values, and
//! opacity is the fill's or the picture's transparency.

use deckcraft_fonts::FontDb;
use deckcraft_model::{Fill, ShapeKind};
use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::icons::{self, Icon};
use crate::ribbon::{cref_param, enabled, fmt_state};
use crate::theme::{self, Tokens};
use crate::{SlideApp, widgets};

/// Letter spacing presets, in points (the values PowerPoint's Character Spacing menu uses).
const SPACING: [(&str, f64); 5] = [("Very tight", -3.0), ("Tight", -1.5), ("Normal", 0.0), ("Loose", 3.0), ("Very loose", 6.0)];

fn run(app: &mut SlideApp, id: &str, p: Value) {
    let _ = app.run(id, p);
}

pub fn design(app: &mut SlideApp, ui: &mut Ui) {
    let Some(st) = app.session.active() else { return };
    let editing = st.selection.text.as_ref().is_some_and(|t| !t.notes);
    let shapes: Vec<deckcraft_model::Shape> = st.selected_shapes().into_iter().cloned().collect();
    if shapes.is_empty() && !editing {
        slide(app, ui);
        return;
    }
    align(app, ui);
    let texty = editing || shapes.iter().any(|s| s.text.is_some() || matches!(s.kind, ShapeKind::Shape));
    if texty {
        text(app, ui);
    }
    link(app, ui);
    let picture = shapes.iter().all(|s| matches!(s.kind, ShapeKind::Picture { .. }));
    if !picture {
        fill(app, ui, shapes.first());
    }
    opacity(app, ui, shapes.first());
    more(app, ui);
}

// ----- Layout helpers ---------------------------------------------------------------------

/// A section: hairline above, bold title, optional icon at the right.
fn section(ui: &mut Ui, title: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(6.0);
    let y = ui.cursor().min.y;
    ui.painter().hline(ui.max_rect().x_range().expand(12.0), y, Stroke::new(1.0, t.separator));
    ui.add_space(10.0);
    ui.label(egui::RichText::new(title).font(theme::bold(13.0)).color(t.text));
    ui.add_space(4.0);
}

fn caption(ui: &mut Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    ui.label(egui::RichText::new(text).font(theme::font(11.5)).color(t.text_dim));
}

/// A square toggle in a segmented row.
fn seg(ui: &mut Ui, icon: Icon, tip: &str, on: bool, en: bool) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(30.0, 28.0), if en { Sense::click() } else { Sense::hover() });
    let p = ui.painter_at(r);
    if on {
        p.rect_filled(r, CornerRadius::same(5), t.accent.gamma_multiply(0.18));
    } else if resp.hovered() {
        p.rect_filled(r, CornerRadius::same(5), t.hover);
    }
    icons::paint(&p, r.shrink(7.0), icon, if on { t.accent } else { t.text }, !en);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, en, on, tip));
    resp.on_hover_text(tip)
}

/// A colour swatch button that opens the theme colour grid.
fn color_field(
    app: &SlideApp,
    ui: &mut Ui,
    label: &str,
    swatch: Option<Color32>,
    none_label: Option<&str>,
) -> Option<Option<deckcraft_model::ColorRef>> {
    let t = Tokens::get(ui.ctx());
    let sc = scheme(app);
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
    ui.painter().rect(
        r,
        CornerRadius::same(5),
        if resp.hovered() { t.hover } else { t.field_bg },
        Stroke::new(1.0, t.border),
        egui::StrokeKind::Inside,
    );
    let sw = Rect::from_center_size(pos2(r.min.x + 16.0, r.center().y), vec2(14.0, 14.0));
    match swatch {
        Some(c) => {
            ui.painter().rect(sw, CornerRadius::same(3), c, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        }
        None => {
            ui.painter().rect_stroke(sw, CornerRadius::same(3), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
            ui.painter().line_segment([sw.left_bottom(), sw.right_top()], Stroke::new(1.0, Color32::from_rgb(0xD0, 0x30, 0x30)));
        }
    }
    ui.painter().text(pos2(r.min.x + 32.0, r.center().y), Align2::LEFT_CENTER, label, theme::font(12.5), t.text);
    widgets::chevron(ui.painter(), pos2(r.max.x - 12.0, r.center().y), 3.0, t.text_dim);
    let mut out = None;
    widgets::color_popup(&resp).show(|ui| {
        out = widgets::color_grid(ui, &sc, none_label);
    });
    out
}

fn scheme(app: &SlideApp) -> deckcraft_color::ColorScheme {
    app.session.active().and_then(|d| d.doc.masters.first().map(|m| m.scheme())).unwrap_or_else(|| deckcraft_model::Theme::default().colors)
}

// ----- Align ------------------------------------------------------------------------------

fn align(app: &mut SlideApp, ui: &mut Ui) {
    let en = enabled(app, "arrange.align");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        for (icon, tip, edge) in [
            (Icon::AlignObjLeft, "Align left", "left"),
            (Icon::AlignObjCenter, "Align horizontal centers", "center"),
            (Icon::AlignObjRight, "Align right", "right"),
            (Icon::AlignObjTop, "Align top", "top"),
            (Icon::AlignObjMiddle, "Align vertical centers", "middle"),
            (Icon::AlignObjBottom, "Align bottom", "bottom"),
        ] {
            if seg(ui, icon, tip, false, en).clicked() {
                run(app, "arrange.align", json!({"edge": edge}));
            }
            if edge == "right" {
                ui.add_space(6.0);
            }
        }
        let r = seg(ui, Icon::More, "Distribute", false, en);
        egui::Popup::menu(&r).show(|ui| {
            if ui.button("Distribute horizontally").clicked() {
                run(app, "arrange.distribute", json!({"dir": "horizontal"}));
            }
            if ui.button("Distribute vertically").clicked() {
                run(app, "arrange.distribute", json!({"dir": "vertical"}));
            }
        });
    });
}

// ----- Text -------------------------------------------------------------------------------

/// The font's base family, the weights it can be set to and the current one.
///
/// A weight is offered when PPTX can name it: Regular and Bold (the bold flag), or a style whose
/// font has a family name of its own ("Inter SemiBold"), which is how PowerPoint stores it.
pub fn weights(font: &str, bold: bool) -> (String, Vec<String>, String) {
    let db = FontDb::global();
    let plain = || (font.to_string(), vec!["Regular".to_string(), "Bold".to_string()], if bold { "Bold" } else { "Regular" }.to_string());
    if font.is_empty() || !db.has_family(font) {
        return plain();
    }
    let base = db.face(font, "Regular").family.clone();
    let current = match font.get(base.len()..) {
        Some(rest) if font.len() > base.len() && font[..base.len()].eq_ignore_ascii_case(&base) => rest.trim().to_string(),
        _ if bold => "Bold".into(),
        _ => "Regular".into(),
    };
    let mut styles: Vec<String> = db
        .styles(&base)
        .into_iter()
        .filter(|s| !s.to_lowercase().contains("italic") && !s.to_lowercase().contains("oblique"))
        .filter(|s| s == "Regular" || s == "Bold" || db.has_family(&format!("{base} {s}")))
        .collect();
    for must in ["Regular", "Bold"] {
        if !styles.iter().any(|s| s == must) {
            styles.push(must.into());
        }
    }
    (base, styles, current)
}

/// The commands that set a weight (see [`weights`]).
pub fn weight_commands(base: &str, style: &str) -> Vec<(&'static str, Value)> {
    match style {
        "Regular" => vec![("format.font", json!({"family": base})), ("format.bold", json!({"on": false}))],
        "Bold" => vec![("format.font", json!({"family": base})), ("format.bold", json!({"on": true}))],
        s => vec![("format.font", json!({"family": format!("{base} {s}")})), ("format.bold", json!({"on": false}))],
    }
}

fn text(app: &mut SlideApp, ui: &mut Ui) {
    let st = fmt_state(app);
    let en = enabled(app, "format.bold");
    section(ui, "Text");
    let font = st.get("font").and_then(Value::as_str).unwrap_or("").to_string();
    let bold = st.get("bold").and_then(Value::as_bool).unwrap_or(false);
    let (base, styles, current) = weights(&font, bold);
    let w = ui.available_width();
    let families = crate::ribbon::font_families(app);
    widgets::dropdown(ui, "insp-font", if base.is_empty() { "Font" } else { &base }, w, en, |ui| {
        for f in families {
            if ui.selectable_label(f == base, &f).clicked() {
                run(app, "format.font", json!({"family": f}));
                ui.close();
            }
        }
    });
    ui.add_space(6.0);
    let half = (w - 8.0) / 2.0;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(half);
            caption(ui, "Weight");
            widgets::dropdown(ui, "insp-weight", &current, half, en, |ui| {
                for s in &styles {
                    if ui.selectable_label(*s == current, s).clicked() {
                        for (id, p) in weight_commands(&base, s) {
                            run(app, id, p);
                        }
                        ui.close();
                    }
                }
            });
        });
        ui.vertical(|ui| {
            caption(ui, "Size");
            let size = st.get("size").and_then(Value::as_f64).unwrap_or(18.0);
            if let Some(v) = widgets::spinner(ui, "insp-size", size, 1.0, " pt", half - 14.0) {
                run(app, "format.size", json!({"size": v.clamp(1.0, 4000.0)}));
            }
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(half);
            caption(ui, "Line height");
            let pct = match st.get("lineSpacing") {
                Some(v) if v.get("kind").and_then(Value::as_str) == Some("pct") => v.get("v").and_then(Value::as_f64).unwrap_or(1.0) * 100.0,
                _ => 100.0,
            };
            if let Some(v) = widgets::spinner(ui, "insp-lh", pct.round(), 5.0, "%", half - 14.0) {
                run(app, "format.lineSpacing", json!({"lines": (v / 100.0).clamp(0.1, 9.99)}));
            }
        });
        ui.vertical(|ui| {
            caption(ui, "Letter spacing");
            widgets::dropdown(ui, "insp-spacing", "Spacing", half, en, |ui| {
                for (l, pt) in SPACING {
                    if ui.button(l).clicked() {
                        run(app, "format.spacing", json!({"pt": pt}));
                        ui.close();
                    }
                }
            });
        });
    });
    ui.add_space(6.0);
    let flag = |k: &str| st.get(k).and_then(Value::as_bool).unwrap_or(false);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        for (icon, tip, id, key) in [
            (Icon::Bold, "Bold (⌘B)", "format.bold", "bold"),
            (Icon::Italic, "Italic (⌘I)", "format.italic", "italic"),
            (Icon::Underline, "Underline (⌘U)", "format.underline", "underline"),
            (Icon::Strikethrough, "Strikethrough", "format.strikethrough", "strike"),
        ] {
            if seg(ui, icon, tip, flag(key), en).clicked() {
                run(app, id, json!({"on": !flag(key)}));
            }
        }
        ui.add_space(8.0);
        let al = st.get("align").and_then(Value::as_str).unwrap_or("l");
        for (icon, tip, id, v) in [
            (Icon::AlignLeft, "Align text left", "format.alignLeft", "l"),
            (Icon::AlignCenter, "Center text", "format.alignCenter", "ctr"),
            (Icon::AlignRight, "Align text right", "format.alignRight", "r"),
        ] {
            if seg(ui, icon, tip, al == v, en).clicked() {
                run(app, id, json!({}));
            }
        }
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        if seg(ui, Icon::Bullets, "Bullets", flag("bullets"), en).clicked() {
            run(app, "format.bullets", json!({"on": !flag("bullets")}));
        }
        if seg(ui, Icon::Numbering, "Numbering", flag("numbering"), en).clicked() {
            run(app, "format.numbering", json!({"on": !flag("numbering")}));
        }
        ui.add_space(8.0);
        for (icon, tip, a) in [
            (Icon::AlignObjTop, "Text at top", "top"),
            (Icon::AlignObjMiddle, "Text in the middle", "middle"),
            (Icon::AlignObjBottom, "Text at bottom", "bottom"),
        ] {
            if seg(ui, icon, tip, false, en).clicked() {
                run(app, "format.anchor", json!({"anchor": a}));
            }
        }
    });
    ui.add_space(6.0);
    caption(ui, "Color");
    if let Some(Some(c)) = color_field(app, ui, "Text color", Some(t_auto(ui)), None) {
        run(app, "format.color", json!({"color": cref_param(&c)}));
    }
}

// ----- Link -------------------------------------------------------------------------------

fn link(app: &mut SlideApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    section(ui, "Link");
    let slides = app.session.active().map(|d| d.doc.slides.len()).unwrap_or(0);
    let id = ui.id().with("insp-link");
    let mut url: String = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
    ui.horizontal(|ui| {
        let w = ui.available_width() - 30.0;
        let r = ui.add(egui::TextEdit::singleline(&mut url).hint_text("Add URL or slide number").desired_width(w).font(theme::font(12.5)));
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !url.trim().is_empty() {
            let v = url.trim();
            match v.parse::<usize>() {
                Ok(n) if (1..=slides).contains(&n) => run(app, "insert.hyperlink", json!({"slide": n - 1})),
                _ => run(app, "insert.hyperlink", json!({"url": v})),
            }
            url.clear();
        }
        let (b, resp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(b, CornerRadius::same(5), t.hover);
        }
        widgets::chevron(ui.painter(), b.center(), 3.0, t.text_dim);
        egui::Popup::menu(&resp).show(|ui| {
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for i in 0..slides {
                    if ui.button(format!("Slide {}", i + 1)).clicked() {
                        run(app, "insert.hyperlink", json!({"slide": i}));
                    }
                }
            });
        });
    });
    ui.data_mut(|d| d.insert_temp(id, url));
}

// ----- Fill and opacity -------------------------------------------------------------------

fn fill(app: &mut SlideApp, ui: &mut Ui, sh: Option<&deckcraft_model::Shape>) {
    let Some(sh) = sh else { return };
    section(ui, "Fill");
    let sc = scheme(app);
    let kind = match &sh.fill {
        Some(Fill::None) => "none",
        Some(Fill::Gradient(_)) => "gradient",
        Some(Fill::Picture(_)) => "picture",
        Some(Fill::Solid { .. }) => "solid",
        _ => "auto",
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        if seg(ui, Icon::ShapeFill, "Solid", kind == "solid" || kind == "auto", true).clicked() && kind != "solid" {
            run(app, "shape.fill", json!({"color": "accent1"}));
        }
        if seg(ui, Icon::Grid, "Gradient", kind == "gradient", true).clicked() && kind != "gradient" {
            let g = crate::fillui::default_gradient(deckcraft_model::ColorRef::scheme(deckcraft_color::SchemeSlot::Accent1), 90.0);
            run(app, "shape.fill", json!({"gradient": crate::fillui::gradient_json(&g)}));
        }
        if seg(ui, Icon::Picture, "Picture…", kind == "picture", true).clicked()
            && let Some(pick) = app.services.pick_open.as_mut()
            && let Some(path) = pick("picture")
            && let Some(Ok(bytes)) = app.services.read.as_ref().map(|r| r(&path))
        {
            run(app, "shape.fill", json!({"picture": deckcraft_engine::cmd::base64_encode(&bytes)}));
        }
        if seg(ui, Icon::Close, "No fill", kind == "none", true).clicked() {
            run(app, "shape.fill", json!({"none": true}));
        }
    });
    ui.add_space(4.0);
    match &sh.fill {
        Some(Fill::Gradient(g)) => {
            if let Some(ng) = crate::fillui::gradient_editor(ui, "insp-grad", g, &sc) {
                run(app, "shape.fill", json!({"gradient": crate::fillui::gradient_json(&ng)}));
            }
        }
        Some(Fill::Picture(_)) => {}
        f => {
            let (label, swatch) = match f {
                Some(Fill::Solid { color }) => {
                    let c = color.resolve(&sc, None);
                    (format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b), Some(theme::to_color32(deckcraft_color::Rgba { a: 255, ..c })))
                }
                Some(Fill::None) => ("No fill".to_string(), None),
                _ => ("Automatic".to_string(), Some(t_auto(ui))),
            };
            match color_field(app, ui, &label, swatch, Some("No Fill")) {
                Some(Some(c)) => run(app, "shape.fill", json!({"color": cref_param(&c)})),
                Some(None) => run(app, "shape.fill", json!({"none": true})),
                None => {}
            }
        }
    }
}

fn t_auto(ui: &Ui) -> Color32 {
    Tokens::get(ui.ctx()).text_faint
}

/// The selection's opacity, 0–1: a picture's, or a solid fill's colour alpha.
pub fn opacity_of(sh: &deckcraft_model::Shape, sc: &deckcraft_color::ColorScheme) -> f64 {
    match (&sh.kind, &sh.fill) {
        (ShapeKind::Picture { fill }, _) => fill.alpha.unwrap_or(1.0),
        (_, Some(Fill::Solid { color })) => color.resolve(sc, None).a as f64 / 255.0,
        _ => 1.0,
    }
}

fn opacity(app: &mut SlideApp, ui: &mut Ui, sh: Option<&deckcraft_model::Shape>) {
    let Some(sh) = sh else { return };
    let picture = matches!(sh.kind, ShapeKind::Picture { .. });
    let has_fill = matches!(sh.fill, Some(Fill::Solid { .. } | Fill::Gradient(_) | Fill::Picture(_)));
    if !picture && !has_fill {
        return;
    }
    section(ui, "Opacity");
    let sc = scheme(app);
    let id = ui.id().with(("insp-opacity", sh.id.0));
    let cur = (opacity_of(sh, &sc) * 100.0).round() as f32;
    let mut v: f32 = ui.data_mut(|d| d.get_temp(id).unwrap_or(cur));
    ui.spacing_mut().slider_width = (ui.available_width() - 64.0).max(60.0);
    let r = ui.add(egui::Slider::new(&mut v, 0.0..=100.0).suffix("%").integer());
    if r.drag_stopped() || (r.changed() && !r.dragged()) {
        let tr = 1.0 - v as f64 / 100.0;
        if picture {
            run(app, "picture.adjust", json!({"transparency": tr}));
        } else {
            run(app, "shape.fill", json!({"transparency": tr}));
        }
    }
    if r.dragged() {
        ui.data_mut(|d| d.insert_temp(id, v));
    } else {
        ui.data_mut(|d| d.remove::<f32>(id));
    }
}

// ----- Everything else --------------------------------------------------------------------

/// The full Format Shape pane (effects, size, text box options…), folded away.
fn more(app: &mut SlideApp, ui: &mut Ui) {
    section(ui, "More options");
    let id = ui.id().with("insp-more");
    let mut open: bool = ui.data_mut(|d| d.get_temp(id).unwrap_or(false));
    if ui.button(if open { "Hide shape effects, size and text box" } else { "Shape effects, size and text box…" }).clicked() {
        open = !open;
    }
    ui.data_mut(|d| d.insert_temp(id, open));
    if open {
        ui.add_space(4.0);
        crate::panes::format_shape(app, ui);
    }
}

/// Nothing selected: the slide's layout, then its background.
fn slide(app: &mut SlideApp, ui: &mut Ui) {
    let Some(st) = app.session.active() else { return };
    let layout =
        st.current_slide().and_then(|s| st.doc.masters.first().and_then(|m| m.layouts.iter().find(|l| l.id == s.layout))).map(|l| l.name.clone());
    let t = Tokens::get(ui.ctx());
    ui.label(egui::RichText::new("Slide").font(theme::bold(13.0)).color(t.text));
    ui.add_space(4.0);
    caption(ui, "Layout");
    let w = ui.available_width();
    let r =
        ui.add_sized(vec2(w, 26.0), egui::Button::new(layout.unwrap_or_else(|| "Layout".into())).fill(t.field_bg).stroke(Stroke::new(1.0, t.border)));
    egui::Popup::menu(&r).width(420.0).show(|ui| crate::ribbon::layout_menu(app, ui, "slide.layout"));
    section(ui, "Background");
    crate::panes::background(app, ui);
}

#[cfg(test)]
#[path = "inspector_tests.rs"]
mod tests;
