#![allow(dead_code, unused_imports, unused_variables)]

use sequoia_ui::prelude::*;

fn hello_world_app() -> App {
    App::new().title("Moja aplikacja").size(800.0, 600.0).root(
        Column::new()
            .fill_size()
            .child(Text::new("Witaj w Sequoia"))
            .child(Button::new("Kliknij mnie").on_click(|| println!("klik!"))),
    )
}

#[derive(Clone)]
struct Counter {
    count: Signal<i32>,
}

impl View for Counter {
    fn body(&self) -> impl View + 'static {
        let count = self.count.clone();
        let dec = self.count.clone();
        let inc = self.count.clone();
        Column::new()
            .gap(8.0)
            .child(Text::with(move || format!("Licznik: {}", count.get())))
            .child(
                Row::new()
                    .gap(8.0)
                    .child(Button::new("−").on_click(move || dec.update(|c| *c -= 1)))
                    .child(Button::new("+").on_click(move || inc.update(|c| *c += 1))),
            )
    }
}

fn text_demo() -> impl View {
    Text::new("statyczny").size(16.0).color(Color::WHITE).bold()
}

fn flex_demo() -> impl View {
    Column::new()
        .fill_size()
        .gap(8.0)
        .align_items(Align::Center)
        .justify_content(Justify::Start)
        .child(Text::new("header"))
        .child_flex(Panel::new(), 1.0)
}

fn stack_demo() -> impl View {
    let open = Signal::new(true);
    Stack::new()
        .fill_size()
        .child_at(Panel::new(), StackPos::fill())
        .child_at(Modal::new(open), StackPos::fill())
        .child_at(
            Panel::new().width(40.0).height(40.0),
            StackPos::top_right().offset(-6.0, -6.0),
        )
}

fn grid_demo() -> impl View {
    Grid::new([Track::Fill(1.0), Track::Fill(1.0)])
        .rows([Track::Px(24.0), Track::Px(48.0)])
        .gap(4.0)
        .child_span(Text::new("Heading"), 0, 0, 2, 1)
}

fn dock_area_demo() -> impl View {
    DockArea::new()
        .fill_height()
        .top(Text::new("header"))
        .bottom(Text::new("footer"))
        .fill(Panel::new())
}

fn split_demo() -> impl View {
    let frac = Signal::new(0.3);
    SplitPanel::new(frac)
        .first(Panel::new())
        .second(Panel::new())
        .min_first(200.0)
        .min_second(200.0)
        .axis(Axis::Horizontal)
}

fn state_demo() {
    let count = Signal::new(0i32);
    let _v: i32 = count.get();
    count.set(5);
    count.update(|c| *c += 1);
    let _raw: i32 = count.get_untracked();
    let layers = Signal::new(vec![String::from("a")]);
    let _name = layers.with(|all| all[0].clone());

    let c1 = count.clone();
    let doubled = memo(move || c1.get() * 2);
    let _ = doubled.get();

    let c2 = count.clone();
    let sub = effect(move || {
        let _ = c2.get();
    });
    drop(sub);

    batch(|| {
        count.set(1);
    });
    sequoia_ui::reactive::transaction("demo", || {
        count.set(2);
    });
    untracked(|| {
        let _ = count.get();
    });

    let doc = UndoSignal::new(String::new(), 60);
    doc.transact_coalesced("typing", |s| s.push('a'));
    doc.set("nowa".to_string());
    let _ = doc.undo();
}

fn binding_demo() -> impl View {
    let text = Signal::new(String::new());
    let check = Signal::new(false);
    let frac = Signal::new(0.0f32);
    let text_read = text.clone();

    Column::new()
        .gap(8.0)
        .child(TextField::new(text.clone()).placeholder("Szukaj…"))
        .child(Checkbox::new(check.clone()).label("Turn on"))
        .child(Slider::new(frac.clone()).min(0.0).max(1.0))
        .child(Text::with(move || {
            format!("You entered: {}", text_read.get())
        }))
}

fn env_demo() {
    let env = Environment::new();
    env.provide(Theme::dark());
    let _t: Option<Theme> = env.get::<Theme>();
}

fn async_demo() {
    let rt = AsyncRuntime::new(4);
    let data = Signal::new(0i32);
    let out = Signal::new(0i32);
    rt.spawn_future(async { 1i32 }, move |result| data.set(result));
    rt.spawn_blocking(|| 2i32, move |v| out.set(v));
    rt.drain();
}

fn click_demo() -> impl View {
    let count = Signal::new(0);
    let c = count.clone();
    Button::new("+").on_click(move || c.update(|v| *v += 1))
}

fn accel_demo() -> App {
    let editing = Signal::new(false);
    App::new()
        .accelerator(Key::from_char('s'), Modifiers::CONTROL, || {})
        .accelerator_when(
            Key::DELETE,
            Modifiers::empty(),
            not_editing(editing.clone()),
            move || {},
        )
}

struct MojWidget;

impl View for MojWidget {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn focusable(&self) -> bool {
        true
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            return;
        }
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Left,
            ..
        }) = event
        {
            ctx.request_focus();
            ctx.capture_pointer();
            ctx.handled();
        }
    }
}

#[derive(Clone)]
struct Watcher {
    source: Signal<i32>,
    hits: Signal<u32>,
    sub: std::rc::Rc<std::cell::RefCell<Option<Effect>>>,
}

impl View for Watcher {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn on_mount(&self) {
        let source = self.source.clone();
        let hits = self.hits.clone();
        *self.sub.borrow_mut() = Some(effect(move || {
            let _ = source.get();
            hits.set(hits.get() + 1);
        }));
    }

    fn on_unmount(&self) {
        self.sub.borrow_mut().take();
    }
}

fn styling_demo() -> impl View {
    Column::new()
        .gap(8.0)
        .child(
            Button::new("OK")
                .background(Color::BLUE)
                .color(Color::WHITE)
                .radius(6.0),
        )
        .child(Button::new("OK").style_with(|s: ButtonStyle| s.background(Color::BLUE).radius(6.0)))
        .child(
            Button::new("OK")
                .opacity(0.9)
                .offset(10.0, 0.0)
                .background(Color::BLUE)
                .clip()
                .scale(1.05)
                .rotate(0.1)
                .enter(fade()),
        )
}

fn login_form() -> impl View {
    let form = Form::new();

    let email = Field::new(Signal::new(String::new()), "E-mail")
        .placeholder("jan@example.com")
        .rule("podaj poprawny e-mail", |v: &str| v.contains('@'));

    let password = Field::new(Signal::new(String::new()), "Password")
        .placeholder("••••••••")
        .rule("min. 8 characters", |v: &str| v.chars().count() >= 8);

    form.register(&email);
    form.register(&password);

    let remember = Signal::new(false);
    let form_btn = form.clone();
    let email_sig = email.value_signal();

    Column::new()
        .gap(12.0)
        .width(360.0)
        .child(Text::new("Log in").size(18.0).bold())
        .child(email.clone())
        .child(password.clone())
        .child(Checkbox::new(remember.clone()).label("Remember Me"))
        .child(form.summary())
        .child(Button::new("Zaloguj").on_click(move || {
            form_btn.touch_all();
            if form_btn.is_valid() {
                println!("login: {}", email_sig.get_untracked());
            }
        }))
}

fn contact_list() -> impl View {
    let query = Signal::new(String::new());
    let all = Signal::new(vec![
        "Ada Lovelace".to_string(),
        "Alan Turing".to_string(),
        "Grace Hopper".to_string(),
    ]);

    let query_for_filter = query.clone();
    let filtered = memo(move || {
        let q = query_for_filter.get().to_lowercase();
        all.get()
            .into_iter()
            .filter(|name| name.to_lowercase().contains(&q))
            .collect::<Vec<_>>()
    });

    Column::new()
        .fill_size()
        .gap(8.0)
        .child(TextField::new(query.clone()).placeholder("Szukaj…"))
        .child_flex(
            VirtualList::new(move || filtered.get())
                .item_height(28.0)
                .item(|name: &String| Text::new(name.clone())),
            1.0,
        )
}

fn navigation() -> impl View {
    let page = Signal::new(0usize);
    let page_for_content = page.clone();

    Column::new()
        .fill_size()
        .gap(12.0)
        .child(
            TabStrip::new(page.clone(), ["Profil", "Ustawienia", "Pomoc"])
                .on_select(move |i| page.set(i)),
        )
        .child_flex(
            Text::with(move || match page_for_content.get() {
                0 => "Widok profilu".to_string(),
                1 => "Settings view".to_string(),
                _ => "Centrum pomocy".to_string(),
            }),
            1.0,
        )
}

fn main_view() -> impl View {
    Panel::new()
}
fn delete_layer() {}

fn confirm_delete() -> impl View {
    let open = Signal::new(false);
    let dlg = Dialog::new(open.clone(), "Remove layer?")
        .content(Text::new("This operation cannot be undone."))
        .danger("Delete", {
            let open = open.clone();
            move || {
                delete_layer();
                open.set(false);
            }
        })
        .cancel("Anuluj");

    Stack::new()
        .child_at(main_view(), StackPos::fill())
        .child_at(dlg.scrim(), StackPos::fill())
        .child_at(dlg, StackPos::fill())
}

#[derive(Clone, PartialEq)]
struct Doc {
    title: String,
}
fn doc_panel() -> impl View {
    let res: Resource<Doc> = Resource::new();
    AsyncView::new(res.clone())
        .loading(|| Text::new("Landing…"))
        .data(|d: &Doc| AnyView::new(Text::new(d.title.clone())))
        .error(|e: &str| Text::new(format!("Error: {e}")))
        .retry("Try again", {
            let res = res.clone();
            move || {
                res.begin_loading();
            }
        })
}

#[derive(Clone, PartialEq)]
struct Person {
    id: u64,
    name: String,
}

fn people_table() -> impl View {
    let rows = Signal::new(vec![Person {
        id: 1,
        name: "Ada".to_string(),
    }]);
    Table::new({
        let rows = rows.clone();
        move || rows.get()
    })
    .key(|p: &Person| p.id)
    .column(
        TableColumn::new("Nazwa")
            .sort_key(|a: &Person, b: &Person| a.name.cmp(&b.name))
            .matches(|p: &Person, q: &str| p.name.contains(q))
            .text_cell(|p: &Person| p.name.clone()),
    )
    .row_height(28.0)
}

fn components_smoke() -> impl View {
    let b = Signal::new(false);
    let f = Signal::new(0.0f32);
    let s = Signal::new(String::new());
    let i = Signal::new(0usize);
    let v = Signal::new(Vec::<usize>::new());
    let open = Signal::new(false);

    Column::new()
        .gap(4.0)
        .child(Button::new("B"))
        .child(Checkbox::new(b.clone()).label("c"))
        .child(Switch::new(b.clone()).label("s"))
        .child(Slider::new(f.clone()))
        .child(Slider1D::new(f.clone(), 0.0, 360.0))
        .child(SvPad::new(f.clone(), f.clone(), f.clone()))
        .child(TextField::new(s.clone()))
        .child(TextArea::new(s.clone()).min_lines(4))
        .child(DragValue::new(f.clone()))
        .child(TabStrip::new(i.clone(), ["A", "B"]))
        .child(DropdownButton::new(i.clone(), vec!["x", "y"]))
        .child(ProgressBar::new(f.clone()))
        .child(Spinner::new())
        .child(Skeleton::new().width(100.0).height(16.0))
        .child(Container::new().child(Text::new("click")).on_click(|| {}))
        .child(Modal::new(open.clone()))
        .child(Sheet::new(open.clone()).side(SheetSide::Bottom))
        .child(Drawer::new(open.clone()).side(DrawerSide::Left))
        .child(Tooltip::new(Text::new("t"), "tip"))
        .child(Avatar::named("Ada Lovelace"))
        .child(Badge::new("Nowe"))
        .child(Chip::new("tag").on_remove(|| {}))
        .child(Divider::horizontal())
        .child(Kbd::new("Ctrl+S"))
        .child(ColorPicker::new().floating())
        .child(
            NumberField::new(f.clone(), "Quantity")
                .min(0.0)
                .max(10.0)
                .step(1.0),
        )
        .child(DateField::new(s.clone(), "Data"))
        .child(MultiSelect::new(vec!["a".into(), "b".into()], "Tagi").min_selected(0))
        .child(ToastStack::new(ToastQueue::new()))
        .child(ScrollView::new(Text::new("scroll")))
        .child(List::new(|| vec![1, 2, 3]).item(|n: &i32| Text::new(n.to_string())))
        .child(
            TreeView::new(|| vec![TreeNode::leaf("root", "root".to_string())])
                .row(|_v: &String, _d| Text::new("")),
        )
        .child(Canvas::new())
        .child(Image::file("icon.png"))
        .child(Icon::new("save"))
        .child(Wrap::new().gap(4.0).child(Text::new("w")))
        .child(RichText::new(vec![TextSpan::new("hi")]))
        .child(
            ErrorBoundary::new(|| AnyView::new(Text::new("safe"))).fallback(|_e| Text::new("err")),
        )
        .child(Fragment::new().child(Text::new("frag")))
        .child(Spacer::flex(1.0))
        .child(FocusScope::new().trap(true).child(Text::new("focus")))
        .child(Container::new().child(Text::new("box")))
}

#[test]
fn all_documented_examples_compile_and_build() {
    let _ = hello_world_app();
    let _ = Counter {
        count: Signal::new(0),
    }
    .body();
    let _ = text_demo();
    let _ = flex_demo();
    let _ = stack_demo();
    let _ = grid_demo();
    let _ = dock_area_demo();
    let _ = split_demo();
    state_demo();
    let _ = binding_demo();
    env_demo();
    async_demo();
    let _ = click_demo();
    let _ = accel_demo();
    let _ = MojWidget.body();
    let _ = Watcher {
        source: Signal::new(0),
        hits: Signal::new(0),
        sub: Default::default(),
    }
    .body();
    let _ = styling_demo();
    let _ = login_form();
    let _ = contact_list();
    let _ = navigation();
    let _ = confirm_delete();
    let _ = doc_panel();
    let _ = people_table();
    let _ = components_smoke();
}
