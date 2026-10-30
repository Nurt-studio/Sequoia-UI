use std::time::Duration;

use sequoia_ui::core::{Fill, GradientStops};
use sequoia_ui::prelude::*;

const APP_BG: Color = Color::hex(0xF1F4F9);
const SURFACE: Color = Color::hex(0xFFFFFF);
const SURFACE_ALT: Color = Color::hex(0xF7F9FC);
const BORDER: Color = Color::hex(0xDDE3EC);
const TEXT: Color = Color::hex(0x172033);
const MUTED: Color = Color::hex(0x64748B);
const FAINT: Color = Color::hex(0x94A3B8);
const ACCENT: Color = Color::hex(0x2563EB);
const ACCENT_HOVER: Color = Color::hex(0x3B82F6);
const ACCENT_PRESSED: Color = Color::hex(0x1D4ED8);
const ACCENT_SOFT: Color = Color::hex(0xDBEAFE);
const GREEN: Color = Color::hex(0x059669);
const AMBER: Color = Color::hex(0xD97706);
const ROSE: Color = Color::hex(0xE11D48);
const VIOLET: Color = Color::hex(0x7C3AED);
const SHADOW: Color = Color::rgba(0.07, 0.11, 0.20, 0.10);

const BTN_PRIMARY: ButtonStyle = ButtonStyle::new()
    .background(ACCENT)
    .hover(ACCENT_HOVER)
    .pressed(ACCENT_PRESSED)
    .text(Color::WHITE)
    .border(1.0, ACCENT_PRESSED)
    .radius(10.0)
    .padding_xy(16.0, 9.0)
    .font_size(12.5)
    .font_weight(600);

const BTN_SECONDARY: ButtonStyle = ButtonStyle::new()
    .background(SURFACE)
    .hover(Color::hex(0xEFF4FE))
    .pressed(Color::hex(0xE0E9FB))
    .text(TEXT)
    .border(1.0, BORDER)
    .radius(10.0)
    .padding_xy(14.0, 9.0)
    .font_size(12.5);

const BTN_GHOST: ButtonStyle = ButtonStyle::new()
    .background(Color::TRANSPARENT)
    .hover(ACCENT_SOFT)
    .pressed(Color::hex(0xBFDBFE))
    .text(ACCENT)
    .radius(10.0)
    .padding_xy(14.0, 9.0)
    .font_size(12.5);

const BTN_DANGER: ButtonStyle = ButtonStyle::new()
    .background(Color::hex(0xFEE2E2))
    .hover(Color::hex(0xFECACA))
    .pressed(Color::hex(0xFCA5A5))
    .text(Color::hex(0xB91C1C))
    .border(1.0, Color::hex(0xFCA5A5))
    .radius(10.0)
    .padding_xy(14.0, 9.0)
    .font_size(12.5);

const STEP_BTN: ButtonStyle = ButtonStyle::new()
    .background(SURFACE_ALT)
    .hover(Color::hex(0xE8EEF9))
    .pressed(Color::hex(0xDDE7F7))
    .text(TEXT)
    .border(1.0, BORDER)
    .radius(8.0)
    .padding_xy(8.0, 5.0)
    .font_size(12.0)
    .font_weight(600);

fn main() {
    let modal_open = Signal::new(false);
    let dialog_open = Signal::new(false);
    let drawer_open = Signal::new(false);
    let toasts = ToastQueue::new();

    let save_dialog = Dialog::new(dialog_open.clone(), "Save changes?")
        .content(
            Text::new(
                "This window shows a dialog, action buttons, modal background and automatic closing.",
            )
            .size(13.0)
            .color(MUTED)
            .wrap(),
        )
        .action("Save", {
            let open = dialog_open.clone();
            move || open.set(false)
        })
        .cancel("Cancel");
    let save_scrim = save_dialog
        .scrim()
        .scrim_color(Color::rgba(0.10, 0.14, 0.22, 0.18));

    let root = Stack::new()
        .fill_size()
        .background(APP_BG)
        .child_at(
            shell(
                modal_open.clone(),
                dialog_open.clone(),
                drawer_open.clone(),
                toasts.clone(),
            ),
            StackPos::fill(),
        )
        .child_at(
            Modal::new(modal_open.clone())
                .scrim()
                .scrim_color(Color::rgba(0.10, 0.14, 0.22, 0.20)),
            StackPos::fill(),
        )
        .child_at(modal(modal_open), StackPos::fill())
        .child_at(save_scrim, StackPos::fill())
        .child_at(save_dialog, StackPos::fill())
        .child_at(
            Drawer::new(drawer_open.clone())
                .scrim()
                .scrim_color(Color::rgba(0.10, 0.14, 0.22, 0.20)),
            StackPos::fill(),
        )
        .child_at(drawer(drawer_open), StackPos::fill())
        .child_at(ToastStack::new(toasts), StackPos::fill());

    App::new()
        .title("Sequoia UI - full component gallery")
        .size(1440.0, 920.0)
        .theme(Theme::light())
        .root(root)
        .run()
        .expect("event loop failed");
}

fn shell(
    modal_open: Signal<bool>,
    dialog_open: Signal<bool>,
    drawer_open: Signal<bool>,
    toasts: ToastQueue,
) -> DockArea {
    DockArea::new()
        .fill_size()
        .background(APP_BG)
        .padding(18.0)
        .gap(14.0)
        .top(header(
            modal_open.clone(),
            dialog_open.clone(),
            drawer_open.clone(),
            toasts.clone(),
        ))
        .left(sidebar())
        .bottom(footer())
        .fill(
            ScrollView::new(content(modal_open, dialog_open, drawer_open, toasts))
                .scrollbar_width(10.0)
                .scrollbar_track(Color::TRANSPARENT)
                .scrollbar_thumb(Color::hex(0xB8C4D6))
                .scroll_step(54.0),
        )
}

fn header(
    modal_open: Signal<bool>,
    dialog_open: Signal<bool>,
    drawer_open: Signal<bool>,
    toasts: ToastQueue,
) -> Row {
    let modal_click = modal_open.clone();
    let dialog_click = dialog_open.clone();
    let drawer_click = drawer_open.clone();
    let toast_click = toasts.clone();

    Row::new()
        .fill_width()
        .align_items(Align::Center)
        .gap(12.0)
        .child(
            Column::new()
                .gap(3.0)
                .child(
                    Text::new("Sequoia UI")
                        .size(25.0)
                        .bold()
                        .color(TEXT)
                        .truncate_fill(),
                )
                .child(
                    Text::new("Light theme, all components and effects in one place")
                        .size(12.0)
                        .color(MUTED)
                        .truncate_fill(),
                ),
        )
        .child(Spacer::flex(1.0))
        .child(
            Chip::new("Theme::light()")
                .background(ACCENT_SOFT)
                .border(Color::hex(0xBFDBFE))
                .text_color(ACCENT)
                .font_size(11.0),
        )
        .child(
            Button::labeled("Toast")
                .style(BTN_SECONDARY)
                .on_click(move || {
                    toast_click.success("Done", "Toast, queue and animations work.");
                }),
        )
        .child(
            Button::labeled("Dialog")
                .style(BTN_SECONDARY)
                .on_click(move || dialog_click.set(true)),
        )
        .child(
            Button::labeled("Modal")
                .style(BTN_SECONDARY)
                .on_click(move || modal_click.set(true)),
        )
        .child(
            Button::labeled("Panel")
                .style(BTN_PRIMARY)
                .on_click(move || drawer_click.set(true)),
        )
}

fn sidebar() -> Container {
    Container::new()
        .width(222.0)
        .fill_height()
        .padding(16.0)
        .box_style(surface_style())
        .child(
            Column::new()
                .fill_size()
                .gap(12.0)
                .child(small_label("NAVIGATION"))
                .child(nav_row("Overview", ACCENT, true))
                .child(nav_row("Forms", GREEN, false))
                .child(nav_row("Data & tables", AMBER, false))
                .child(nav_row("Overlays", ROSE, false))
                .child(nav_row("Effects", VIOLET, false))
                .child(Divider::horizontal().color(BORDER).margin_xy(0.0, 4.0))
                .child(small_label("PALETTE"))
                .child(palette())
                .child(Spacer::flex(1.0))
                .child(
                    Container::new()
                        .fill_width()
                        .padding(12.0)
                        .background(SURFACE_ALT)
                        .border(1.0, BORDER)
                        .radius(12.0)
                        .child(
                            Column::new()
                                .gap(8.0)
                                .child(
                                    Row::new()
                                        .align_items(Align::Center)
                                        .gap(8.0)
                                        .child(
                                            Pulse::new(
                                                Container::new()
                                                    .square(9.0)
                                                    .radius(5.0)
                                                    .background(GREEN),
                                            )
                                            .period_duration(Duration::from_secs_f32(1.6)),
                                        )
                                        .child(
                                            Text::new("Renderer active").size(11.0).color(TEXT),
                                        ),
                                )
                                .child(
                                    Text::new(
                                        "Light background, shadows, gradients and blur are visible immediately.",
                                    )
                                    .size(11.0)
                                    .color(MUTED)
                                    .wrap(),
                                )
                                .child(
                                    ProgressBar::new(Signal::new(0.82))
                                        .height(7.0)
                                        .track(Color::hex(0xE2E8F0))
                                        .fill(GREEN),
                                ),
                        ),
                ),
        )
}

fn footer() -> Row {
    Row::new()
        .fill_width()
        .align_items(Align::Center)
        .gap(10.0)
        .child(
            Text::new(
                "Scroll the gallery with the mouse wheel. Click buttons, fields, tabs and menus.",
            )
            .size(11.0)
            .color(MUTED)
            .truncate_fill(),
        )
        .child(Spacer::flex(1.0))
        .child(
            Kbd::new("Ctrl")
                .background(SURFACE)
                .border(BORDER)
                .text_color(MUTED),
        )
        .child(
            Kbd::new("K")
                .background(SURFACE)
                .border(BORDER)
                .text_color(MUTED),
        )
        .child(Text::new("gallery").size(11.0).color(FAINT))
}

fn content(
    modal_open: Signal<bool>,
    dialog_open: Signal<bool>,
    drawer_open: Signal<bool>,
    toasts: ToastQueue,
) -> Column {
    Column::new()
        .fill_width()
        .align_items(Align::Stretch)
        .gap(18.0)
        .padding_xy(4.0, 4.0)
        .child(hero(
            modal_open.clone(),
            dialog_open.clone(),
            toasts.clone(),
        ))
        .child(
            Row::new()
                .align_items(Align::Stretch)
                .gap(18.0)
                .child_flex(card("Buttons", "ACTIONS", buttons_body()), 1.0)
                .child_flex(card("Text fields", "FORMS", inputs_body()), 1.45),
        )
        .child(
            Row::new()
                .align_items(Align::Stretch)
                .gap(18.0)
                .child_flex(card("Selection", "CONTROLS", selection_body()), 1.0)
                .child_flex(
                    card("Status and feedback", "COMPONENTS", feedback_body()),
                    1.0,
                ),
        )
        .child(
            Row::new()
                .align_items(Align::Stretch)
                .gap(18.0)
                .child_flex(card("Table", "DATA", table_body()), 1.7)
                .child_flex(card("Tree", "NAVIGATION", tree_body()), 1.0),
        )
        .child(
            Row::new()
                .align_items(Align::Stretch)
                .gap(18.0)
                .child_flex(card("Tabs", "NAVIGATION", tabs_body()), 1.0)
                .child_flex(
                    card(
                        "Menu and tooltips",
                        "OVERLAYS",
                        overlays_body(modal_open, drawer_open),
                    ),
                    1.0,
                ),
        )
        .child(card(
            "Shadows, gradient and blur",
            "EFFECTS",
            effects_body(),
        ))
        .child(card("Virtual list", "PERFORMANCE", virtual_list_body()))
        .child(
            Row::new()
                .align_items(Align::Center)
                .gap(10.0)
                .child(
                    Button::labeled("Show success toast")
                        .style(BTN_PRIMARY)
                        .on_click({
                            let toasts = toasts.clone();
                            move || {
                                toasts.success("Saved", "All components were mounted correctly.");
                            }
                        }),
                )
                .child(
                    Button::labeled("Show warning")
                        .style(BTN_SECONDARY)
                        .on_click({
                            let toasts = toasts.clone();
                            move || {
                                toasts.warn("Warning", "This is a sample warning notification.");
                            }
                        }),
                )
                .child(
                    Text::new("The buttons above add notifications to the queue.")
                        .size(12.0)
                        .color(MUTED),
                ),
        )
}

fn hero(modal_open: Signal<bool>, dialog_open: Signal<bool>, toasts: ToastQueue) -> Stack {
    let modal_click = modal_open.clone();
    let dialog_click = dialog_open.clone();
    let toast_click = toasts.clone();

    Stack::new()
        .fill_width()
        .height(196.0)
        .child_at(
            Canvas::new()
                .background(Fill::linear(
                    GradientStops::two(Color::hex(0xDBEAFE), Color::hex(0xFCE7F3)),
                    22.0,
                ))
                .on_draw(|ctx| {
                    let r = ctx.rect();
                    ctx.circle(
                        Point::new(r.right() - 95.0, r.top() + 20.0),
                        120.0,
                        Fill::solid(Color::rgba(1.0, 1.0, 1.0, 0.26)),
                    );
                    ctx.circle(
                        Point::new(r.right() - 245.0, r.bottom() + 55.0),
                        105.0,
                        Fill::solid(Color::rgba(1.0, 1.0, 1.0, 0.16)),
                    );
                    ctx.circle(
                        Point::new(r.right() - 360.0, r.top() - 35.0),
                        70.0,
                        Fill::solid(Color::rgba(0.15, 0.39, 0.92, 0.08)),
                    );
                }),
            StackPos::fill(),
        )
        .child_at(
            Container::new()
                .fill_size()
                .padding_xy(30.0, 26.0)
                .child(
                    Column::new()
                        .fill_size()
                        .gap(12.0)
                        .child(
                            Chip::new("COMPONENT GALLERY")
                                .background(Color::rgba(1.0, 1.0, 1.0, 0.72))
                                .border(Color::rgba(1.0, 1.0, 1.0, 0.9))
                                .text_color(ACCENT)
                                .font_size(10.0),
                        )
                        .child(
                            Text::new("See every detail of the framework")
                                .size(32.0)
                                .bold()
                                .color(TEXT),
                        )
                        .child(
                            Text::new(
                                "Shadows, gradients, blur, forms, tables, trees, menus, dialogs, toasts and animations.",
                            )
                            .size(12.5)
                            .color(MUTED)
                            .wrap_width(620.0),
                        )
                        .child(
                            Row::new()
                                .align_items(Align::Center)
                                .gap(10.0)
                                .child(
                                    Button::labeled("Open modal")
                                        .style(BTN_PRIMARY)
                                        .on_click(move || modal_click.set(true)),
                                )
                                .child(
                                    Button::labeled("Open dialog")
                                        .style(BTN_SECONDARY)
                                        .on_click(move || dialog_click.set(true)),
                                )
                                .child(
                                    Button::labeled("Test toast")
                                        .style(BTN_GHOST)
                                        .on_click(move || {
                                            toast_click.info(
                                                "Information",
                                                "This notification comes from the toast queue.",
                                            );
                                        }),
                                ),
                        ),
                ),
            StackPos::fill(),
        )
}

fn buttons_body() -> Column {
    Column::new()
        .gap(12.0)
        .child(
            Wrap::new()
                .gap(10.0)
                .align_items(Align::Center)
                .child(Button::labeled("Save changes").style(BTN_PRIMARY))
                .child(Button::labeled("Cancel").style(BTN_SECONDARY))
                .child(Button::labeled("Ghost").style(BTN_GHOST))
                .child(Button::labeled("Delete").style(BTN_DANGER)),
        )
        .child(
            Wrap::new()
                .gap(10.0)
                .align_items(Align::Center)
                .child(
                    Button::labeled("Disabled")
                        .style(BTN_SECONDARY)
                        .disabled(true),
                )
                .child(Button::labeled("Minimal").style(BTN_GHOST).font_size(11.0))
                .child(
                    Button::labeled("Larger size")
                        .style(BTN_SECONDARY)
                        .font_size(13.0)
                        .padding_xy(18.0, 10.0),
                ),
        )
        .child(hint(
            "Each button has separate hover, pressed and disabled states.",
        ))
}

fn inputs_body() -> Column {
    let email = Signal::new(String::from("ada@sequoia.dev"));
    let password = Signal::new(String::from("very-secret-password"));
    let note = Signal::new(String::from(
        "This field shows multiple lines of text, selection and scrolling.",
    ));
    let count = Signal::new(12.0f32);
    let date = Signal::new(String::from("2026-10-28"));
    let drag = Signal::new(24.0f32);

    Column::new()
        .gap(12.0)
        .child(
            Row::new()
                .align_items(Align::Start)
                .gap(12.0)
                .child_flex(
                    Field::new(email, "E-mail")
                        .placeholder("your@email.com")
                        .rule("Enter a valid e-mail", |v: &str| v.contains('@')),
                    1.0,
                )
                .child_flex(
                    Column::new().gap(5.0).child(label("Password")).child(
                        TextField::new(password)
                            .placeholder("Minimum 8 characters")
                            .background(SURFACE_ALT)
                            .border(1.0, BORDER)
                            .radius(10.0)
                            .text_color(TEXT)
                            .placeholder_color(FAINT)
                            .line_color(ACCENT)
                            .padding_xy(10.0, 8.0)
                            .width(Length::Fill(1.0)),
                    ),
                    1.0,
                ),
        )
        .child(
            TextArea::new(note)
                .placeholder("Type a note...")
                .background(SURFACE_ALT)
                .border(1.0, BORDER)
                .radius(10.0)
                .color(TEXT)
                .placeholder_color(FAINT)
                .line_height_factor(1.35)
                .padding_xy(10.0, 8.0)
                .height(88.0)
                .width(Length::Fill(1.0)),
        )
        .child(
            Row::new()
                .align_items(Align::Start)
                .gap(12.0)
                .child_flex(
                    NumberField::new(count, "Quantity")
                        .min(0.0)
                        .max(999.0)
                        .style(FormStyle::new().button(STEP_BTN)),
                    1.0,
                )
                .child_flex(
                    DateField::new(date, "Date").style(FormStyle::new().button(STEP_BTN)),
                    1.0,
                )
                .child_flex(
                    Column::new().gap(5.0).child(label("Drag")).child(
                        DragValue::new(drag)
                            .min(0.0)
                            .max(100.0)
                            .precision(0)
                            .color(TEXT)
                            .line_color(ACCENT)
                            .width(Length::Fill(1.0)),
                    ),
                    0.8,
                ),
        )
}

fn selection_body() -> Column {
    let notifications = Signal::new(true);
    let auto_save = Signal::new(true);
    let volume = Signal::new(64.0f32);
    let plan = Signal::new(1usize);

    Column::new()
        .gap(14.0)
        .child(
            Checkbox::new(notifications)
                .label("Notifications")
                .background(SURFACE)
                .checked_background(ACCENT)
                .border_color(BORDER)
                .checked_border(ACCENT)
                .check_color(Color::WHITE)
                .text_color(TEXT),
        )
        .child(
            Switch::new(auto_save)
                .label("Auto save")
                .track_on(ACCENT)
                .track_off(Color::hex(0xD7DEE9))
                .thumb_color(SURFACE)
                .thumb_on_color(SURFACE)
                .border_color(BORDER)
                .border_on_color(ACCENT)
                .text_color(TEXT),
        )
        .child(
            Column::new()
                .gap(6.0)
                .child(
                    Row::new()
                        .align_items(Align::Center)
                        .child(Text::new("Volume").size(12.0).color(TEXT))
                        .child(Spacer::flex(1.0))
                        .child(
                            Text::with({
                                let volume = volume.clone();
                                move || format!("{:.0}%", volume.get())
                            })
                            .size(12.0)
                            .color(MUTED),
                        ),
                )
                .child(
                    Slider::new(volume)
                        .min(0.0)
                        .max(100.0)
                        .track_color(Color::hex(0xE2E8F0))
                        .fill_color(ACCENT)
                        .thumb_center(Color::WHITE)
                        .thumb_outline(ACCENT)
                        .thumb_size(16.0)
                        .track_height(5.0)
                        .width(Length::Fill(1.0)),
                ),
        )
        .child(Divider::horizontal().color(BORDER))
        .child(
            Column::new().gap(8.0).child(label("Plan")).child(
                RadioGroup::new(plan, ["Start", "Pro", "Enterprise"])
                    .horizontal()
                    .gap(18.0),
            ),
        )
}

fn feedback_body() -> Column {
    let progress = Signal::new(0.68f32);

    Column::new()
        .gap(14.0)
        .child(
            Row::new()
                .align_items(Align::Center)
                .gap(10.0)
                .child(Avatar::named("Ada Lovelace").size(42.0).background(ACCENT))
                .child(
                    Column::new()
                        .gap(2.0)
                        .child(Text::new("Ada Lovelace").size(13.0).bold().color(TEXT))
                        .child(Text::new("Interface engineer").size(11.0).color(MUTED)),
                )
                .child(Spacer::flex(1.0))
                .child(Badge::count(8).background(ROSE).text_color(Color::WHITE)),
        )
        .child(
            Wrap::new()
                .gap(8.0)
                .child(
                    Chip::new("Rust")
                        .selected(true)
                        .selected_background(ACCENT_SOFT)
                        .selected_border(Color::hex(0xBFDBFE))
                        .selected_text(ACCENT),
                )
                .child(
                    Chip::new("GPU")
                        .background(SURFACE_ALT)
                        .border(BORDER)
                        .text_color(TEXT),
                )
                .child(
                    Chip::new("Animations")
                        .background(SURFACE_ALT)
                        .border(BORDER)
                        .text_color(TEXT),
                )
                .child(
                    Chip::new("Accessibility")
                        .background(SURFACE_ALT)
                        .border(BORDER)
                        .text_color(TEXT),
                ),
        )
        .child(
            Column::new()
                .gap(6.0)
                .child(
                    Row::new()
                        .align_items(Align::Center)
                        .child(Text::new("Task progress").size(12.0).color(TEXT))
                        .child(Spacer::flex(1.0))
                        .child(
                            Text::with({
                                let progress = progress.clone();
                                move || format!("{:.0}%", progress.get() * 100.0)
                            })
                            .size(12.0)
                            .color(MUTED),
                        ),
                )
                .child(
                    ProgressBar::new(progress)
                        .height(9.0)
                        .track(Color::hex(0xE2E8F0))
                        .fill(ACCENT),
                ),
        )
        .child(
            Row::new()
                .align_items(Align::Center)
                .gap(10.0)
                .child(Spinner::new().size(20.0).color(ACCENT))
                .child(
                    Column::new()
                        .gap(5.0)
                        .child(Skeleton::new().height(9.0).width(Length::Fill(1.0)))
                        .child(Skeleton::new().height(9.0).width(Length::Px(140.0))),
                ),
        )
        .child(
            Row::new()
                .align_items(Align::Center)
                .gap(8.0)
                .child(
                    Kbd::new("Ctrl")
                        .background(SURFACE_ALT)
                        .border(BORDER)
                        .text_color(MUTED),
                )
                .child(
                    Kbd::new("Shift")
                        .background(SURFACE_ALT)
                        .border(BORDER)
                        .text_color(MUTED),
                )
                .child(
                    Kbd::new("P")
                        .background(SURFACE_ALT)
                        .border(BORDER)
                        .text_color(MUTED),
                )
                .child(Text::new("Quick search").size(11.0).color(MUTED)),
        )
}

#[derive(Clone)]
struct FileEntry {
    name: String,
    kind: String,
    size: String,
    modified: String,
}

fn file_key(entry: &FileEntry) -> u64 {
    entry.name.as_bytes().iter().fold(0u64, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(u64::from(*byte))
    })
}

fn table_body() -> Container {
    let rows = vec![
        FileEntry {
            name: String::from("annual-report.pdf"),
            kind: String::from("PDF"),
            size: String::from("2.4 MB"),
            modified: String::from("today, 09:42"),
        },
        FileEntry {
            name: String::from("sequoia-project.fig"),
            kind: String::from("Design"),
            size: String::from("18.1 MB"),
            modified: String::from("today, 08:15"),
        },
        FileEntry {
            name: String::from("team-notes.md"),
            kind: String::from("Markdown"),
            size: String::from("24 KB"),
            modified: String::from("yesterday"),
        },
        FileEntry {
            name: String::from("budget-2026.xlsx"),
            kind: String::from("Spreadsheet"),
            size: String::from("412 KB"),
            modified: String::from("yesterday"),
        },
        FileEntry {
            name: String::from("q4-presentation.key"),
            kind: String::from("Presentation"),
            size: String::from("6.8 MB"),
            modified: String::from("Oct 28, 2026"),
        },
        FileEntry {
            name: String::from("customer-data.csv"),
            kind: String::from("CSV"),
            size: String::from("1.2 MB"),
            modified: String::from("Oct 27, 2026"),
        },
        FileEntry {
            name: String::from("photo-archive.zip"),
            kind: String::from("Archive"),
            size: String::from("94 MB"),
            modified: String::from("Oct 25, 2026"),
        },
        FileEntry {
            name: String::from("training-agreement.docx"),
            kind: String::from("Document"),
            size: String::from("88 KB"),
            modified: String::from("Oct 22, 2026"),
        },
    ];

    let table = Table::new(move || rows.clone())
        .key(file_key)
        .column(
            TableColumn::new("Name")
                .sort_key(|a: &FileEntry, b: &FileEntry| a.name.cmp(&b.name))
                .matches(|r: &FileEntry, query: &str| {
                    r.name.to_lowercase().contains(&query.to_lowercase())
                })
                .text_cell(|r: &FileEntry| r.name.clone()),
        )
        .column(
            TableColumn::new("Type")
                .sort_key(|a: &FileEntry, b: &FileEntry| a.kind.cmp(&b.kind))
                .text_cell(|r: &FileEntry| r.kind.clone()),
        )
        .column(
            TableColumn::new("Size")
                .sort_key(|a: &FileEntry, b: &FileEntry| a.size.cmp(&b.size))
                .text_cell(|r: &FileEntry| r.size.clone()),
        )
        .column(TableColumn::new("Modified").text_cell(|r: &FileEntry| r.modified.clone()))
        .row_height(30.0)
        .style(
            TableStyle::new()
                .header_size(11.0)
                .header_color(MUTED)
                .row_stripe(SURFACE_ALT),
        );

    Container::new()
        .fill_width()
        .height(270.0)
        .clip(true)
        .child(table)
}

fn tree_body() -> Container {
    let tree = TreeView::new(|| {
        vec![
            TreeNode::branch(
                "src",
                String::from("src"),
                vec![
                    TreeNode::leaf("main.rs", String::from("main.rs")),
                    TreeNode::leaf("app.rs", String::from("app.rs")),
                    TreeNode::branch(
                        "views",
                        String::from("views"),
                        vec![
                            TreeNode::leaf("button.rs", String::from("button.rs")),
                            TreeNode::leaf("table.rs", String::from("table.rs")),
                            TreeNode::leaf("modal.rs", String::from("modal.rs")),
                        ],
                    )
                    .expanded(true),
                    TreeNode::leaf("theme.rs", String::from("theme.rs")),
                ],
            )
            .expanded(true),
            TreeNode::branch(
                "docs",
                String::from("docs"),
                vec![
                    TreeNode::leaf("guide.md", String::from("guide.md")),
                    TreeNode::leaf("api.md", String::from("api.md")),
                ],
            )
            .expanded(true),
            TreeNode::leaf("Cargo.toml", String::from("Cargo.toml")),
            TreeNode::leaf("README.md", String::from("README.md")),
        ]
    })
    .row(|value: &String, _depth| Text::new(value.clone()).size(12.0).color(TEXT))
    .row_height(28.0)
    .style(TreeStyle::new().arrow_color(MUTED).font_size(12.0));

    Container::new()
        .fill_width()
        .height(270.0)
        .clip(true)
        .child(tree)
}

fn tabs_body() -> Column {
    let active = Signal::new(0usize);

    Column::new()
        .gap(12.0)
        .child(
            TabStrip::new(active.clone(), ["Overview", "Activity", "Settings"])
                .height(38.0)
                .equal_width(true)
                .accent(ACCENT)
                .active_bg(SURFACE_ALT)
                .label_color(MUTED)
                .active_label_color(TEXT),
        )
        .child(
            Container::new()
                .fill_width()
                .padding(16.0)
                .background(SURFACE_ALT)
                .border(1.0, BORDER)
                .radius(12.0)
                .child(
                    Column::new()
                        .gap(6.0)
                        .child(
                            Text::with({
                                let active = active.clone();
                                move || {
                                    match active.get() {
                                        0 => String::from("Project overview"),
                                        1 => String::from("Recent activity"),
                                        _ => String::from("Account settings"),
                                    }
                                }
                            })
                            .size(14.0)
                            .bold()
                            .color(TEXT),
                        )
                        .child(
                            Text::with({
                                let active = active.clone();
                                move || {
                                    match active.get() {
                                        0 => String::from(
                                            "Tabs change content without rebuilding the entire tree.",
                                        ),
                                        1 => String::from(
                                            "Click another tab to see reactive text change.",
                                        ),
                                        _ => String::from(
                                            "Tab style, active color and equal widths are configurable.",
                                        ),
                                    }
                                }
                            })
                            .size(12.0)
                            .color(MUTED)
                            .wrap(),
                        ),
                ),
        )
        .child(hint(
            "TabStrip supports selection, closing and detaching tabs.",
        ))
}

fn overlays_body(modal_open: Signal<bool>, drawer_open: Signal<bool>) -> Column {
    let modal_click = modal_open.clone();
    let drawer_click = drawer_open.clone();

    Column::new()
        .gap(12.0)
        .child(
            Wrap::new()
                .gap(10.0)
                .align_items(Align::Center)
                .child(
                    Tooltip::new(
                        Button::labeled("Hover me").style(BTN_SECONDARY),
                        "Tooltip rendered in the overlay layer",
                    )
                    .background(TEXT)
                    .text_color(Color::WHITE)
                    .radius(8.0)
                    .padding_xy(10.0, 6.0),
                )
                .child(
                    FloatingMenu::wrap(Button::labeled("Actions menu").style(BTN_SECONDARY))
                        .open_on_click()
                        .below_invoker()
                        .background(SURFACE)
                        .border(BORDER)
                        .text_color(TEXT)
                        .radius(12.0)
                        .text_row("Duplicate", || {})
                        .text_row("Export", || {})
                        .separator()
                        .text_row("Delete", || {}),
                )
                .child(
                    Button::labeled("Modal")
                        .style(BTN_GHOST)
                        .on_click(move || modal_click.set(true)),
                )
                .child(
                    Button::labeled("Side panel")
                        .style(BTN_GHOST)
                        .on_click(move || drawer_click.set(true)),
                ),
        )
        .child(Divider::horizontal().color(BORDER))
        .child(Column::new().gap(6.0).child(label("Layers")).child(hint(
            "Modal, dialog, drawer, menu and tooltip use a shared overlay system.",
        )))
}

fn effects_body() -> Column {
    Column::new()
        .gap(16.0)
        .child(
            Row::new()
                .align_items(Align::Stretch)
                .gap(18.0)
                .child_flex(elevation("Subtle", 14.0, 4.0, 0.07), 1.0)
                .child_flex(elevation("Medium", 26.0, 10.0, 0.11), 1.0)
                .child_flex(elevation("Strong", 42.0, 18.0, 0.15), 1.0)
                .child_flex(
                    Container::new()
                        .padding(14.0)
                        .box_style(
                            BoxStyle::new()
                                .background(SURFACE)
                                .border(1.0, BORDER)
                                .radius(14.0)
                                .inset_shadow(
                                    ShadowSpec::new(Color::rgba(0.07, 0.11, 0.20, 0.10), 14.0)
                                        .spread(2.0),
                                ),
                        )
                        .child(
                            Column::new()
                                .gap(6.0)
                                .child(Text::new("Inset").size(13.0).bold().color(TEXT))
                                .child(Text::new("Inner shadow").size(11.0).color(MUTED)),
                        ),
                    1.0,
                ),
        )
        .child(
            Stack::new()
                .fill_width()
                .height(150.0)
                .child_at(
                    Canvas::new()
                        .background(Fill::linear(
                            GradientStops::two(Color::hex(0xDBEAFE), Color::hex(0xFBCFE8)),
                            0.0,
                        ))
                        .on_draw(|ctx| {
                            let r = ctx.rect();
                            ctx.circle(
                                Point::new(r.left() + 120.0, r.top() + 40.0),
                                70.0,
                                Fill::solid(Color::rgba(0.15, 0.39, 0.92, 0.18)),
                            );
                            ctx.circle(
                                Point::new(r.right() - 150.0, r.bottom() - 20.0),
                                90.0,
                                Fill::solid(Color::rgba(0.86, 0.15, 0.47, 0.16)),
                            );
                        }),
                    StackPos::fill(),
                )
                .child_at(
                    BackdropBlur::blurred(
                        Container::new()
                            .padding(16.0)
                            .background(Color::rgba(1.0, 1.0, 1.0, 0.58))
                            .border(1.0, Color::rgba(1.0, 1.0, 1.0, 0.75))
                            .radius(14.0)
                            .child(
                                Column::new()
                                    .gap(5.0)
                                    .child(
                                        Text::new("Backdrop blur + gradient")
                                            .size(15.0)
                                            .bold()
                                            .color(TEXT),
                                    )
                                    .child(
                                        Text::new(
                                            "The layer blurs everything beneath it, and the gradient remains visible.",
                                        )
                                        .size(11.5)
                                        .color(MUTED)
                                        .wrap_width(420.0),
                                    ),
                            ),
                        18.0,
                    ),
                    StackPos::top_left().offset(24.0, 24.0),
                )
                .child_at(
                    Pulse::new(
                        Container::new()
                            .padding_xy(12.0, 8.0)
                            .background(Color::rgba(1.0, 1.0, 1.0, 0.8))
                            .border(1.0, Color::rgba(1.0, 1.0, 1.0, 0.95))
                            .radius(10.0)
                            .child(
                                Row::new()
                                    .align_items(Align::Center)
                                    .gap(8.0)
                                    .child(
                                        Container::new()
                                            .square(8.0)
                                            .radius(4.0)
                                            .background(GREEN),
                                    )
                                    .child(
                                        Text::new("Pulse animation")
                                            .size(11.0)
                                            .bold()
                                            .color(TEXT),
                                    ),
                            ),
                    )
                    .period_duration(Duration::from_secs_f32(2.0)),
                    StackPos::bottom_left().offset(24.0, -22.0),
                ),
        )
        .child(hint(
            "The light background lets you evaluate the soft shadow edge, inset and background blur.",
        ))
}

fn elevation(label: &str, blur: f32, offset_y: f32, alpha: f32) -> Container {
    Container::new()
        .padding(14.0)
        .box_style(
            BoxStyle::new()
                .background(SURFACE)
                .border(1.0, BORDER)
                .radius(14.0)
                .shadow(
                    ShadowSpec::new(Color::rgba(0.07, 0.11, 0.20, alpha), blur)
                        .offset(0.0, offset_y),
                ),
        )
        .child(
            Column::new()
                .gap(6.0)
                .child(Text::new(label).size(13.0).bold().color(TEXT))
                .child(
                    Text::new(format!("blur {:.0} px", blur))
                        .size(11.0)
                        .color(MUTED),
                )
                .child(
                    Text::new(format!("offset 0 / {:.0}", offset_y))
                        .size(11.0)
                        .color(FAINT),
                ),
        )
}

fn virtual_list_body() -> Container {
    let list = VirtualList::new(|| {
        (0..500)
            .map(|index| format!("Item {:03} - virtual row", index))
            .collect::<Vec<_>>()
    })
    .key(|value: &String| {
        value.as_bytes().iter().fold(0u64, |hash, byte| {
            hash.wrapping_mul(31).wrapping_add(u64::from(*byte))
        })
    })
    .item_indexed(|index, value: &String| {
        Container::new()
            .fill_width()
            .height(38.0)
            .padding_xy(12.0, 9.0)
            .background(if index % 2 == 0 { SURFACE } else { SURFACE_ALT })
            .border_bottom(1.0, Color::hex(0xEEF2F7))
            .child(
                Row::new()
                    .align_items(Align::Center)
                    .gap(10.0)
                    .child(
                        Container::new()
                            .width(34.0)
                            .height(20.0)
                            .background(ACCENT_SOFT)
                            .radius(6.0)
                            .center()
                            .child(
                                Text::new(format!("{:02}", index % 100))
                                    .size(10.0)
                                    .bold()
                                    .color(ACCENT),
                            ),
                    )
                    .child(Text::new(value.clone()).size(12.0).color(TEXT))
                    .child(Spacer::flex(1.0))
                    .child(Text::new("visible").size(10.0).color(FAINT)),
            )
    })
    .item_height(38.0)
    .scrollbar_width(9.0)
    .scrollbar_track(Color::TRANSPARENT)
    .scrollbar_thumb(Color::hex(0xB8C4D6))
    .height(250.0);

    Container::new()
        .fill_width()
        .height(250.0)
        .clip(true)
        .border(1.0, BORDER)
        .radius(12.0)
        .child(list)
}

fn modal(open: Signal<bool>) -> Modal {
    let close = open.clone();

    Modal::new(open)
        .width(470.0)
        .padding(24.0)
        .background(SURFACE)
        .border(1.0, BORDER)
        .radius(18.0)
        .child(
            Column::new()
                .gap(14.0)
                .child(Text::new("Modal").size(21.0).bold().color(TEXT))
                .child(
                    Text::new(
                        "Modal is a layer with its own background, shadow and Escape handling. Clicking outside the window also closes it.",
                    )
                    .size(12.5)
                    .color(MUTED)
                    .wrap(),
                )
                .child(
                    Container::new()
                        .fill_width()
                        .padding(14.0)
                        .background(SURFACE_ALT)
                        .border(1.0, BORDER)
                        .radius(12.0)
                        .child(
                            Row::new()
                                .align_items(Align::Center)
                                .gap(10.0)
                                .child(Avatar::named("Sequoia UI").size(34.0).background(ACCENT))
                                .child(
                                    Text::new("The overlay works over the entire gallery.")
                                        .size(12.0)
                                        .color(TEXT),
                                ),
                        ),
                )
                .child(
                    Row::new()
                        .gap(8.0)
                        .child(Spacer::flex(1.0))
                        .child(
                            Button::labeled("Close")
                                .style(BTN_PRIMARY)
                                .on_click(move || close.set(false)),
                        ),
                ),
        )
}

fn drawer(open: Signal<bool>) -> Drawer {
    let close = open.clone();

    Drawer::new(open)
        .right()
        .width(370.0)
        .padding(24.0)
        .background(SURFACE)
        .border(1.0, BORDER)
        .child(
            Column::new()
                .fill_size()
                .gap(14.0)
                .child(Text::new("Side panel").size(22.0).bold().color(TEXT))
                .child(
                    Text::new(
                        "The drawer slides in from the edge and can contain any views - forms, lists, settings.",
                    )
                    .size(12.5)
                    .color(MUTED)
                    .wrap(),
                )
                .child(Divider::horizontal().color(BORDER))
                .child(check_row("Notifications", true))
                .child(check_row("Auto save", true))
                .child(check_row("Compact mode", false))
                .child(Spacer::flex(1.0))
                .child(
                    Button::labeled("Close panel")
                        .style(BTN_SECONDARY)
                        .on_click(move || close.set(false)),
                ),
        )
}

fn check_row(label_text: &str, checked: bool) -> Checkbox {
    Checkbox::new(Signal::new(checked))
        .label(label_text)
        .background(SURFACE)
        .checked_background(ACCENT)
        .border_color(BORDER)
        .checked_border(ACCENT)
        .check_color(Color::WHITE)
        .text_color(TEXT)
}

fn nav_row(text: &str, color: Color, active: bool) -> Container {
    Container::new()
        .fill_width()
        .padding_xy(10.0, 9.0)
        .background(if active {
            ACCENT_SOFT
        } else {
            Color::TRANSPARENT
        })
        .radius(9.0)
        .child(
            Row::new()
                .align_items(Align::Center)
                .gap(9.0)
                .child(Container::new().square(8.0).radius(4.0).background(color))
                .child(
                    Text::new(text)
                        .size(12.0)
                        .bold_if(active)
                        .color(if active { ACCENT } else { TEXT })
                        .truncate_fill(),
                ),
        )
}

fn palette() -> Row {
    Row::new()
        .gap(7.0)
        .child(swatch(ACCENT))
        .child(swatch(GREEN))
        .child(swatch(AMBER))
        .child(swatch(ROSE))
        .child(swatch(VIOLET))
        .child(swatch(Color::hex(0x0EA5E9)))
}

fn swatch(color: Color) -> Container {
    Container::new()
        .square(22.0)
        .radius(7.0)
        .background(color)
        .border(2.0, SURFACE)
        .box_style(
            BoxStyle::new()
                .background(color)
                .radius(7.0)
                .border(2.0, SURFACE)
                .shadow(ShadowSpec::new(Color::rgba(0.07, 0.11, 0.20, 0.16), 8.0).offset(0.0, 2.0)),
        )
}

fn card(title: &str, eyebrow: &str, body: impl View + 'static) -> Container {
    Container::new()
        .padding(20.0)
        .box_style(surface_style())
        .child(
            Column::new()
                .gap(12.0)
                .child(Text::new(eyebrow).size(9.5).bold().color(ACCENT))
                .child(Text::new(title).size(17.0).bold().color(TEXT))
                .child(body),
        )
}

fn surface_style() -> BoxStyle {
    BoxStyle::new()
        .background(SURFACE)
        .border(1.0, BORDER)
        .radius(16.0)
        .shadow(ShadowSpec::new(SHADOW, 24.0).offset(0.0, 8.0))
}

fn label(text: &str) -> Text {
    Text::new(text).size(11.5).color(MUTED)
}

fn small_label(text: &str) -> Text {
    Text::new(text).size(9.5).bold().color(FAINT)
}

fn hint(text: &str) -> Text {
    Text::new(text).size(11.0).color(FAINT).wrap()
}
