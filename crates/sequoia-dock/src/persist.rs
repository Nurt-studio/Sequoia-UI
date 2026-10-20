use crate::model::{DockModel, DockNode, FloatingPanel, PanelId, SplitAxis};

const TAG: &str = "dock-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeError {
    Expected { what: &'static str, pos: usize },

    Number { pos: usize },

    Trailing { pos: usize },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Expected { what, pos } => {
                write!(f, "dock: oczekiwano {what} @{pos}")
            }
            DecodeError::Number { pos } => write!(f, "dock: wrong number @{pos}"),
            DecodeError::Trailing { pos } => write!(f, "dock: nadmiar danych @{pos}"),
        }
    }
}

impl std::error::Error for DecodeError {}

pub fn encode(model: &DockModel) -> String {
    let mut s = String::from(TAG);
    s.push_str(" root=");
    match &model.root {
        None => s.push('-'),
        Some(n) => encode_node(n, &mut s),
    }
    s.push_str(" float=");
    for (i, f) in model.floating.iter().enumerate() {
        if i > 0 {
            s.push(';');
        }
        encode_float(f, &mut s);
    }
    s
}

pub fn decode(text: &str) -> Result<DockModel, DecodeError> {
    let rest = text.strip_prefix(TAG).ok_or(DecodeError::Expected {
        what: "tag `dock-v1`",
        pos: 0,
    })?;
    let rest = rest.strip_prefix(' ').ok_or(DecodeError::Expected {
        what: "spacja po tagu",
        pos: TAG.len(),
    })?;

    let base = TAG.len() + 1;
    let split_at = rest.find(" float=").ok_or(DecodeError::Expected {
        what: "` float=`",
        pos: base,
    })?;
    let (root_part, float_part) = (&rest[..split_at], &rest[split_at + " float=".len()..]);
    let float_base = base + split_at + " float=".len();
    let root_src = root_part
        .strip_prefix("root=")
        .ok_or(DecodeError::Expected {
            what: "`root=`",
            pos: base,
        })?;
    let root = if root_src == "-" {
        None
    } else {
        let mut p = Parser {
            src: root_src.as_bytes(),
            pos: 0,
            base: base + "root=".len(),
        };
        let n = parse_node(&mut p)?;
        p.expect_end()?;
        Some(n)
    };
    let mut floating = Vec::new();
    if !float_part.is_empty() {
        let mut off = float_base;
        for item in float_part.split(';') {
            floating.push(parse_float(item, off)?);
            off += item.len() + 1;
        }
    }
    let mut model = DockModel { root, floating };
    model.sanitize();
    Ok(model)
}

fn split_once<'a>(s: &'a str, pat: &str) -> Option<(&'a str, &'a str)> {
    let i = s.find(pat)?;
    Some((&s[..i], &s[i + pat.len()..]))
}

fn encode_node(n: &DockNode, s: &mut String) {
    use std::fmt::Write as _;
    match n {
        DockNode::Leaf(p) => {
            let _ = write!(s, "L{}", p.get());
        }
        DockNode::Tabs { tabs, active } => {
            let _ = write!(s, "T[{active}]{{");
            for (i, t) in tabs.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                let _ = write!(s, "{}", t.get());
            }
            s.push('}');
        }
        DockNode::Split {
            axis,
            frac,
            first,
            second,
        } => {
            let a = match axis {
                SplitAxis::Horizontal => 'H',
                SplitAxis::Vertical => 'V',
            };
            let _ = write!(s, "S({a},{frac:.4},");
            encode_node(first, s);
            s.push(',');
            encode_node(second, s);
            s.push(')');
        }
    }
}

fn encode_float(f: &FloatingPanel, s: &mut String) {
    use std::fmt::Write as _;
    let _ = write!(s, "F{}@{},{},{},{}", f.panel.get(), f.x, f.y, f.w, f.h);
    if let Some(w) = f.window {
        let _ = write!(s, "#{w}");
    }
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,

    base: usize,
}

impl Parser<'_> {
    fn abs(&self) -> usize {
        self.base + self.pos
    }

    fn err<T>(&self, what: &'static str) -> Result<T, DecodeError> {
        Err(DecodeError::Expected {
            what,
            pos: self.abs(),
        })
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn eat(&mut self, b: u8) -> bool {
        if self.peek() == Some(b) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, b: u8, what: &'static str) -> Result<(), DecodeError> {
        if self.eat(b) { Ok(()) } else { self.err(what) }
    }

    fn expect_end(&self) -> Result<(), DecodeError> {
        if self.pos == self.src.len() {
            Ok(())
        } else {
            Err(DecodeError::Trailing { pos: self.abs() })
        }
    }

    fn number(&mut self) -> Result<String, DecodeError> {
        let start = self.pos;
        let mut seen = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'.' || c == b'-' || c == b'+' {
                self.pos += 1;
                seen = true;
            } else {
                break;
            }
        }
        if seen {
            Ok(String::from_utf8_lossy(&self.src[start..self.pos]).into_owned())
        } else {
            self.err("liczby")
        }
    }

    fn u64_num(&mut self) -> Result<u64, DecodeError> {
        let t = self.number()?;
        t.parse::<u64>()
            .map_err(|_| DecodeError::Number { pos: self.abs() })
    }

    fn f32_num(&mut self) -> Result<f32, DecodeError> {
        let t = self.number()?;
        t.parse::<f32>()
            .map_err(|_| DecodeError::Number { pos: self.abs() })
    }
}

fn parse_node(p: &mut Parser<'_>) -> Result<DockNode, DecodeError> {
    match p.peek() {
        Some(b'L') => {
            p.pos += 1;
            Ok(DockNode::Leaf(PanelId::new(p.u64_num()?)))
        }
        Some(b'T') => {
            p.pos += 1;
            p.expect(b'[', "`[`")?;
            let active: usize = p
                .number()?
                .parse()
                .map_err(|_| DecodeError::Number { pos: p.abs() })?;
            p.expect(b']', "`]`")?;
            p.expect(b'{', "`{`")?;
            let mut tabs = Vec::new();

            tabs.push(PanelId::new(p.u64_num()?));
            while p.eat(b',') {
                tabs.push(PanelId::new(p.u64_num()?));
            }
            p.expect(b'}', "`}`")?;
            if tabs.is_empty() || active >= tabs.len() {
                return p.err("active poza zakresem");
            }
            Ok(DockNode::Tabs { tabs, active })
        }
        Some(b'S') => {
            p.pos += 1;
            p.expect(b'(', "`(`")?;
            let axis = match p.peek() {
                Some(b'H') => {
                    p.pos += 1;
                    SplitAxis::Horizontal
                }
                Some(b'V') => {
                    p.pos += 1;
                    SplitAxis::Vertical
                }
                _ => return p.err("H|V axis"),
            };
            p.expect(b',', "`,`")?;
            let frac = p.f32_num()?;
            if !frac.is_finite() {
                return p.err("frac must be finished");
            }
            p.expect(b',', "`,`")?;
            let first = parse_node(p)?;
            p.expect(b',', "`,`")?;
            let second = parse_node(p)?;
            p.expect(b')', "`)`")?;
            Ok(DockNode::Split {
                axis,
                frac: DockNode::sanitized_frac(frac),
                first: Box::new(first),
                second: Box::new(second),
            })
        }
        _ => p.err("L|T|S node"),
    }
}

fn parse_float(item: &str, base: usize) -> Result<FloatingPanel, DecodeError> {
    let body = item.strip_prefix('F').ok_or(DecodeError::Expected {
        what: "`F{id}@x,y,w,h`",
        pos: base,
    })?;
    let (id_part, rest) = split_once(body, "@").ok_or(DecodeError::Expected {
        what: "`@`",
        pos: base + 1,
    })?;
    let id_base = base + 1;
    let id: u64 = id_part
        .parse()
        .map_err(|_| DecodeError::Number { pos: id_base })?;
    let (geom, win) = match split_once(rest, "#") {
        Some((g, w)) => (g, Some(w)),
        None => (rest, None),
    };
    let geom_base = id_base + id_part.len() + 1;
    let nums: Vec<&str> = geom.split(',').collect();
    if nums.len() != 4 {
        return Err(DecodeError::Expected {
            what: "`x,y,w,h`",
            pos: geom_base,
        });
    }
    let parse = |s: &str| {
        s.parse::<f32>()
            .map_err(|_| DecodeError::Number { pos: geom_base })
    };
    let (x, y, w, h) = (
        parse(nums[0])?,
        parse(nums[1])?,
        parse(nums[2])?,
        parse(nums[3])?,
    );
    let window = match win {
        Some(w) => Some(w.parse::<u64>().map_err(|_| DecodeError::Number {
            pos: geom_base + geom.len() + 1,
        })?),
        None => None,
    };
    Ok(FloatingPanel {
        panel: PanelId::new(id),
        x,
        y,
        w,
        h,
        window,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop::DockDrop;
    use crate::model::DockModel;
    use crate::ops::DockOp;

    fn sample() -> DockModel {
        let mut m = DockModel::single(PanelId::new(1));
        m.apply(DockOp::Insert {
            panel: PanelId::new(2),
            target: PanelId::new(1),
            drop: DockDrop::Right,
            frac: 0.3,
            moving: false,
        });
        m.apply(DockOp::Insert {
            panel: PanelId::new(3),
            target: PanelId::new(2),
            drop: DockDrop::Tab,
            frac: 0.5,
            moving: false,
        });
        m.apply(DockOp::Float {
            panel: PanelId::new(3),
            x: 40.0,
            y: 40.0,
            w: 320.0,
            h: 240.0,
        });

        m.apply(DockOp::Insert {
            panel: PanelId::new(4),
            target: PanelId::new(2),
            drop: DockDrop::Tab,
            frac: 0.5,
            moving: false,
        });
        m
    }

    #[test]
    fn roundtrip() {
        let m = sample();
        let s = encode(&m);
        let back = decode(&s).expect("roundtrip");
        assert_eq!(m, back, "{s}");
    }

    #[test]
    fn empty_roundtrip() {
        let m = DockModel::new();
        assert_eq!(decode(&encode(&m)).unwrap(), m);
    }

    #[test]
    fn bad_inputs_error_not_panic() {
        assert!(decode("xxx").is_err());

        assert!(decode("dock-v1 root=L1 float=").is_ok());
        assert!(decode("dock-v1 root=L1").is_err());
        assert!(decode("dock-v1 root=S(H,0.5,L1) float=").is_err());
        assert!(decode("dock-v1 root=L1 float=F1@1,2").is_err());
        assert!(decode("dock-v1 root=T[5]{1,2} float=").is_err());
        assert!(decode("dock-v1 root=T[0]{1,2 float=").is_err());
    }

    #[test]
    fn tabs_inside_split_with_sibling_roundtrip() {
        let text = "dock-v1 root=S(H,0.72,T[0]{2,3},L5) float=";
        let back = decode(text).expect("tabs + siblings");

        let again = decode(&encode(&back)).expect("re-encode");
        assert_eq!(back, again);
        match back.root {
            Some(DockNode::Split { .. }) => {}
            other => panic!("oczekiwano splitu, jest {other:?}"),
        }
    }

    #[test]
    fn window_float_roundtrip() {
        let mut m = DockModel::single(PanelId::new(1));
        m.apply(DockOp::Float {
            panel: PanelId::new(1),
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 300.0,
        });
        m.attach_window(PanelId::new(1), Some(7));
        let back = decode(&encode(&m)).unwrap();
        assert_eq!(back.floating[0].window, Some(7));
    }
}
