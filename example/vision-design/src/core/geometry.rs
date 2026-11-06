//! Pure geometry: grid, handles and resize math.

/// Grid size in world units.
pub(crate) const GRID_SIZE: f32 = 24.0;

/// Selection handle size in screen pixels.
pub(crate) const HANDLE_PX: f32 = 7.0;

/// Handle hit tolerance in screen pixels.
pub(crate) const HANDLE_TOL_PX: f32 = 6.0;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Handle {
    Nw,
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
}

pub(crate) fn snap(v: f32) -> f32 {
    (v / GRID_SIZE).round() * GRID_SIZE
}

pub(crate) fn apply_resize(
    h: Handle,
    x: f32,
    y: f32,
    w: f32,
    hh: f32,
    dx: f32,
    dy: f32,
    snap_on: bool,
) -> (f32, f32, f32, f32) {
    let (mut nx, mut ny, mut nw, mut nh) = (x, y, w, hh);
    match h {
        Handle::Nw => {
            nx = x + dx;
            ny = y + dy;
            nw = w - dx;
            nh = hh - dy;
        }
        Handle::N => {
            ny = y + dy;
            nh = hh - dy;
        }
        Handle::Ne => {
            ny = y + dy;
            nw = w + dx;
            nh = hh - dy;
        }
        Handle::E => {
            nw = w + dx;
        }
        Handle::Se => {
            nw = w + dx;
            nh = hh + dy;
        }
        Handle::S => {
            nh = hh + dy;
        }
        Handle::Sw => {
            nx = x + dx;
            nw = w - dx;
            nh = hh + dy;
        }
        Handle::W => {
            nx = x + dx;
            nw = w - dx;
        }
    }
    if snap_on {
        let right = nx + nw;
        let bottom = ny + nh;
        match h {
            Handle::Nw => {
                nx = snap(nx);
                ny = snap(ny);
                nw = right - nx;
                nh = bottom - ny;
            }
            Handle::N => {
                ny = snap(ny);
                nh = bottom - ny;
            }
            Handle::Ne => {
                ny = snap(ny);
                nw = snap(right) - nx;
                nh = bottom - ny;
            }
            Handle::E => {
                nw = snap(right) - nx;
            }
            Handle::Se => {
                nw = snap(right) - nx;
                nh = snap(bottom) - ny;
            }
            Handle::S => {
                nh = snap(bottom) - ny;
            }
            Handle::Sw => {
                nx = snap(nx);
                nw = right - nx;
                nh = snap(bottom) - ny;
            }
            Handle::W => {
                nx = snap(nx);
                nw = right - nx;
            }
        }
    }
    (nx, ny, nw.max(4.0), nh.max(4.0))
}
