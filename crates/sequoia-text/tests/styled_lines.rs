#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::rc::Rc;

use sequoia_buffer::{SpanAttr, StyledBuffer, hash_spans_rebased};
use sequoia_core::Color;
use sequoia_text::{FontStyle, FontWeight, TextSystem, WrapMode};

const EMOJI: &str = "👨‍👩‍👧";
const FLAG: &str = "🇵🇱";

fn doc_and_buf() -> (String, StyledBuffer) {
    let l0 = format!("hi {EMOJI} yo");
    let l1 = format!("flaga {FLAG} end");
    let l2 = "café déjà vu".to_owned();
    let doc = format!("{l0}\n{l1}\n{l2}");
    let mut buf = StyledBuffer::from(doc.clone());

    let e0 = doc.find(EMOJI).expect("emoji in doc");
    buf.set_attr(
        e0..e0 + EMOJI.len(),
        SpanAttr {
            bold: true,
            mention: Some(7),
            ..SpanAttr::plain()
        },
    );
    let f0 = doc.find(FLAG).expect("flag in doc");
    buf.set_attr(
        f0..f0 + FLAG.len(),
        SpanAttr {
            underline: true,
            link: Some("https://pl.example".into()),
            ..SpanAttr::plain()
        },
    );
    let d0 = doc.find("déjà").expect("deja in doc");
    buf.set_attr(
        d0..d0 + "déjà".len(),
        SpanAttr {
            color: Some(Color::BLUE),
            size_delta: 4.0,
            ..SpanAttr::plain()
        },
    );
    (doc, buf)
}

fn fonts_available(sys: &mut TextSystem) -> bool {
    sys.shape("M", 16.0).glyph_count() > 0
}

fn extra_for(buf: &StyledBuffer, line: usize) -> u64 {
    let (s, e) = buf.lines().line_range(line);
    let spans = buf.spans_for_line(s..e);
    if spans.iter().all(|x| x.is_empty()) {
        0
    } else {
        let mut h = DefaultHasher::new();
        hash_spans_rebased(&spans, s, &mut h);
        h.finish()
    }
}

#[test]
fn joint_buffer_to_shaped_lines_unicode() {
    let (doc, buf) = doc_and_buf();
    assert_eq!(buf.lines().line_count(), 3);
    let mut sys = TextSystem::new();

    let lines_text: Vec<String> = (0..3)
        .map(|i| {
            let (s, e) = buf.lines().line_range(i);
            buf.substring(s..e)
        })
        .collect();
    assert_eq!(lines_text.join("\n"), doc);
    for (i, text) in lines_text.iter().enumerate().take(3) {
        let (s, e) = buf.lines().line_range(i);
        let spans = buf.spans_for_line(s..e);
        assert_eq!(spans.len(), 1, "line {i}: exactly one styled run");
        for sp in &spans {
            assert!(sp.start >= s && sp.end <= e, "clipped to line {i}");

            let rel = (sp.start - s, sp.end - s);
            let slice = text.get(rel.0..rel.1).expect("span on char boundary");
            assert!(!slice.is_empty());
        }
    }

    let (s0, e0) = buf.lines().line_range(0);
    let sp0 = buf.spans_for_line(s0..e0);
    let t0 = buf.substring(s0..e0);
    assert_eq!(&t0[sp0[0].start - s0..sp0[0].end - s0], EMOJI);
    assert_eq!(sp0[0].attr.mention, Some(7));
    let (s1, e1) = buf.lines().line_range(1);
    let sp1 = buf.spans_for_line(s1..e1);
    let t1 = buf.substring(s1..e1);
    assert_eq!(&t1[sp1[0].start - s1..sp1[0].end - s1], FLAG);
    assert_eq!(sp1[0].attr.link.as_deref(), Some("https://pl.example"));

    let shaped = sys.shape_lines_visible_buffer(
        &buf,
        0..3,
        14.0,
        FontWeight::NORMAL,
        0.0,
        WrapMode::Word,
        0.0,
        None,
        FontStyle::Normal,
    );
    assert_eq!(shaped.len(), 3);
    for (i, line) in shaped.iter().enumerate() {
        assert!(line.height() > 0.0, "line {i} has height");
        assert!(line.row_count() >= 1);
        assert!(line.width() >= 0.0);
    }

    assert!(shaped[2].height() >= shaped[0].height());

    sys.reset_line_reshapes();
    let via_full = sys.shape_lines_visible_full(
        0..3,
        |i| {
            let (s, e) = buf.lines().line_range(i);
            Some(buf.substring(s..e))
        },
        14.0,
        FontWeight::NORMAL,
        0.0,
        WrapMode::Word,
        0.0,
        None,
        FontStyle::Normal,
        &|i| extra_for(&buf, i),
    );
    assert_eq!(sys.line_reshapes(), 0, "same keys across both APIs");
    for i in 0..3 {
        assert!(
            Rc::ptr_eq(&shaped[i], &via_full[i]),
            "line {i} shared entry"
        );
    }

    for i in 0..3 {
        let (s, e) = buf.lines().line_range(i);
        let text = buf.substring(s..e);
        let spans = buf.spans_for_line(s..e);
        assert_eq!(
            TextSystem::line_hash(&text, &spans, s, 0.0),
            sequoia_buffer::line_hash(&text, &sequoia_buffer::rebase_spans(&spans, s), 0.0),
            "digest agreement on line {i} ({text:?})"
        );
    }
}

#[test]
fn joint_rich_from_styled_unicode() {
    use sequoia_core::{RenderItem, RenderList};

    let (_doc, buf) = doc_and_buf();
    let mut sys = TextSystem::new();

    for i in 0..3 {
        let (s, e) = buf.lines().line_range(i);
        let text = buf.substring(s..e);
        let spans = buf.spans_for_line(s..e);
        let rich = sys.shape_rich_from_styled(
            &text,
            s,
            &spans,
            16.0,
            0.0,
            WrapMode::Word,
            0.0,
            sequoia_text::RichAlign::Left,
        );

        let char_len = text.chars().count();
        let mut covered = vec![false; char_len];
        for line in rich.lines() {
            for c in line.chars.clone() {
                covered[c] = true;
            }
        }
        assert!(covered.iter().all(|&b| b), "line {i} fully covered");
        assert!(rich.height() > 0.0);

        match i {
            0 => {
                assert_eq!(rich.span_mention(1), Some(7));
            }
            1 => assert_eq!(rich.span_link(1), Some("https://pl.example")),
            _ => {}
        }

        let again = sys.shape_rich_from_styled(
            &text,
            s,
            &spans,
            16.0,
            0.0,
            WrapMode::Word,
            0.0,
            sequoia_text::RichAlign::Left,
        );
        assert!(Rc::ptr_eq(&rich, &again), "rich cache hit on line {i}");
        let mut list = RenderList::new();
        sys.draw_rich(&rich, (0.0, 0.0), Color::WHITE, &mut list);
        for item in list.items() {
            if let RenderItem::Text(t) = item {
                for g in &t.glyphs {
                    for v in g.uv {
                        assert!((0.0..=1.0).contains(&v), "uv in range");
                    }
                }
            }
        }
    }

    if fonts_available(&mut sys) {
        let (s, e) = buf.lines().line_range(0);
        let text = buf.substring(s..e);
        let spans = buf.spans_for_line(s..e);
        let rich = sys.shape_rich_from_styled(
            &text,
            s,
            &spans,
            16.0,
            0.0,
            WrapMode::Word,
            0.0,
            sequoia_text::RichAlign::Left,
        );
        let mid_y = rich.line(0).expect("line").top + rich.line_height() * 0.5;
        let x = rich.caret_pos(0).x + 1.0;
        let hit = rich.hit_point(x, mid_y);
        assert_eq!(hit.span, Some(0), "plain 'h' span");
        assert_eq!(rich.span_mention(hit.span.expect("span")), None);
    }
}

#[test]
fn joint_edit_unicode_doc_invalidates_one_line() {
    let (_doc, mut buf) = doc_and_buf();
    let mut sys = TextSystem::new();
    let n = buf.lines().line_count();
    let before = sys.shape_lines_visible_buffer(
        &buf,
        0..n,
        14.0,
        FontWeight::NORMAL,
        0.0,
        WrapMode::Word,
        0.0,
        None,
        FontStyle::Normal,
    );

    let (s, _e) = buf.lines().line_range(0);
    let _ = buf.replace(s..s + 2, "HO", None);
    sys.invalidate_for_edit(buf.lines(), s, s + 2);
    sys.reset_line_reshapes();
    let after = sys.shape_lines_visible_buffer(
        &buf,
        0..n,
        14.0,
        FontWeight::NORMAL,
        0.0,
        WrapMode::Word,
        0.0,
        None,
        FontStyle::Normal,
    );
    assert_eq!(sys.line_reshapes(), 1, "only the edited line reshapes");
    assert!(!Rc::ptr_eq(&before[0], &after[0]));
    assert!(Rc::ptr_eq(&before[1], &after[1]), "flag line survives");
    assert!(Rc::ptr_eq(&before[2], &after[2]), "accents line survives");
}
