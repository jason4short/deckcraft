//! Find: the places a word or phrase appears in the presentation's slides (text in shapes,
//! placeholders and groups), in reading order, and going to one: its slide, its shape, and the
//! words selected as text being edited.

use deckcraft_model::{Presentation, ShapeId};
use serde_json::{Value, json};

use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "edit.find",
            "Find",
            [],
            None,
            "{query, matchCase?: bool, wholeWord?: bool} → [{slide: index, id, paragraph, start, end, context}]: where the text appears on the slides (start / end: characters in the paragraph)",
            has_doc,
            find
        ),
        cmd!(
            noundo "edit.showMatch",
            "Show Find Result",
            [],
            None,
            "{slide: index, id, paragraph, start, end}: go to the slide and select those characters of the shape's text",
            has_doc,
            show_match
        ),
    ]
}

/// One place the text was found.
#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    pub slide: usize,
    pub id: ShapeId,
    pub paragraph: usize,
    /// Characters within the paragraph.
    pub start: usize,
    pub end: usize,
}

fn same(a: char, b: char, case: bool) -> bool {
    if case { a == b } else { a == b || a.to_lowercase().eq(b.to_lowercase()) }
}

/// Where `query` starts in `text` (character indices), not overlapping.
pub fn find_in(text: &[char], query: &[char], case: bool, whole: bool) -> Vec<usize> {
    let mut out = vec![];
    if query.is_empty() || query.len() > text.len() {
        return out;
    }
    let word = |c: Option<&char>| c.is_some_and(|c| c.is_alphanumeric() || *c == '_');
    let mut i = 0;
    while i + query.len() <= text.len() {
        let hit = text.get(i..i + query.len()).is_some_and(|w| w.iter().zip(query).all(|(a, b)| same(*a, *b, case)));
        let bounded = !whole || (!word(i.checked_sub(1).and_then(|k| text.get(k))) && !word(text.get(i + query.len())));
        if hit && bounded {
            out.push(i);
            i += query.len();
        } else {
            i += 1;
        }
    }
    out
}

/// Every match on the slides, slide by slide, front shapes after back ones (as they're read).
pub fn matches(doc: &Presentation, query: &str, case: bool, whole: bool) -> Vec<Match> {
    let q: Vec<char> = query.chars().collect();
    let mut out = vec![];
    if q.is_empty() {
        return out;
    }
    for (si, slide) in doc.slides.iter().enumerate() {
        deckcraft_model::walk(&slide.shapes, &mut |sh, _| {
            let Some(t) = &sh.text else { return };
            for (pi, para) in t.paragraphs.iter().enumerate() {
                let chars: Vec<char> = para.text().chars().collect();
                for start in find_in(&chars, &q, case, whole) {
                    if out.len() < 10_000 {
                        out.push(Match { slide: si, id: sh.id, paragraph: pi, start, end: start + q.len() });
                    }
                }
            }
        });
    }
    out
}

fn find(s: &mut Session, p: &Value) -> Result<Value> {
    let query = str_param(p, "query").unwrap_or("");
    let case = bool_or(p, "matchCase", false);
    let whole = bool_or(p, "wholeWord", false);
    let doc = &s.doc()?.doc;
    let list: Vec<Value> = matches(doc, query, case, whole)
        .into_iter()
        .map(|m| {
            let context: String = doc
                .slides
                .get(m.slide)
                .and_then(|sl| deckcraft_model::find_shape(&sl.shapes, m.id))
                .and_then(|sh| sh.text.as_ref())
                .and_then(|t| t.paragraphs.get(m.paragraph))
                .map(|pa| pa.text().chars().skip(m.start.saturating_sub(20)).take(60).collect())
                .unwrap_or_default();
            json!({"slide": m.slide, "id": m.id, "paragraph": m.paragraph, "start": m.start, "end": m.end, "context": context})
        })
        .collect();
    Ok(Value::Array(list))
}

fn show_match(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "edit.showMatch";
    let slide = usize_param(p, "slide").ok_or_else(|| bad(CMD, "missing `slide`"))?;
    let id = id_param(p, "id").ok_or_else(|| bad(CMD, "missing `id`"))?;
    let para = usize_param(p, "paragraph").unwrap_or(0);
    let (start, end) = (usize_param(p, "start").unwrap_or(0), usize_param(p, "end").unwrap_or(0));
    s.execute("slide.go", &json!({"index": slide}))?;
    s.execute("text.edit", &json!({"id": id, "at": [para, start]}))?;
    s.execute("text.select", &json!({"anchor": [para, start], "caret": [para, end]}))?;
    ok()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn finds_words_ignoring_case_unless_asked() {
        assert_eq!(find_in(&chars("The theme of the day"), &chars("the"), false, false), vec![0, 4, 13]);
        assert_eq!(find_in(&chars("The theme of the day"), &chars("the"), true, false), vec![4, 13]);
        assert_eq!(find_in(&chars("The theme of the day"), &chars("the"), false, true), vec![0, 13], "whole words");
        assert_eq!(find_in(&chars("ÉCOLE école"), &chars("école"), false, false), vec![0, 6]);
        assert!(find_in(&chars("abc"), &chars(""), false, false).is_empty());
    }

    #[test]
    fn find_lists_matches_across_slides_and_selects_one() {
        let mut s = Session::with_new();
        s.execute("insert.textBox", &json!({"rect": [50, 50, 300, 50], "text": "Quarterly plan"})).unwrap();
        s.execute("slide.new", &json!({})).unwrap();
        s.execute("insert.textBox", &json!({"rect": [50, 50, 300, 50], "text": "The plan, and a planet"})).unwrap();
        let v = s.execute("edit.find", &json!({"query": "plan"})).unwrap();
        let list = v.as_array().unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0]["slide"], 0);
        assert_eq!((list[1]["slide"].as_u64(), list[1]["start"].as_u64()), (Some(1), Some(4)));
        let whole = s.execute("edit.find", &json!({"query": "plan", "wholeWord": true})).unwrap();
        assert_eq!(whole.as_array().unwrap().len(), 2, "not inside planet");
        s.execute("edit.showMatch", &list[2]).unwrap();
        let st = s.doc().unwrap();
        assert_eq!(st.selection.slide, 1);
        let t = st.selection.text.clone().unwrap();
        assert_eq!((t.anchor, t.caret), ((0, 16), (0, 20)), "the word is selected in its text box");
    }
}
