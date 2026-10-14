//! Dialogs (Insert Table, Slide Size, Header & Footer, Hyperlink, Set Up Show, Zoom, …) and the
//! About window. The start screen is File › Home (`backstage.rs`).

use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::SlideApp;
use crate::theme::{self, Tokens};

#[derive(Clone, Debug, serde::Serialize)]
pub struct Dialog {
    pub id: String,
    pub params: Value,
    /// Field values while the dialog is open.
    #[serde(skip)]
    pub fields: std::collections::HashMap<String, String>,
}

impl Dialog {
    pub fn new(id: &str) -> Self {
        Dialog { id: id.into(), params: Value::Null, fields: Default::default() }
    }
    pub fn modal(&self) -> bool {
        true
    }
    fn get(&mut self, k: &str, default: &str) -> String {
        self.fields.entry(k.into()).or_insert_with(|| default.to_string()).clone()
    }
}

fn title(id: &str) -> &'static str {
    match id {
        "table" => "Insert Table",
        "slideSize" => "Slide Size",
        "headerFooter" => "Header and Footer",
        "hyperlink" => "Insert Hyperlink",
        "setupShow" => "Set Up Slide Show",
        "zoom" => "Zoom",
        "comment" => "New Comment",
        "altText" => "Alt Text",
        "outline" => "Slides from Outline",
        "renameSection" => "Rename Section",
        "renameShape" => "Rename",
        "renameLayout" => "Rename Layout",
        "paragraph" => "Paragraph",
        "about" => "About DeckCraft",
        "preferences" => "Preferences",
        "export" => "Export",
        "symbol" => "Symbol",
        "equation" => "Equation",
        "spelling" => "Spelling",
        "accessibility" => "Accessibility Checker",
        "language" => "Language",
        "customShow" => "Custom Shows",
        "chartData" => "Chart Data",
        "trim" => "Trim Media",
        "smartart" => "Choose a SmartArt Graphic",
        "quit" => "DeckCraft",
        _ => "DeckCraft",
    }
}

/// The width a dialog's body takes: (at least, at most).
fn widths(id: &str) -> (f32, f32) {
    match id {
        "about" => (640.0, 700.0),
        "export" => (580.0, 600.0),
        "preferences" => (620.0, 640.0),
        "chartData" => (420.0, 720.0),
        "symbol" | "smartart" | "outline" | "accessibility" | "spelling" => (440.0, 520.0),
        _ => (380.0, 440.0),
    }
}

/// The open dialog, as PhotoCraft's are: a modal card with its title and a hairline, the window
/// behind barely dimmed. It opens centred and stays where it's dragged (by its title); Esc
/// cancels it (the topmost, with no menu open), a click outside does nothing.
pub fn show(app: &mut SlideApp, ctx: &egui::Context) {
    let Some(mut d) = app.dialog.take() else { return };
    let id = egui::Id::new(("dialog", d.id.clone()));
    let pinned: Option<egui::Vec2> = ctx.data(|m| m.get_temp(id));
    let area = match pinned {
        Some(offset) => egui::Modal::default_area(id).anchor(Align2::LEFT_TOP, offset),
        None => egui::Modal::default_area(id),
    };
    let mut close = false;
    let mut drag = egui::Vec2::ZERO;
    let mut sizing = false;
    let prompt = matches!(d.id.as_str(), "quit" | "close");
    let modal = egui::Modal::new(id).area(area).backdrop_color(Color32::from_black_alpha(28)).frame(crate::form::frame(ctx)).show(ctx, |ui| {
        sizing = ui.is_sizing_pass();
        let (min, max) = widths(&d.id);
        ui.set_min_width(min);
        ui.set_max_width(max);
        ui.spacing_mut().item_spacing.y = 8.0;
        if !prompt {
            let label = egui::Label::new(egui::RichText::new(title(&d.id)).font(theme::bold(15.0))).selectable(false);
            let tr = ui.add(label).rect;
            let bar = Rect::from_min_max(tr.min, pos2(ui.max_rect().right(), tr.bottom()));
            drag = ui.interact(bar, id.with("title"), Sense::drag()).drag_delta();
            ui.add_space(2.0);
            crate::form::hairline(ui);
            ui.add_space(4.0);
        }
        close = body(app, ui, &mut d);
    });
    if !sizing && (pinned.is_none() || drag != egui::Vec2::ZERO) {
        let screen = ctx.content_rect();
        let room = (screen.size() - modal.response.rect.size()).max(egui::Vec2::ZERO);
        let offset = pinned.unwrap_or(modal.response.rect.min - screen.min) + drag;
        ctx.data_mut(|m| m.insert_temp(id, offset.clamp(egui::Vec2::ZERO, room)));
    }
    if !close && modal.is_top_modal && !modal.any_popup_open && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        close = true;
    }
    if close {
        ctx.data_mut(|m| m.remove::<egui::Vec2>(id));
    } else {
        app.dialog = Some(d);
    }
}

/// OK (labelled `ok_label`) and Cancel; Enter is OK.
fn buttons(ui: &mut Ui, ok_label: &str) -> (bool, bool) {
    buttons_ex(ui, ok_label, true)
}

/// [`buttons`] for a dialog typing several lines: Enter is a new line there, ⌘Enter is OK.
fn buttons_multiline(ui: &mut Ui, ok_label: &str) -> (bool, bool) {
    // ⌘ as the Enter press carried it.
    let cmd = ui.input(|i| {
        i.modifiers.command
            || i.events.iter().any(|e| matches!(e, egui::Event::Key { key: egui::Key::Enter, pressed: true, modifiers, .. } if modifiers.command))
    });
    buttons_ex(ui, ok_label, cmd)
}

fn buttons_ex(ui: &mut Ui, ok_label: &str, enter: bool) -> (bool, bool) {
    use crate::form::{Button, ButtonRole};
    match crate::form::buttons(ui, &[Button::new(ButtonRole::Default, ok_label), Button::new(ButtonRole::Cancel, "Cancel")], enter) {
        Some(ButtonRole::Default) => (true, false),
        Some(_) => (false, true),
        None => (false, false),
    }
}

/// A closing button alone (Close, Done).
fn close_button(ui: &mut Ui, label: &str) -> bool {
    use crate::form::{Button, ButtonRole};
    crate::form::buttons(ui, &[Button::new(ButtonRole::Default, label)], true).is_some()
}

/// A labelled text field row.
fn field(ui: &mut Ui, d: &mut Dialog, label: &str, key: &str, default: &str) {
    let mut v = d.get(key, default);
    ui.horizontal(|ui| {
        crate::form::label(ui, label);
        crate::form::text(ui, &mut v, crate::form::field_width(ui), "");
    });
    d.fields.insert(key.into(), v);
}

/// A labelled number field row (with its unit, ↑ / ↓ and arithmetic).
#[allow(clippy::too_many_arguments)]
fn number(ui: &mut Ui, d: &mut Dialog, label: &str, key: &str, default: f64, unit: &str, step: f64, range: (f64, f64)) -> f64 {
    let v = num(d, key, default);
    let mut out = v;
    ui.horizontal(|ui| {
        crate::form::label(ui, label);
        if let Some(n) = crate::form::number(ui, key, v, unit, step, range, 110.0) {
            out = n;
        }
    });
    d.fields.insert(key.into(), crate::widgets::fmt_num(out, ""));
    out
}

/// A labelled dropdown row over `options` (value, label); returns the value chosen.
fn choice(ui: &mut Ui, d: &mut Dialog, label: &str, key: &str, default: &str, options: &[(&str, &str)]) -> String {
    let cur = d.get(key, default);
    let shown = options.iter().find(|o| o.0 == cur).map(|o| o.1).unwrap_or(cur.as_str()).to_string();
    let mut chose = None;
    ui.horizontal(|ui| {
        crate::form::label(ui, label);
        let w = crate::form::field_width(ui);
        crate::form::dropdown(ui, label, &shown, w, |ui| {
            for (v, l) in options {
                if ui.selectable_label(*v == cur, *l).clicked() {
                    chose = Some(v.to_string());
                    ui.close();
                }
            }
        });
    });
    let v = chose.unwrap_or(cur);
    d.fields.insert(key.into(), v.clone());
    v
}

/// A checkbox row under the label column.
fn check(ui: &mut Ui, d: &mut Dialog, label: &str, key: &str, default: bool) -> bool {
    let mut v = d.get(key, if default { "1" } else { "0" }) == "1";
    ui.horizontal(|ui| {
        crate::form::no_label(ui);
        crate::form::checkbox(ui, &mut v, label);
    });
    d.fields.insert(key.into(), if v { "1".into() } else { "0".into() });
    v
}

fn num(d: &mut Dialog, key: &str, default: f64) -> f64 {
    crate::widgets::parse_num(&d.get(key, "")).filter(|v| v.is_finite()).unwrap_or(default)
}

/// Returns true when the dialog should close.
fn body(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) -> bool {
    let run = |app: &mut SlideApp, id: &str, p: Value| {
        let _ = app.run(id, p);
    };
    match d.id.as_str() {
        "table" => {
            let c = number(ui, d, "Columns", "cols", 5.0, "", 1.0, (1.0, 75.0));
            let r = number(ui, d, "Rows", "rows", 2.0, "", 1.0, (1.0, 75.0));
            let (ok, cancel) = buttons(ui, "Insert");
            if ok {
                let (r, c) = (r.round() as usize, c.round() as usize);
                run(app, "insert.table", json!({"rows": r, "cols": c}));
            }
            ok || cancel
        }
        "slideSize" => {
            let cur = app.session.active().map(|s| s.doc.slide_size).unwrap_or(deckcraft_model::defaults::WIDE);
            // The preset the size matches, else Custom.
            let w_in = num(d, "w", cur.width / 72.0);
            let h_in = num(d, "h", cur.height / 72.0);
            let matching = deckcraft_model::defaults::SLIDE_SIZES
                .iter()
                .find(|(_, w, h)| (w / 72.0 - w_in).abs() < 0.005 && (h / 72.0 - h_in).abs() < 0.005)
                .map(|p| p.0)
                .unwrap_or("Custom");
            let mut chose = None;
            ui.horizontal(|ui| {
                crate::form::label(ui, "Slides sized for");
                let w = crate::form::field_width(ui);
                crate::form::dropdown(ui, "Slides sized for", matching, w, |ui| {
                    for (label, w, h) in deckcraft_model::defaults::SLIDE_SIZES {
                        if ui.selectable_label(matching == *label, *label).clicked() {
                            chose = Some((*w, *h));
                            ui.close();
                        }
                    }
                });
            });
            if let Some((w, h)) = chose {
                d.fields.insert("w".into(), crate::widgets::fmt_num(w / 72.0, ""));
                d.fields.insert("h".into(), crate::widgets::fmt_num(h / 72.0, ""));
            }
            let w = number(ui, d, "Width", "w", cur.width / 72.0, "in", 0.1, (1.0, 56.0));
            let h = number(ui, d, "Height", "h", cur.height / 72.0, "in", 0.1, (1.0, 56.0));
            let scale = choice(ui, d, "Content", "scale", "ensureFit", &[("ensureFit", "Ensure fit"), ("maximize", "Maximize")]);
            crate::form::note(ui, "Ensure fit scales the content down to fit the new size; Maximize keeps it as large as it is.");
            let (ok, cancel) = buttons(ui, "OK");
            if ok {
                run(app, "design.slideSize", json!({"w": w * 72.0, "h": h * 72.0, "scale": scale}));
            }
            ok || cancel
        }
        "headerFooter" => {
            let hf = app.session.active().map(|s| s.doc.header_footer.clone()).unwrap_or_default();
            crate::form::section(ui, "Include on slide");
            let date = check(ui, d, "Date and time", "date", hf.date);
            let mut date_text = d.get("dateText", &hf.date_text);
            ui.add_enabled_ui(date, |ui| {
                ui.horizontal(|ui| {
                    crate::form::label(ui, "Fixed date");
                    crate::form::text(ui, &mut date_text, crate::form::field_width(ui), "Empty: today's date, updated");
                });
            });
            d.fields.insert("dateText".into(), date_text);
            let num_on = check(ui, d, "Slide number", "num", hf.slide_number);
            let foot = check(ui, d, "Footer", "footer", hf.footer);
            let mut footer_text = d.get("footerText", &hf.footer_text);
            ui.add_enabled_ui(foot, |ui| {
                ui.horizontal(|ui| {
                    crate::form::label(ui, "Footer text");
                    crate::form::text(ui, &mut footer_text, crate::form::field_width(ui), "");
                });
            });
            d.fields.insert("footerText".into(), footer_text);
            ui.add_space(2.0);
            let hide = check(ui, d, "Don't show on title slide", "hide", hf.hide_on_title);
            use crate::form::{Button, ButtonRole};
            let picked = crate::form::buttons(
                ui,
                &[
                    Button::new(ButtonRole::Default, "Apply to All"),
                    Button::new(ButtonRole::Alternate, "Apply"),
                    Button::new(ButtonRole::Cancel, "Cancel"),
                ],
                true,
            );
            let apply_all = picked == Some(ButtonRole::Default);
            let apply = picked == Some(ButtonRole::Alternate);
            if picked == Some(ButtonRole::Cancel) {
                d.fields.insert("cancel".into(), "1".into());
            }
            if apply || apply_all {
                let dt = d.get("dateText", "");
                let ft = d.get("footerText", "");
                run(
                    app,
                    "design.headerFooter",
                    json!({"date": date, "dateText": dt, "slideNumber": num_on, "footer": foot, "footerText": ft, "hideOnTitle": hide, "all": apply_all}),
                );
            }
            apply || apply_all || d.fields.contains_key("cancel")
        }
        "hyperlink" => {
            let mode = d.get("mode", "url");
            let modes = ["url", "slide", "email"];
            let at = modes.iter().position(|m| *m == mode).unwrap_or(0);
            if let Some(k) = crate::form::tabs(ui, &["Web Page or File", "This Document", "Email Address"], at) {
                d.fields.insert("mode".into(), modes.get(k).copied().unwrap_or("url").into());
            }
            match mode.as_str() {
                "slide" => {
                    let titles: Vec<String> = app
                        .session
                        .active()
                        .map(|s| s.doc.slides.iter().enumerate().map(|(i, sl)| format!("{}. {}", i + 1, sl.title())).collect())
                        .unwrap_or_default();
                    let sel = d.get("slide", "0").parse::<usize>().unwrap_or(0);
                    let t = Tokens::get(ui.ctx());
                    egui::Frame::NONE.fill(t.field).corner_radius(CornerRadius::same(6)).inner_margin(egui::Margin::same(4)).show(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            for (i, t) in titles.iter().enumerate() {
                                if ui.selectable_label(sel == i, t).clicked() {
                                    d.fields.insert("slide".into(), i.to_string());
                                }
                            }
                        });
                    });
                }
                "email" => field(ui, d, "Email address", "email", ""),
                _ => field(ui, d, "Address", "url", "https://"),
            }
            field(ui, d, "ScreenTip", "tip", "");
            let (ok, cancel) = buttons(ui, "OK");
            if ok {
                let tip = d.get("tip", "");
                let p = match mode.as_str() {
                    "slide" => json!({"slide": d.get("slide", "0").parse::<usize>().unwrap_or(0), "tooltip": tip}),
                    "email" => json!({"url": format!("mailto:{}", d.get("email", "")), "tooltip": tip}),
                    _ => json!({"url": d.get("url", ""), "tooltip": tip}),
                };
                run(app, "insert.hyperlink", p);
            }
            ok || cancel
        }
        "setupShow" => {
            let sh = app.session.active().map(|s| s.doc.show.clone()).unwrap_or_default();
            choice(
                ui,
                d,
                "Show type",
                "type",
                &sh.show_type,
                &[
                    ("speaker", "Presented by a speaker (full screen)"),
                    ("browsed", "Browsed by an individual (window)"),
                    ("kiosk", "Browsed at a kiosk (full screen)"),
                ],
            );
            crate::form::section(ui, "Show options");
            let lp = check(ui, d, "Loop continuously until 'Esc'", "loop", sh.loop_until_esc);
            let nn = check(ui, d, "Show without narration", "nn", sh.without_narration);
            let na = check(ui, d, "Show without animation", "na", sh.without_animation);
            let ut = check(ui, d, "Use timings, if present", "ut", sh.use_timings);
            let (ok, cancel) = buttons(ui, "OK");
            if ok {
                let ty = d.get("type", "speaker");
                run(app, "show.setup", json!({"type": ty, "loop": lp, "noNarration": nn, "noAnimation": na, "useTimings": ut}));
            }
            ok || cancel
        }
        "quit" => {
            let names: Vec<String> = app.session.documents().iter().filter(|d| d.is_dirty()).map(|d| d.title()).collect();
            if names.is_empty() {
                app.quit_confirmed = true;
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                return true;
            }
            let what = if names.len() == 1 { format!("“{}”", names[0]) } else { format!("{} presentations", names.len()) };
            prompt_head(ui, &format!("Do you want to save the changes you made to {what}?"));
            use crate::form::{Button, ButtonRole};
            let picked = crate::form::buttons(
                ui,
                &[
                    Button::new(ButtonRole::Default, "Save"),
                    Button::new(ButtonRole::Alternate, "Don't Save"),
                    Button::new(ButtonRole::Cancel, "Cancel"),
                ],
                true,
            );
            match picked {
                Some(ButtonRole::Default) => {
                    if app.save_all() {
                        app.quit_confirmed = true;
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    true
                }
                Some(ButtonRole::Alternate) => {
                    let _ = app.session.execute("file.recovery.discard", &json!({}));
                    app.quit_confirmed = true;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    true
                }
                Some(_) => true,
                None => false,
            }
        }
        "close" => {
            // File › Close and Ctrl+W on a presentation with unsaved changes.
            let Some(name) = app.session.active().map(|d| d.title()) else { return true };
            prompt_head(ui, &format!("Do you want to save the changes you made to “{name}”?"));
            use crate::form::{Button, ButtonRole};
            let picked = crate::form::buttons(
                ui,
                &[
                    Button::new(ButtonRole::Default, "Save"),
                    Button::new(ButtonRole::Alternate, "Don't Save"),
                    Button::new(ButtonRole::Cancel, "Cancel"),
                ],
                true,
            );
            match picked {
                Some(ButtonRole::Default) => {
                    app.save();
                    // In the browser a save is a download: the presentation stays "unsaved".
                    let web = app.services.pick_save.is_none() && app.services.download.is_some();
                    if web || app.session.active().is_some_and(|d| !d.is_dirty()) {
                        run(app, "file.close", json!({}));
                    }
                    true
                }
                Some(ButtonRole::Alternate) => {
                    run(app, "file.close", json!({}));
                    true
                }
                Some(_) => true,
                None => false,
            }
        }
        "zoom" => {
            for pct in [400, 200, 150, 100, 75, 66, 50, 33] {
                if ui.radio(false, format!("{pct}%")).clicked() {
                    let _ = app.run("view.zoom", json!({"percent": pct}));
                }
            }
            if ui.button("Fit").clicked() {
                app.ui.zoom = None;
            }
            close_button(ui, "Done")
        }
        "comment" => {
            let mut v = d.get("text", "");
            let w = ui.available_width();
            crate::form::text_area(ui, &mut v, w, 4, "Start a conversation");
            d.fields.insert("text".into(), v.clone());
            let (ok, cancel) = buttons_multiline(ui, "Post");
            if ok && !v.trim().is_empty() {
                run(app, "comment.add", json!({"text": v}));
                app.ui.pane = Some("comments".into());
            }
            ok || cancel
        }
        "altText" => {
            let cur = app.session.active().and_then(|s| s.selected_shapes().first().map(|x| x.descr.clone())).unwrap_or_default();
            crate::form::note(ui, "How would you describe this object and its context to someone who is blind or has low vision?");
            let mut v = d.get("text", &cur);
            let w = ui.available_width();
            crate::form::text_area(ui, &mut v, w, 4, "");
            d.fields.insert("text".into(), v.clone());
            let mut deco = d.get("deco", "0") == "1";
            crate::form::checkbox(ui, &mut deco, "Mark as decorative");
            d.fields.insert("deco".into(), if deco { "1".into() } else { "0".into() });
            let (ok, cancel) = buttons_multiline(ui, "OK");
            if ok {
                run(app, "shape.altText", json!({"text": v, "decorative": deco}));
            }
            ok || cancel
        }
        "outline" => {
            crate::form::note(ui, "Paste or type an outline. Unindented lines become slide titles; tab-indented lines become bullets.");
            let mut v = d.get("text", "");
            let w = ui.available_width();
            crate::form::text_area(ui, &mut v, w, 10, "");
            d.fields.insert("text".into(), v.clone());
            let (ok, cancel) = buttons_multiline(ui, "Insert");
            if ok {
                run(app, "slide.fromOutline", json!({"text": v}));
            }
            ok || cancel
        }
        "renameSection" | "renameShape" | "renameLayout" => {
            let cur = d.params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            field(ui, d, "Name", "name", &cur);
            let (ok, cancel) = buttons(ui, "Rename");
            if ok {
                let name = d.get("name", "");
                match d.id.as_str() {
                    "renameSection" => run(app, "section.rename", json!({"index": d.params.get("index").cloned().unwrap_or_default(), "name": name})),
                    "renameShape" => run(app, "shape.rename", json!({"id": d.params.get("id").cloned().unwrap_or_default(), "name": name})),
                    _ => run(app, "master.renameLayout", json!({"name": name})),
                }
            }
            ok || cancel
        }
        "paragraph" => {
            // Opens on the paragraph's own values.
            if d.fields.is_empty() {
                let st = app.session.execute("format.state", &json!({})).unwrap_or_default();
                let f = |k: &str| st.get(k).and_then(Value::as_f64).unwrap_or(0.0);
                let align = match st.get("align").and_then(Value::as_str) {
                    Some("ctr") => "center",
                    Some("r") => "right",
                    Some("just") => "justify",
                    _ => "left",
                };
                let first = f("indentFirst");
                let special = if first < -0.01 {
                    "hanging"
                } else if first > 0.01 {
                    "first"
                } else {
                    "none"
                };
                let lines = match st.get("lineSpacing") {
                    Some(v) if v.get("kind").and_then(Value::as_str) == Some("pct") => v.get("v").and_then(Value::as_f64).unwrap_or(1.0),
                    _ => 1.0,
                };
                let spacing = match lines {
                    l if (l - 1.0).abs() < 0.01 => "single",
                    l if (l - 1.5).abs() < 0.01 => "onehalf",
                    l if (l - 2.0).abs() < 0.01 => "double",
                    _ => "multiple",
                };
                for (k, v) in [
                    ("align", align.to_string()),
                    ("left", crate::widgets::fmt_num(f("indentLeft") / 72.0, "")),
                    ("special", special.to_string()),
                    ("by", crate::widgets::fmt_num(first.abs() / 72.0, "")),
                    ("before", crate::widgets::fmt_num(f("spaceBefore"), "")),
                    ("after", crate::widgets::fmt_num(f("spaceAfter"), "")),
                    ("spacing", spacing.to_string()),
                    ("lines", crate::widgets::fmt_num(lines, "")),
                ] {
                    d.fields.insert(k.into(), v);
                }
            }
            crate::form::section(ui, "General");
            let align = choice(
                ui,
                d,
                "Alignment",
                "align",
                "left",
                &[("left", "Left"), ("center", "Centered"), ("right", "Right"), ("justify", "Justified")],
            );
            crate::form::section(ui, "Indentation");
            let left = number(ui, d, "Before text", "left", 0.0, "in", 0.1, (0.0, 20.0));
            let special = choice(ui, d, "Special", "special", "none", &[("none", "None"), ("first", "First line"), ("hanging", "Hanging")]);
            let by = if special == "none" { 0.0 } else { number(ui, d, "By", "by", 0.5, "in", 0.1, (0.0, 20.0)) };
            crate::form::section(ui, "Spacing");
            let before = number(ui, d, "Before", "before", 0.0, "pt", 1.0, (0.0, 1584.0));
            let after = number(ui, d, "After", "after", 0.0, "pt", 1.0, (0.0, 1584.0));
            let spacing = choice(
                ui,
                d,
                "Line spacing",
                "spacing",
                "single",
                &[("single", "Single"), ("onehalf", "1.5 lines"), ("double", "Double"), ("multiple", "Multiple")],
            );
            let lines = match spacing.as_str() {
                "single" => 1.0,
                "onehalf" => 1.5,
                "double" => 2.0,
                _ => number(ui, d, "At", "lines", 1.2, "lines", 0.1, (0.1, 9.99)),
            };
            let (ok, cancel) = buttons(ui, "OK");
            if ok {
                let first = match special.as_str() {
                    "hanging" => -by,
                    "first" => by,
                    _ => 0.0,
                };
                run(
                    app,
                    "format.paragraph",
                    json!({"align": align, "indentLeft": left * 72.0, "indentFirst": first * 72.0, "spaceBefore": before, "spaceAfter": after, "lineSpacing": lines}),
                );
            }
            ok || cancel
        }
        "export" => {
            let fmt = d.get("fmt", "pdf");
            let pdf_layout = d.get("pdfLayout", "slides");
            ui.horizontal_top(|ui| {
                // Left: what to export and its settings.
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    crate::form::section(ui, "Format");
                    let t = Tokens::get(ui.ctx());
                    egui::Frame::NONE.fill(t.field).corner_radius(CornerRadius::same(6)).inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                        ui.set_width(288.0);
                        for (l, f) in [
                            ("PDF document", "pdf"),
                            ("PNG image of this slide", "png"),
                            ("PNG images of every slide", "pngAll"),
                            ("JPEG image of this slide", "jpeg"),
                            ("PowerPoint presentation (.pptx)", "pptx"),
                            ("Outline (.txt)", "outline"),
                        ] {
                            if crate::form::list_row(ui, l, fmt == f, 284.0).clicked() {
                                d.fields.insert("fmt".into(), f.into());
                            }
                        }
                    });
                    if fmt == "pdf" {
                        crate::form::section(ui, "PDF");
                        let mut chose = None;
                        let shown = match pdf_layout.as_str() {
                            "notes" => "Notes pages",
                            "h1" => "Handouts, 1 per page",
                            "h2" => "Handouts, 2 per page",
                            "h3" => "Handouts, 3 per page",
                            "h4" => "Handouts, 4 per page",
                            "h6" => "Handouts, 6 per page",
                            "h9" => "Handouts, 9 per page",
                            _ => "Full page slides",
                        };
                        crate::form::dropdown(ui, "Layout", shown, 288.0, |ui| {
                            for (k, l) in [
                                ("slides", "Full page slides"),
                                ("notes", "Notes pages"),
                                ("h1", "Handouts, 1 per page"),
                                ("h2", "Handouts, 2 per page"),
                                ("h3", "Handouts, 3 per page"),
                                ("h4", "Handouts, 4 per page"),
                                ("h6", "Handouts, 6 per page"),
                                ("h9", "Handouts, 9 per page"),
                            ] {
                                if ui.selectable_label(pdf_layout == k, l).clicked() {
                                    chose = Some(k);
                                    ui.close();
                                }
                            }
                        });
                        if let Some(k) = chose {
                            d.fields.insert("pdfLayout".into(), k.into());
                        }
                        let mut hidden = d.get("pdfHidden", "false") == "true";
                        if crate::form::checkbox(ui, &mut hidden, "Include hidden slides").changed() {
                            d.fields.insert("pdfHidden".into(), hidden.to_string());
                        }
                    }
                });
                ui.add_space(12.0);
                // Right: the current slide.
                ui.vertical(|ui| {
                    crate::form::section(ui, "Preview");
                    export_preview(app, ui, 248.0);
                });
            });
            let pdf_params = {
                let (layout, per) = match pdf_layout.as_str() {
                    "notes" => ("notes", 0),
                    l if l.starts_with('h') => ("handouts", l[1..].parse().unwrap_or(6)),
                    _ => ("slides", 0),
                };
                json!({"layout": layout, "perPage": per, "includeHidden": d.get("pdfHidden", "false") == "true"})
            };
            let (ok, cancel) = buttons(ui, "Export");
            if ok {
                let extra = if fmt == "pdf" { pdf_params } else { json!({}) };
                app.export_as(&fmt, extra);
            }
            ok || cancel
        }
        "symbol" => {
            ui.label("Click a symbol to insert it at the insertion point.");
            let groups: [(&str, &str); 6] = [
                ("Punctuation", "–—‘’“”…•·§¶†‡‰′″‹›«»¡¿"),
                ("Currency", "$€£¥¢₹₩₽₺₿"),
                ("Math", "±×÷≠≈≤≥∞√∑∏∫∂∆∇∈∉∩∪⊂⊃∀∃¬∧∨⇒⇔°′"),
                ("Greek", "αβγδεζηθικλμνξοπρστυφχψωΓΔΘΛΞΠΣΦΨΩ"),
                ("Arrows", "←↑→↓↔↕⇐⇑⇒⇓↖↗↘↙⟵⟶"),
                ("Shapes", "■□▪▫▲△▶▷▼▽◀◁◆◇○●◎★☆✓✔✗✘♠♣♥♦"),
            ];
            for (g, chars) in groups {
                ui.label(egui::RichText::new(g).font(theme::bold(12.0)));
                ui.horizontal_wrapped(|ui| {
                    for c in chars.chars() {
                        if ui.add(egui::Button::new(egui::RichText::new(c.to_string()).size(16.0)).min_size(vec2(26.0, 26.0))).clicked() {
                            if app.session.active().is_some_and(|s| s.selection.text.is_none()) {
                                let _ = app.run("text.edit", json!({}));
                            }
                            let _ = app.run("insert.symbol", json!({"text": c.to_string()}));
                        }
                    }
                });
            }
            close_button(ui, "Close")
        }
        "equation" => {
            crate::form::note(ui, "Type an equation in linear form (e.g. a^2+b^2=c^2, x=(-b±√(b^2-4ac))/2a).");
            let mut v = d.get("eq", "");
            let w = ui.available_width();
            crate::form::text(ui, &mut v, w, "a^2+b^2=c^2");
            d.fields.insert("eq".into(), v.clone());
            let (ok, cancel) = buttons(ui, "Insert");
            if ok && !v.is_empty() {
                let pretty = pretty_equation(&v);
                let size = app.session.active().map(|s| s.doc.slide_size).unwrap_or(deckcraft_model::defaults::WIDE);
                run(app, "insert.textBox", json!({"rect": [size.width / 2.0 - 150.0, size.height / 2.0 - 25.0, 300, 50], "text": pretty}));
                run(app, "text.exit", json!({}));
                run(app, "format.font", json!({"family": "Liberation Serif"}));
                run(app, "format.italic", json!({"on": true}));
                run(app, "format.size", json!({"size": 28}));
            }
            ok || cancel
        }
        "spelling" => {
            let v = app.session.execute("review.spelling", &json!({})).unwrap_or_default();
            let list = v.as_array().cloned().unwrap_or_default();
            if list.is_empty() {
                crate::form::note(ui, "The spelling check is complete. No issues found.");
            }
            for item in &list {
                ui.horizontal(|ui| {
                    let w = item.get("word").and_then(Value::as_str).unwrap_or("");
                    ui.label(format!("Slide {}: “{w}”", item.get("slide").and_then(Value::as_u64).unwrap_or(0) + 1));
                    if ui.small_button("Go to").clicked() {
                        let _ = app.run("slide.go", json!({"index": item.get("slide").cloned().unwrap_or_default()}));
                    }
                });
            }
            close_button(ui, "Close")
        }
        "accessibility" => {
            let v = app.session.execute("review.accessibility", &json!({})).unwrap_or_default();
            let list = v.as_array().cloned().unwrap_or_default();
            if list.is_empty() {
                crate::form::note(ui, "No accessibility issues found. People with disabilities should not have difficulty reading this document.");
            }
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for item in &list {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "Slide {}: {}{}",
                            item.get("slide").and_then(Value::as_u64).unwrap_or(0) + 1,
                            item.get("issue").and_then(Value::as_str).unwrap_or(""),
                            item.get("name").and_then(Value::as_str).map(|n| format!(" — {n}")).unwrap_or_default()
                        ));
                        if ui.small_button("Go to").clicked() {
                            let _ = app.run("slide.go", json!({"index": item.get("slide").cloned().unwrap_or_default()}));
                            if let Some(id) = item.get("shape") {
                                let _ = app.run("edit.select", json!({"ids": [id]}));
                            }
                        }
                    });
                }
            });
            close_button(ui, "Close")
        }
        "language" => {
            ui.label("Mark selected text as:");
            for l in [
                "English (United States)",
                "English (United Kingdom)",
                "Español",
                "Français",
                "Deutsch",
                "Italiano",
                "Português",
                "日本語",
                "中文",
                "العربية",
            ] {
                let _ = ui.selectable_label(l.starts_with("English (United States)"), l);
            }
            let (ok, cancel) = buttons(ui, "OK");
            ok || cancel
        }
        "customShow" => {
            let shows = app.session.active().map(|s| s.doc.custom_shows.clone()).unwrap_or_default();
            for s in &shows {
                ui.horizontal(|ui| {
                    ui.label(format!("{} ({} slides)", s.name, s.slides.len()));
                    if ui.small_button("Delete").clicked() {
                        let _ = app.run("show.customShow", json!({"name": s.name, "delete": true}));
                    }
                });
            }
            crate::form::section(ui, "New custom show");
            field(ui, d, "Name", "name", "Custom Show 1");
            field(ui, d, "Slides", "slides", "1");
            crate::form::note(ui, "Slide numbers in the order to show them, e.g. 1, 3, 5.");
            let (ok, cancel) = buttons(ui, "Create");
            if ok {
                let idx: Vec<usize> =
                    d.get("slides", "").split(',').filter_map(|s| s.trim().parse::<usize>().ok()).filter(|v| *v > 0).map(|v| v - 1).collect();
                run(app, "show.customShow", json!({"name": d.get("name", "Custom Show"), "slides": idx}));
            }
            ok || cancel
        }
        "chartData" => {
            let ch = app.session.active().and_then(|s| {
                s.selected_shapes()
                    .into_iter()
                    .find_map(|x| if let deckcraft_model::ShapeKind::Chart(c) = &x.kind { Some(((**c).clone(), x.id)) } else { None })
            });
            let Some((c, id)) = ch else {
                ui.label("Select a chart first.");
                let (ok, cancel) = buttons(ui, "Close");
                return ok || cancel;
            };
            ui.label("Edit the data: first row = series names, first column = categories.");
            let mut grid: Vec<Vec<String>> = d.params.get("grid").and_then(|g| serde_json::from_value(g.clone()).ok()).unwrap_or_else(|| {
                let mut g = vec![std::iter::once(String::new()).chain(c.series.iter().map(|s| s.name.clone())).collect::<Vec<_>>()];
                for (i, cat) in c.categories.iter().enumerate() {
                    let mut row = vec![cat.clone()];
                    for s in &c.series {
                        row.push(s.values.get(i).copied().flatten().map(|v| v.to_string()).unwrap_or_default());
                    }
                    g.push(row);
                }
                g
            });
            egui::Grid::new("chartdata").striped(true).show(ui, |ui| {
                for row in grid.iter_mut() {
                    for cell in row.iter_mut() {
                        ui.add(egui::TextEdit::singleline(cell).desired_width(70.0));
                    }
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("+ Row").clicked() {
                    let n = grid.first().map(|r| r.len()).unwrap_or(2);
                    grid.push(vec![String::new(); n]);
                }
                if ui.button("+ Series").clicked() {
                    for r in grid.iter_mut() {
                        r.push(String::new());
                    }
                }
            });
            d.params = json!({"grid": grid});
            let (ok, cancel) = buttons(ui, "OK");
            if ok {
                let names: Vec<String> = grid.first().map(|r| r.iter().skip(1).cloned().collect()).unwrap_or_default();
                let cats: Vec<String> = grid.iter().skip(1).map(|r| r.first().cloned().unwrap_or_default()).collect();
                let series: Vec<Value> = names.iter().enumerate().map(|(k, n)| json!({"name": n, "values": grid.iter().skip(1).map(|r| r.get(k + 1).and_then(|v| v.trim().parse::<f64>().ok())).collect::<Vec<_>>()})).collect();
                run(app, "chart.data", json!({"id": id.0, "categories": cats, "series": series}));
            }
            ok || cancel
        }
        "trim" => {
            crate::form::section(ui, "Trim");
            number(ui, d, "Start", "start", 0.0, "s", 0.1, (0.0, 36_000.0));
            number(ui, d, "End trim", "end", 0.0, "s", 0.1, (0.0, 36_000.0));
            crate::form::section(ui, "Fade");
            number(ui, d, "Fade in", "fin", 0.0, "s", 0.1, (0.0, 60.0));
            number(ui, d, "Fade out", "fout", 0.0, "s", 0.1, (0.0, 60.0));
            let (ok, cancel) = buttons(ui, "OK");
            if ok {
                run(
                    app,
                    "media.options",
                    json!({"trimStart": (num(d, "start", 0.0) * 1000.0) as u64, "trimEnd": (num(d, "end", 0.0) * 1000.0) as u64, "fadeIn": (num(d, "fin", 0.0) * 1000.0) as u64, "fadeOut": (num(d, "fout", 0.0) * 1000.0) as u64}),
                );
            }
            ok || cancel
        }
        "smartart" => {
            crate::form::note(ui, "Pick a graphic; type one item per line.");
            let kind = d.get("kind", "process");
            ui.horizontal_wrapped(|ui| {
                for (k, l) in [
                    ("list", "Basic List"),
                    ("process", "Basic Process"),
                    ("cycle", "Basic Cycle"),
                    ("hierarchy", "Hierarchy"),
                    ("pyramid", "Pyramid"),
                    ("matrix", "Matrix"),
                ] {
                    if ui.selectable_label(kind == k, l).clicked() {
                        d.fields.insert("kind".into(), k.into());
                    }
                }
            });
            let mut v = d.get("text", "Plan\nBuild\nLaunch");
            let w = ui.available_width();
            crate::form::text_area(ui, &mut v, w, 5, "");
            d.fields.insert("text".into(), v.clone());
            let (ok, cancel) = buttons_multiline(ui, "Insert");
            if ok {
                run(app, "insert.smartArt", json!({"kind": kind, "items": v.lines().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>()}));
            }
            ok || cancel
        }
        "about" => {
            let tab = d.get("tab", "about");
            let keys = ["about", "contributors", "models"];
            let at = keys.iter().position(|k| *k == tab).unwrap_or(0);
            if let Some(k) = crate::form::tabs(ui, &["About", "Contributors", "Models"], at) {
                d.fields.insert("tab".into(), keys.get(k).copied().unwrap_or("about").into());
            }
            match tab.as_str() {
                "contributors" | "models" => {
                    // The credits need room for the table; the About tab keeps its compact size.
                    ui.set_min_size(vec2(640.0, 380.0));
                    ui.set_max_height(380.0);
                    if tab == "models" { crate::credits::models_ui(ui) } else { crate::credits::contributors_ui(ui) }
                }
                _ => about(app, ui),
            }
            close_button(ui, "Close")
        }
        "preferences" => preferences(app, ui, d),
        _ => {
            ui.label(format!("“{}” is not available yet.", d.id));
            let (ok, cancel) = buttons(ui, "OK");
            ok || cancel
        }
    }
}

/// The settings Preferences edits, as the dialog's fields hold them while it's open.
fn prefs_now(app: &SlideApp) -> Vec<(&'static str, String)> {
    let p = &app.session.prefs;
    let b = |v: bool| if v { "1".to_string() } else { "0".to_string() };
    vec![
        ("dark", b(app.ui.brightness == theme::Brightness::Dark)),
        ("shell", app.ui.shell.id().to_string()),
        ("scale", crate::widgets::fmt_num((app.ui.ui_scale * 100.0).round() as f64, "")),
        ("guides", b(p.smart_guides)),
        ("snap", b(p.snap_to_grid)),
        ("quotes", b(p.smart_quotes)),
        ("autocorrect", b(p.autocorrect)),
        ("author", p.author.clone()),
    ]
}

/// Write the staged settings.
fn prefs_apply(app: &mut SlideApp, d: &mut Dialog) {
    let on = |d: &mut Dialog, k: &str| d.get(k, "0") == "1";
    let dark = on(d, "dark");
    let _ = app.run("view.dark", json!({"on": dark}));
    let shell = d.get("shell", "classic");
    if shell != app.ui.shell.id() {
        let _ = app.run("view.shell", json!({"shell": shell}));
    }
    app.ui.ui_scale = (num(d, "scale", 100.0) / 100.0).clamp(0.75, 1.75) as f32;
    app.session.prefs.smart_guides = on(d, "guides");
    app.session.prefs.snap_to_grid = on(d, "snap");
    app.session.prefs.smart_quotes = on(d, "quotes");
    app.session.prefs.autocorrect = on(d, "autocorrect");
    app.session.prefs.author = d.get("author", "");
}

/// Preferences, as PhotoCraft lays them out: sections in a list at the left, the section's
/// settings at the right, and OK / Cancel / Apply (nothing changes until OK or Apply).
fn preferences(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) -> bool {
    let t = Tokens::get(ui.ctx());
    if d.fields.is_empty() {
        for (k, v) in prefs_now(app) {
            d.fields.insert(k.into(), v);
        }
        d.fields.insert("section".into(), "general".into());
    }
    let section = d.get("section", "general");
    let sections = [("general", "General"), ("editing", "Editing"), ("comments", "Comments")];
    ui.horizontal_top(|ui| {
        // The section list.
        ui.vertical(|ui| {
            ui.set_width(150.0);
            for (k, l) in sections {
                let on = section == k;
                let (r, resp) = ui.allocate_exact_size(vec2(150.0, 30.0), Sense::click());
                if on {
                    ui.painter().rect_filled(r, CornerRadius::same(6), t.selected);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(6), t.hover);
                }
                ui.painter().text(
                    pos2(r.min.x + 10.0, r.center().y),
                    Align2::LEFT_CENTER,
                    l,
                    if on { theme::bold(13.0) } else { theme::font(13.0) },
                    t.text,
                );
                resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, on, l));
                if resp.clicked() {
                    d.fields.insert("section".into(), k.into());
                }
            }
        });
        let (sep, _) = ui.allocate_exact_size(vec2(1.0, 300.0), Sense::hover());
        ui.painter().vline(sep.center().x, sep.y_range(), Stroke::new(1.0, t.separator));
        ui.add_space(8.0);
        // The section's settings.
        ui.vertical(|ui| {
            ui.set_width(420.0);
            ui.set_min_height(300.0);
            let title = sections.iter().find(|s| s.0 == section).map(|s| s.1).unwrap_or("General");
            ui.label(egui::RichText::new(title).font(theme::bold(14.0)));
            ui.add_space(4.0);
            match section.as_str() {
                "editing" => {
                    check(ui, d, "Show smart guides", "guides", true);
                    check(ui, d, "Snap objects to grid", "snap", true);
                    check(ui, d, "Replace straight quotes with smart quotes", "quotes", true);
                    check(ui, d, "AutoCorrect as you type", "autocorrect", true);
                }
                "comments" => {
                    field(ui, d, "Your name", "author", "");
                    crate::form::note(ui, "Shown on the comments you write.");
                }
                _ => {
                    let dark = choice(ui, d, "Appearance", "dark", "0", &[("0", "Light"), ("1", "Dark")]);
                    let _ = dark;
                    choice(ui, d, "Layout", "shell", "classic", &[("classic", "Ribbon (PowerPoint)"), ("canvas", "Canvas (Figma-style)")]);
                    number(ui, d, "Interface size", "scale", 100.0, "%", 5.0, (75.0, 175.0));
                }
            }
        });
    });
    let changed = prefs_now(app).into_iter().any(|(k, v)| d.fields.get(k).is_some_and(|s| *s != v));
    use crate::form::{Button, ButtonRole};
    let picked = crate::form::buttons(
        ui,
        &[
            Button::new(ButtonRole::Default, "OK"),
            Button::new(ButtonRole::Cancel, "Cancel"),
            Button::new(ButtonRole::Apply, "Apply").enabled(changed),
        ],
        true,
    );
    match picked {
        Some(ButtonRole::Default) => {
            prefs_apply(app, d);
            true
        }
        Some(ButtonRole::Apply) => {
            prefs_apply(app, d);
            false
        }
        Some(_) => true,
        None => false,
    }
}

/// The current slide, `w` wide, on a light board, with its number under it.
fn export_preview(app: &mut SlideApp, ui: &mut Ui, w: f32) {
    let t = Tokens::get(ui.ctx());
    let Some(st) = app.session.active() else { return };
    let doc = st.doc.clone();
    let i = st.selection.slide.min(doc.slides.len().saturating_sub(1));
    let h = w * (doc.slide_size.height / doc.slide_size.width.max(1.0)) as f32;
    let (board, _) = ui.allocate_exact_size(vec2(w, h + 24.0), Sense::hover());
    ui.painter().rect_filled(board, CornerRadius::same(8), t.field);
    let img = Rect::from_center_size(pos2(board.center().x, board.min.y + 12.0 + (h - 24.0) / 2.0), vec2(w - 24.0, h - 24.0));
    let mut budget = 2u32;
    let px = ((w - 24.0) * ui.ctx().pixels_per_point()).round() as u32;
    match app.textures.thumb(ui.ctx(), &doc, i, px, &mut budget) {
        Some(tex) => {
            ui.painter().image(tex.id(), img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            ui.painter().rect_filled(img, CornerRadius::ZERO, Color32::WHITE);
            ui.ctx().request_repaint();
        }
    }
    ui.painter().rect_stroke(img, CornerRadius::ZERO, Stroke::new(1.0, t.card_border), egui::StrokeKind::Outside);
    ui.painter().text(
        pos2(board.center().x, board.max.y - 10.0),
        Align2::CENTER_CENTER,
        format!("Slide {} of {}", i + 1, doc.slides.len()),
        theme::font(11.5),
        t.text_dim,
    );
}

/// A save prompt's head: the app's mark and the question in bold, the consequence in grey.
fn prompt_head(ui: &mut Ui, question: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
        paint_logo(ui.painter(), r);
        ui.vertical(|ui| {
            ui.add(egui::Label::new(egui::RichText::new(question).font(theme::bold(13.5))).wrap());
            crate::form::note(ui, "Your changes will be lost if you don't save them.");
        });
    });
}

fn pretty_equation(s: &str) -> String {
    let sup = |c: char| match c {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        'n' => 'ⁿ',
        'i' => 'ⁱ',
        '+' => '⁺',
        '-' => '⁻',
        _ => c,
    };
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '^' => {
                while let Some(&n) = chars.peek() {
                    if n.is_ascii_alphanumeric() || n == '+' || n == '-' {
                        out.push(sup(n));
                        chars.next();
                        if !n.is_ascii_digit() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            '*' => out.push('·'),
            _ => out.push(c),
        }
    }
    out.replace("sqrt", "√").replace("<=", "≤").replace(">=", "≥").replace("!=", "≠").replace("pi", "π").replace("+-", "±")
}

pub fn about(_app: &mut SlideApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    ui.vertical_centered(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
        paint_logo(ui.painter(), r);
        ui.label(egui::RichText::new("DeckCraft").font(theme::bold(22.0)));
        ui.label(egui::RichText::new(version_line()).color(t.text_dim));
        ui.add_space(6.0);
        ui.label("Presentations and slide shows, rebuilt from scratch in pure Rust.");
        ui.label("An open-source, clean-room project. Not affiliated with Microsoft.");
        ui.add_space(6.0);
        ui.hyperlink_to("Join our community on Discord", deckcraft_engine::links::DISCORD);
        ui.hyperlink_to("getartcraft.com/apps/deckcraft", deckcraft_engine::links::APP_PAGE);
        ui.hyperlink_to("Source on GitHub", deckcraft_engine::links::GITHUB);
        ui.add_space(6.0);
        ui.label(egui::RichText::new("MIT OR Apache-2.0 · © 2026 ArtCraft Team and the DeckCraft contributors").size(11.0).color(t.text_faint));
    });
}

/// "Version X.Y.Z" plus, for release builds, the short commit and build date. The version comes
/// from `[workspace.package] version` (the single source of truth, `cargo xtask version`); the
/// release workflow sets `DECKCRAFT_BUILD_SHA` / `DECKCRAFT_BUILD_DATE` at build time.
pub fn version_line() -> String {
    let mut s = format!("Version {}", env!("CARGO_PKG_VERSION"));
    let sha = option_env!("DECKCRAFT_BUILD_SHA").map(|s| s.get(..9).unwrap_or(s)).filter(|s| !s.is_empty());
    let date = option_env!("DECKCRAFT_BUILD_DATE").filter(|s| !s.is_empty());
    match (sha, date) {
        (Some(sha), Some(date)) => s.push_str(&format!(" ({sha}, {date})")),
        (Some(x), None) | (None, Some(x)) => s.push_str(&format!(" ({x})")),
        (None, None) => {}
    }
    s
}

/// DeckCraft's mark, drawn in code: a slide card with a play triangle in our orange.
pub fn paint_logo(p: &egui::Painter, r: Rect) {
    let orange = Color32::from_rgb(0xF2, 0x6B, 0x1D);
    let deep = Color32::from_rgb(0xC2, 0x4E, 0x14);
    let card = r.shrink(r.width() * 0.08);
    p.rect_filled(card.translate(vec2(r.width() * 0.04, r.width() * 0.05)), CornerRadius::same((r.width() * 0.14) as u8), deep);
    p.rect_filled(card, CornerRadius::same((r.width() * 0.14) as u8), orange);
    let c = card.center();
    let s = card.width() * 0.22;
    p.add(egui::Shape::convex_polygon(
        vec![pos2(c.x - s * 0.7, c.y - s), pos2(c.x + s, c.y), pos2(c.x - s * 0.7, c.y + s)],
        Color32::WHITE,
        Stroke::NONE,
    ));
}

#[cfg(test)]
#[path = "dialogs_tests.rs"]
mod tests;
