use sequoia_ui::prelude::*;

const APP_BG: Color = Color::hex(0xF1F4F9);
const SURFACE: Color = Color::hex(0xFFFFFF);
const BORDER: Color = Color::hex(0xDDE3EC);
const TEXT: Color = Color::hex(0x172033);
const MUTED: Color = Color::hex(0x64748B);
const GREEN: Color = Color::hex(0x059669);

fn main() {
    let name = Signal::new(String::new());
    let email = Signal::new(String::new());
    let seats = Signal::new(3.0);
    let plan = Signal::new(1usize);
    let newsletter = Signal::new(true);
    let volume = Signal::new(65.0);

    let form = Form::new();

    let name_field = Field::new(name.clone(), "Name")
        .placeholder("Ada Lovelace")
        .rule("Name is required", |value| !value.trim().is_empty());
    let email_field = Field::new(email.clone(), "Email")
        .placeholder("you@example.com")
        .rule("Enter a valid email", |value| value.contains('@'));
    let seats_field = NumberField::new(seats.clone(), "Seats").min(1.0).max(20.0);

    form.register(&name_field);
    form.register(&email_field);
    form.register(&seats_field);

    let submitted = Signal::new(false);
    let submit_form = form.clone();
    let submitted_flag = submitted.clone();
    let submit = Button::new("Create account")
        .style(ButtonStyle::PRIMARY)
        .on_click(move || {
            submit_form.touch_all();
            if submit_form.is_valid() {
                submitted_flag.set(true);
            }
        });

    let plan_group = RadioGroup::new(plan.clone(), ["Free", "Pro", "Team"]).label("Plan");
    let volume_row = Row::new()
        .fill_width()
        .gap(12.0)
        .child(Text::new("Volume").size(13.0).color(TEXT))
        .child(Slider::new(volume.clone()).min(0.0).max(100.0))
        .child(
            Text::with({
                let volume = volume.clone();
                move || format!("{:.0}%", volume.get())
            })
            .size(13.0)
            .color(MUTED),
        );

    let status = Text::with({
        let name = name.clone();
        let email = email.clone();
        let submitted = submitted.clone();
        move || {
            let _ = name.get();
            let _ = email.get();
            if submitted.get() {
                "Account created.".to_string()
            } else {
                "All fields validate on submit.".to_string()
            }
        }
    })
    .size(13.0)
    .color(GREEN);

    let card = Container::new()
        .width(460.0)
        .padding(28.0)
        .background(SURFACE)
        .radius(16.0)
        .border(1.0, BORDER)
        .child(
            Column::new()
                .fill_width()
                .gap(16.0)
                .child(Text::new("Create an account").size(24.0).bold())
                .child(
                    Text::new("A form with validation, numeric input and live state.")
                        .size(14.0)
                        .color(MUTED),
                )
                .child(name_field)
                .child(email_field)
                .child(
                    Row::new()
                        .fill_width()
                        .gap(16.0)
                        .align_items(Align::Start)
                        .child(Container::new().flex(1.0).child(seats_field))
                        .child(Container::new().flex(1.0).child(plan_group)),
                )
                .child(volume_row)
                .child(Checkbox::new(newsletter.clone()).label("Send product updates"))
                .child(form.summary())
                .child(
                    Row::new()
                        .fill_width()
                        .gap(12.0)
                        .child(status)
                        .child(Spacer::flex(1.0))
                        .child(submit),
                ),
        );

    App::new()
        .title("Form")
        .size(720.0, 640.0)
        .root(
            Stack::new()
                .fill_size()
                .background(APP_BG)
                .padding(24.0)
                .child(card),
        )
        .run()
        .expect("failed to run the form example");
}
