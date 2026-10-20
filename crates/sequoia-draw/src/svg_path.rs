use sequoia_core::Point;

use crate::path::{Path, PathError, PathSeg};

struct Parser<'a> {
    src: &'a [u8],
    i: usize,
    path: Path,
    start: Point,
    current: Point,

    last_cubic_ctrl: Option<Point>,

    last_quad_ctrl: Option<Point>,

    last_cmd: u8,
}

pub(crate) fn parse_svg_path(data: &str) -> Result<Path, PathError> {
    let mut p = Parser {
        src: data.as_bytes(),
        i: 0,
        path: Path::new(),
        start: Point::ZERO,
        current: Point::ZERO,
        last_cubic_ctrl: None,
        last_quad_ctrl: None,
        last_cmd: b'M',
    };
    p.run()?;
    Ok(p.path)
}

impl Parser<'_> {
    fn err(&self, message: &'static str) -> PathError {
        PathError {
            position: self.i,
            message,
        }
    }

    fn skip_sep(&mut self) {
        while self.i < self.src.len() {
            match self.src[self.i] {
                b' ' | b'\t' | b'\n' | b'\r' | b',' => self.i += 1,
                _ => break,
            }
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_sep();
        self.src.get(self.i).copied()
    }

    fn number(&mut self) -> Result<f32, PathError> {
        self.skip_sep();
        let start = self.i;
        if matches!(self.src.get(self.i), Some(b'+') | Some(b'-')) {
            self.i += 1;
        }
        let mut digits = false;
        while matches!(self.src.get(self.i), Some(b'0'..=b'9')) {
            self.i += 1;
            digits = true;
        }
        if self.src.get(self.i) == Some(&b'.') {
            self.i += 1;
            while matches!(self.src.get(self.i), Some(b'0'..=b'9')) {
                self.i += 1;
                digits = true;
            }
        }
        if !digits {
            return Err(PathError {
                position: start,
                message: "expected a number",
            });
        }

        if matches!(self.src.get(self.i), Some(b'e') | Some(b'E')) {
            let save = self.i;
            self.i += 1;
            if matches!(self.src.get(self.i), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            let mut exp_digits = false;
            while matches!(self.src.get(self.i), Some(b'0'..=b'9')) {
                self.i += 1;
                exp_digits = true;
            }
            if !exp_digits {
                self.i = save;
            }
        }
        let text = std::str::from_utf8(&self.src[start..self.i]).unwrap_or("");
        text.parse::<f32>().map_err(|_| PathError {
            position: start,
            message: "invalid number",
        })
    }

    fn point(&mut self) -> Result<Point, PathError> {
        let x = self.number()?;
        let y = self.number()?;
        Ok(Point::new(x, y))
    }

    fn flag(&mut self) -> Result<bool, PathError> {
        match self.peek() {
            Some(b'0') => {
                self.i += 1;
                Ok(false)
            }
            Some(b'1') => {
                self.i += 1;
                Ok(true)
            }
            _ => Err(self.err("expected a 0/1 flag")),
        }
    }

    fn run(&mut self) -> Result<(), PathError> {
        loop {
            self.skip_sep();
            if self.i >= self.src.len() {
                return Ok(());
            }
            let c = self.src[self.i];
            let absolute = c.is_ascii_uppercase();
            let cmd = c.to_ascii_lowercase();
            if !matches!(
                cmd,
                b'm' | b'l' | b'h' | b'v' | b'c' | b's' | b'q' | b't' | b'a' | b'z'
            ) {
                return Err(self.err("unknown path command"));
            }
            self.i += 1;
            self.command(cmd, absolute, c)?;
            self.last_cmd = cmd;
        }
    }

    fn command(&mut self, cmd: u8, absolute: bool, raw: u8) -> Result<(), PathError> {
        let base = if absolute { Point::ZERO } else { self.current };
        match cmd {
            b'm' => {
                let target = self.point()?;
                let p = Point::new(base.x + target.x, base.y + target.y);
                self.path.move_to(p);
                self.start = p;
                self.current = p;

                loop {
                    match self.peek() {
                        Some(b) if b.is_ascii_digit() || b == b'-' || b == b'+' || b == b'.' => {
                            let t = self.point()?;
                            let p = Point::new(
                                if absolute { 0.0 } else { self.current.x } + t.x,
                                if absolute { 0.0 } else { self.current.y } + t.y,
                            );
                            self.path.line_to(p);
                            self.current = p;
                        }
                        _ => break,
                    }
                }
            }
            b'l' => loop {
                let p = self.relative_point(absolute)?;
                self.path.line_to(p);
                self.current = p;
                if !self.more_numbers() {
                    return Ok(());
                }
            },
            b'h' => loop {
                let x = self.number()?;
                let p = Point::new(
                    if absolute { x } else { self.current.x + x },
                    self.current.y,
                );
                self.path.line_to(p);
                self.current = p;
                if !self.more_numbers() {
                    return Ok(());
                }
            },
            b'v' => loop {
                let y = self.number()?;
                let p = Point::new(
                    self.current.x,
                    if absolute { y } else { self.current.y + y },
                );
                self.path.line_to(p);
                self.current = p;
                if !self.more_numbers() {
                    return Ok(());
                }
            },
            b'c' => loop {
                let c1 = self.relative_point(absolute)?;
                let c2 = self.relative_point(absolute)?;
                let p = self.relative_point(absolute)?;
                self.path.cubic_to(c1, c2, p);
                self.current = p;
                self.last_cubic_ctrl = Some(c2);
                self.last_quad_ctrl = None;
                if !self.more_numbers() {
                    return Ok(());
                }
            },
            b's' => loop {
                let c1 = self
                    .last_cubic_ctrl
                    .map(|c| Point::new(2.0 * self.current.x - c.x, 2.0 * self.current.y - c.y))
                    .unwrap_or(self.current);
                let c2 = self.relative_point(absolute)?;
                let p = self.relative_point(absolute)?;
                self.path.cubic_to(c1, c2, p);
                self.current = p;
                self.last_cubic_ctrl = Some(c2);
                self.last_quad_ctrl = None;
                if !self.more_numbers() {
                    return Ok(());
                }
            },
            b'q' => loop {
                let c = self.relative_point(absolute)?;
                let p = self.relative_point(absolute)?;
                self.path.quad_to(c, p);
                self.current = p;
                self.last_quad_ctrl = Some(c);
                self.last_cubic_ctrl = None;
                if !self.more_numbers() {
                    return Ok(());
                }
            },
            b't' => loop {
                let c = self
                    .last_quad_ctrl
                    .map(|c| Point::new(2.0 * self.current.x - c.x, 2.0 * self.current.y - c.y))
                    .unwrap_or(self.current);
                let p = self.relative_point(absolute)?;
                self.path.quad_to(c, p);
                self.current = p;
                self.last_quad_ctrl = Some(c);
                self.last_cubic_ctrl = None;
                if !self.more_numbers() {
                    return Ok(());
                }
            },
            b'a' => loop {
                let rx = self.number()?;
                let ry = self.number()?;
                let rotation = self.number()?;
                let large = self.flag()?;
                let sweep = self.flag()?;
                let target = self.relative_point(absolute)?;
                arc_to_cubics(
                    &mut self.path,
                    self.current,
                    rx,
                    ry,
                    rotation,
                    large,
                    sweep,
                    target,
                );
                self.current = target;
                self.last_cubic_ctrl = None;
                self.last_quad_ctrl = None;
                if !self.more_numbers() {
                    return Ok(());
                }
            },
            b'z' => {
                self.path.close();
                self.current = self.start;
                self.last_cubic_ctrl = None;
                self.last_quad_ctrl = None;
                return Ok(());
            }
            _ => return Err(self.err("unknown path command")),
        }
        let _ = raw;
        Ok(())
    }

    fn relative_point(&mut self, absolute: bool) -> Result<Point, PathError> {
        let p = self.point()?;
        Ok(if absolute {
            p
        } else {
            Point::new(self.current.x + p.x, self.current.y + p.y)
        })
    }

    fn more_numbers(&mut self) -> bool {
        matches!(
            self.peek(),
            Some(b'0'..=b'9') | Some(b'-') | Some(b'+') | Some(b'.')
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn arc_to_cubics(
    path: &mut Path,
    from: Point,
    rx: f32,
    ry: f32,
    rotation_deg: f32,
    large_arc: bool,
    sweep: bool,
    to: Point,
) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx <= 0.0 || ry <= 0.0 || from == to {
        path.line_to(to);
        return;
    }
    let phi = rotation_deg.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let dx2 = (from.x - to.x) * 0.5;
    let dy2 = (from.y - to.y) * 0.5;
    let x1p = cos_phi * dx2 + sin_phi * dy2;
    let y1p = -sin_phi * dx2 + cos_phi * dy2;

    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let denom = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let mut coef = ((rx * rx * ry * ry - denom) / denom).max(0.0).sqrt();
    if large_arc == sweep {
        coef = -coef;
    }
    let cxp = coef * rx * y1p / ry;
    let cyp = -coef * ry * x1p / rx;
    let cx = cos_phi * cxp - sin_phi * cyp + (from.x + to.x) * 0.5;
    let cy = sin_phi * cxp + cos_phi * cyp + (from.y + to.y) * 0.5;

    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let dot = ux * vx + uy * vy;
        let len = ((ux * ux + uy * uy) * (vx * vx + vy * vy)).sqrt();
        let mut a = (dot / len.max(f32::MIN_POSITIVE)).clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 {
            a = -a;
        }
        a
    };
    let ux = (x1p - cxp) / rx;
    let uy = (y1p - cyp) / ry;
    let vx = (-x1p - cxp) / rx;
    let vy = (-y1p - cyp) / ry;
    let theta0 = angle(1.0, 0.0, ux, uy);
    let mut sweep_angle = angle(ux, uy, vx, vy);
    if !sweep && sweep_angle > 0.0 {
        sweep_angle -= std::f32::consts::TAU;
    } else if sweep && sweep_angle < 0.0 {
        sweep_angle += std::f32::consts::TAU;
    }

    let steps = (sweep_angle.abs() / std::f32::consts::FRAC_PI_2)
        .ceil()
        .max(1.0) as usize;
    let step = sweep_angle / steps as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    for i in 0..steps {
        let a0 = theta0 + step * i as f32;
        let a1 = a0 + step;
        let p0 = ellipse_point(cx, cy, rx, ry, sin_phi, cos_phi, a0);
        let p1 = ellipse_point(cx, cy, rx, ry, sin_phi, cos_phi, a1);
        let d0 = ellipse_derivative(rx, ry, sin_phi, cos_phi, a0);
        let d1 = ellipse_derivative(rx, ry, sin_phi, cos_phi, a1);
        let c1 = Point::new(p0.x + k * d0.x, p0.y + k * d0.y);
        let c2 = Point::new(p1.x - k * d1.x, p1.y - k * d1.y);
        if i == 0 {
            path.line_to(p0);
        }
        path.cubic_to(c1, c2, p1);
    }
}

fn ellipse_point(
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    sin_phi: f32,
    cos_phi: f32,
    angle: f32,
) -> Point {
    let (s, c) = angle.sin_cos();
    Point::new(
        cx + cos_phi * rx * c - sin_phi * ry * s,
        cy + sin_phi * rx * c + cos_phi * ry * s,
    )
}

fn ellipse_derivative(rx: f32, ry: f32, sin_phi: f32, cos_phi: f32, angle: f32) -> Point {
    let (s, c) = angle.sin_cos();
    Point::new(
        -cos_phi * rx * s - sin_phi * ry * c,
        -sin_phi * rx * s + cos_phi * ry * c,
    )
}

pub(crate) fn segs_to_debug(path: &Path) -> String {
    let mut out = String::new();
    for seg in path.segs() {
        match *seg {
            PathSeg::Move(p) => out.push_str(&format!("M {:.2} {:.2} ", p.x, p.y)),
            PathSeg::Line(p) => out.push_str(&format!("L {:.2} {:.2} ", p.x, p.y)),
            PathSeg::Quad(c, p) => {
                out.push_str(&format!("Q {:.2} {:.2} {:.2} {:.2} ", c.x, c.y, p.x, p.y))
            }
            PathSeg::Cubic(c1, c2, p) => out.push_str(&format!(
                "C {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} ",
                c1.x, c1.y, c2.x, c2.y, p.x, p.y
            )),
            PathSeg::Close => out.push_str("Z "),
        }
    }
    out.trim_end().to_string()
}
