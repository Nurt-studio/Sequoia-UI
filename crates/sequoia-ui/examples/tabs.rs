use sequoia_ui::prelude::*;

const APP_BG: Color = Color::hex(0xF1F4F9);
const SURFACE: Color = Color::hex(0xFFFFFF);
const SURFACE_ALT: Color = Color::hex(0xF7F9FC);
const BORDER: Color = Color::hex(0xDDE3EC);
const TEXT: Color = Color::hex(0x172033);
const MUTED: Color = Color::hex(0x64748B);
const ACCENT: Color = Color::hex(0x2563EB);
const GREEN: Color = Color::hex(0x059669);

fn stat(label: &str, value: &str, color: Color) -> AnyView {
    AnyView::new(
        Container::new()
            .flex(1.0)
            .padding(16.0)
            .background(SURFACE_ALT)
            .radius(12.0)
            .border(1.0, BORDER)
            .child(
                Column::new()
                    .gap(4.0)
                    .child(Text::new(label).size(12.0).color(MUTED))
                    .child(Text::new(value).size(26.0).bold().color(color)),
            ),
    )
}

fn page(title: &str, body: &str) -> AnyView {
    AnyView::new(
        Column::new()
            .fill_width()
            .gap(10.0)
            .child(Text::new(title).size(24.0).bold().color(TEXT))
            .child(Text::new(body).size(14.0).color(MUTED).wrap()),
    )
}

struct TabBody {
    active: Signal<usize>,
    notifications: Signal<bool>,
    compact: Signal<bool>,
}

impl View for TabBody {
    fn body(&self) -> impl View + 'static {
        let content = match self.active.get() {
            0 => AnyView::new(
                Column::new()
                    .fill_width()
                    .gap(16.0)
                    .child(page(
                        "Overview",
                        "The overview tab reads the active index and rebuilds its body when it changes.",
                    ))
                    .child(
                        Row::new()
                            .fill_width()
                            .gap(12.0)
                            .child(stat("Deploys", "128", ACCENT))
                            .child(stat("Success rate", "99.2%", GREEN))
                            .child(stat("Build time", "2m 14s", TEXT)),
                    ),
            ),
            1 => AnyView::new(
                Column::new()
                    .fill_width()
                    .gap(12.0)
                    .child(page(
                        "Activity",
                        "Every row below is a plain view inside the tab body.",
                    ))
                    .child(
                        Container::new()
                            .fill_width()
                            .padding(16.0)
                            .background(SURFACE_ALT)
                            .radius(12.0)
                            .child(
                                Column::new()
                                    .gap(8.0)
                                    .child(Text::new("Deploy finished on main").size(14.0))
                                    .child(Text::new("Cache warmed in 0.8s").size(14.0))
                                    .child(Text::new("Two new assets registered").size(14.0)),
                            ),
                    ),
            ),
            _ => AnyView::new(
                Column::new()
                    .fill_width()
                    .gap(12.0)
                    .child(page(
                        "Settings",
                        "Switch tabs with the strip above. The active index is a single signal.",
                    ))
                    .child(Checkbox::new(self.notifications.clone()).label("Enable notifications"))
                    .child(Checkbox::new(self.compact.clone()).label("Compact rows"))
                    .child(
                        Button::new("Save settings")
                            .style(ButtonStyle::PRIMARY)
                            .on_click(|| println!("settings saved")),
                    ),
            ),
        };

        Container::new().fill_size().padding(24.0).child(content)
    }
}

fn main() {
    let active = Signal::new(0usize);
    let notifications = Signal::new(true);
    let compact = Signal::new(false);

    let tabs = TabStrip::new(active.clone(), ["Overview", "Activity", "Settings"])
        .height(40.0)
        .equal_width(true)
        .gap(4.0)
        .accent(ACCENT);

    let card = Container::new()
        .width(640.0)
        .height(480.0)
        .background(SURFACE)
        .radius(16.0)
        .border(1.0, BORDER)
        .child(
            Column::new()
                .fill_size()
                .child(Container::new().padding_xy(12.0, 8.0).child(tabs))
                .child(Divider::horizontal())
                .child(Container::new().flex(1.0).child(TabBody {
                    active,
                    notifications,
                    compact,
                })),
        );

    App::new()
        .title("Tabs")
        .size(760.0, 620.0)
        .root(
            Stack::new()
                .fill_size()
                .background(APP_BG)
                .padding(24.0)
                .child(card),
        )
        .run()
        .expect("failed to run the tabs example");
}
