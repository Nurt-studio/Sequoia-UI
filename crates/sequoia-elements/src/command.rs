use std::cell::RefCell;
use std::rc::Rc;

use sequoia_core::Length;
use sequoia_reactive::Signal;
use sequoia_view::VirtualList;
use sequoia_view::{AnyView, Column, Dismiss, DismissableLayer, FocusScope, Row, Text, Unit, View};

use crate::button::Button;
use crate::floating_menu::FloatingMenu;
use crate::modal::Modal;
use crate::text_field::TextField;

#[derive(Clone)]
pub struct Command {
    pub id: String,
    pub title: String,
    pub category: String,
    pub shortcut: Option<String>,
    action: Rc<dyn Fn()>,
}

impl Command {
    pub fn run(&self) {
        (self.action)();
    }
}

#[derive(Clone, Default)]
pub struct CommandRegistry {
    inner: Rc<RefCell<Vec<Command>>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &self,
        id: impl Into<String>,
        title: impl Into<String>,
        category: impl Into<String>,
        shortcut: Option<impl Into<String>>,
        action: impl Fn() + 'static,
    ) {
        self.inner.borrow_mut().push(Command {
            id: id.into(),
            title: title.into(),
            category: category.into(),
            shortcut: shortcut.map(|s| s.into()),
            action: Rc::new(action),
        });
    }

    pub fn run(&self, id: &str) -> bool {
        let cmds = self.inner.borrow();
        match cmds.iter().find(|c| c.id == id) {
            Some(c) => {
                c.run();
                true
            }
            None => false,
        }
    }

    pub fn search(&self, query: &str) -> Vec<Command> {
        let q = query.to_lowercase();
        self.inner
            .borrow()
            .iter()
            .filter(|c| {
                q.is_empty()
                    || c.title.to_lowercase().contains(&q)
                    || c.category.to_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }

    pub fn categories(&self) -> Vec<String> {
        let mut out = Vec::new();
        for c in self.inner.borrow().iter() {
            if !out.contains(&c.category) {
                out.push(c.category.clone());
            }
        }
        out
    }

    pub fn commands_in(&self, category: &str) -> Vec<Command> {
        self.inner
            .borrow()
            .iter()
            .filter(|c| c.category == category)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }
}

#[derive(Clone)]
pub struct MenuBar {
    registry: CommandRegistry,
}

impl MenuBar {
    pub fn new(registry: CommandRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> CommandRegistry {
        self.registry.clone()
    }
}

impl View for MenuBar {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let mut row = Row::new().gap(2.0);
        for cat in self.registry.categories() {
            let mut menu = FloatingMenu::wrap(Button::new(cat.clone()))
                .below_invoker()
                .open_on_click()
                .popup_width(240.0);
            for cmd in self.registry.commands_in(&cat) {
                let title = cmd.title.clone();
                let shortcut = cmd.shortcut.clone().unwrap_or_default();
                let action = cmd.action.clone();
                menu = menu.shortcut_row("", title, shortcut, move || action());
            }
            row = row.child(menu);
        }
        Some(vec![AnyView::new(row)])
    }

    fn layout_style(&self) -> sequoia_layout::LayoutStyle {
        sequoia_layout::LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fit,
            ..sequoia_layout::LayoutStyle::default()
        }
    }
}

const RESULT_H: f32 = 32.0;

#[derive(Clone)]
pub struct CommandPalette {
    registry: CommandRegistry,
    open: Signal<bool>,
    query: Signal<String>,
    selected: Signal<usize>,
}

impl CommandPalette {
    pub fn new(registry: CommandRegistry, open: Signal<bool>) -> Self {
        Self {
            registry,
            open,
            query: Signal::new(String::new()),
            selected: Signal::new(0),
        }
    }

    pub fn query_signal(&self) -> Signal<String> {
        self.query.clone()
    }

    pub fn is_open(&self) -> bool {
        self.open.get_untracked()
    }

    pub fn close(&self) {
        self.open.set(false);
        self.query.set(String::new());
        self.selected.set(0);
    }

    fn run_at(&self, index: usize) {
        let results = self.registry.search(&self.query.get_untracked());
        if let Some(cmd) = results.get(index) {
            cmd.run();
            self.close();
        }
    }

    fn run_id(&self, id: &str) {
        if self.registry.run(id) {
            self.close();
        }
    }
}

impl View for CommandPalette {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let this = self.clone();
        let submit_this = self.clone();
        let query = self.query.clone();
        let open = self.open.clone();
        let field = TextField::new(query)
            .placeholder("Enter command...")
            .on_submit(move |_| {
                let idx = submit_this.selected.get_untracked();
                submit_this.run_at(idx);
            });
        let content = FocusScope::new().trap(true).child(
            Column::new()
                .gap(8.0)
                .child(field)
                .child(PaletteResults { palette: this }),
        );
        let open2 = self.open.clone();
        let close_this = self.clone();
        let layer = DismissableLayer::wrap(content)
            .bind(open2)
            .on_escape(move || close_this.close());
        let modal = Modal::new(open)
            .width(520.0)
            .dismiss(Dismiss::OutsideClick)
            .child(layer);
        Some(vec![AnyView::new(modal)])
    }

    fn layout_style(&self) -> sequoia_layout::LayoutStyle {
        sequoia_layout::LayoutStyle::default()
    }
}

#[derive(Clone)]
struct PaletteResults {
    palette: CommandPalette,
}

impl View for PaletteResults {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let pal = self.palette.clone();
        let results = pal.registry.search(&pal.query.get());
        let _ = pal.selected.get();
        if results.is_empty() {
            return Some(vec![AnyView::new(Text::new("No results").size(12.0))]);
        }

        let list = VirtualList::new(move || pal.registry.search(&pal.query.get_untracked()))
            .key(|c: &Command| {
                let mut h = 0u64;
                for b in c.id.bytes() {
                    h = h.wrapping_mul(31).wrapping_add(b as u64);
                }
                h
            })
            .item({
                let pal = self.palette.clone();
                move |c: &Command| {
                    let p = pal.clone();
                    let id = c.id.clone();
                    let label = if c.shortcut.as_deref().unwrap_or("").is_empty() {
                        c.title.clone()
                    } else {
                        format!("{}  ({})", c.title, c.shortcut.clone().unwrap_or_default())
                    };
                    Button::new(label).on_click(move || p.run_id(&id))
                }
            })
            .item_height(RESULT_H);
        Some(vec![AnyView::new(list)])
    }

    fn layout_style(&self) -> sequoia_layout::LayoutStyle {
        sequoia_layout::LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..sequoia_layout::LayoutStyle::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_matches_title_and_category_case_insensitive() {
        let r = CommandRegistry::new();
        r.register("a", "Zapisz", "Plik", Some("Ctrl+S"), || {});
        r.register("b", "Open", "Plik", None::<String>, || {});
        r.register("c", "Cofnij", "Edycja", None::<String>, || {});
        assert_eq!(r.search("").len(), 3);
        assert_eq!(r.search("zap").len(), 1);
        assert_eq!(r.search("PLIK").len(), 2);
        assert_eq!(r.search("zzz").len(), 0);
    }

    #[test]
    fn run_returns_false_for_unknown_id() {
        let r = CommandRegistry::new();
        let n = Rc::new(std::cell::Cell::new(0));
        let n2 = n.clone();
        r.register("x", "X", "K", None::<String>, move || n2.set(n2.get() + 1));
        assert!(r.run("x"));
        assert_eq!(n.get(), 1);
        assert!(!r.run("nope"));
    }

    #[test]
    fn categories_keep_first_seen_order() {
        let r = CommandRegistry::new();
        r.register("1", "A", "Plik", None::<String>, || {});
        r.register("2", "B", "Edycja", None::<String>, || {});
        r.register("3", "C", "Plik", None::<String>, || {});
        assert_eq!(
            r.categories(),
            vec!["Plik".to_string(), "Edycja".to_string()]
        );
        assert_eq!(r.commands_in("Plik").len(), 2);
    }
}
