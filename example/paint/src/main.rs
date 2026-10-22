mod app;
mod assets;
mod files;
mod icons;
mod layout;
mod model;
mod raster;
mod text;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use sequoia_ui::prelude::*;

use app::{PaintApp, perform};
use model::{Action, Dialog, State};

fn accelerator(
    app: App,
    state: &Rc<RefCell<State>>,
    key: Key,
    modifiers: Modifiers,
    action: Action,
) -> App {
    let guard_state = state.clone();
    let when: Rc<dyn Fn() -> bool> = Rc::new(move || {
        let s = guard_state.borrow();
        s.text.is_none() && s.dialog.is_none()
    });
    let action_state = state.clone();
    app.accelerator_ctx_when(key, modifiers, when, move |ctx| {
        perform(&action_state, action, ctx.size);
    })
}

fn main() {
    let state = Rc::new(RefCell::new(State::new()));
    let root = PaintApp::with_state(state.clone());

    let close_state = state.clone();
    let close = move || {
        let mut s = close_state.borrow_mut();
        if s.dirty {
            s.pending = Some(Action::CloseApp);
            s.dialog = Some(Dialog::ConfirmNew);
            s.bump();
            false
        } else {
            true
        }
    };

    let mut app = App::new()
        .title("Paint")
        .size(1360.0, 860.0)
        .min_size(1100.0, 680.0)
        .theme(Theme::light())
        .decorations(false)
        .root(root)
        .on_close_request(close);

    if let Ok(scale) = std::env::var("PAINT_SCALE")
        && let Ok(scale) = scale.parse::<f64>()
    {
        app = app.scale_factor(scale);
    }
    if let Ok(pos) = std::env::var("PAINT_POS")
        && let Some((x, y)) = pos.split_once(',')
        && let (Ok(x), Ok(y)) = (x.trim().parse::<f64>(), y.trim().parse::<f64>())
    {
        app = app.position(x, y);
    }

    let ctrl = Modifiers::CONTROL;
    let ctrl_shift = Modifiers::CONTROL | Modifiers::SHIFT;
    app = accelerator(app, &state, Key::from_char('z'), ctrl, Action::Undo);
    app = accelerator(app, &state, Key::from_char('y'), ctrl, Action::Redo);
    app = accelerator(app, &state, Key::from_char('s'), ctrl, Action::Save);
    app = accelerator(app, &state, Key::from_char('S'), ctrl_shift, Action::SaveAs);
    app = accelerator(app, &state, Key::from_char('o'), ctrl, Action::Open);
    app = accelerator(app, &state, Key::from_char('n'), ctrl, Action::New);
    app = accelerator(app, &state, Key::from_char('a'), ctrl, Action::SelectAll);
    app = accelerator(app, &state, Key::from_char('c'), ctrl, Action::Copy);
    app = accelerator(app, &state, Key::from_char('x'), ctrl, Action::Cut);
    app = accelerator(app, &state, Key::from_char('v'), ctrl, Action::Paste);
    app = accelerator(app, &state, Key::DELETE, Modifiers::NONE, Action::Cut);
    app = accelerator(app, &state, Key::from_char('+'), ctrl, Action::ZoomIn);
    app = accelerator(app, &state, Key::from_char('-'), ctrl, Action::ZoomOut);

    if let Err(e) = app.run() {
        eprintln!("Paint: {e}");
    }
}
