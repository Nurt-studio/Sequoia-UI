use sequoia_ui::prelude::*;

const APP_BG: Color = Color::hex(0xF1F4F9);
const SURFACE: Color = Color::hex(0xFFFFFF);
const BORDER: Color = Color::hex(0xDDE3EC);
const TEXT: Color = Color::hex(0x172033);
const MUTED: Color = Color::hex(0x64748B);
const DONE: Color = Color::hex(0x94A3B8);

#[derive(Clone, PartialEq)]
struct Todo {
    id: u64,
    title: String,
    done: bool,
}

fn add_todo(todos: &Signal<Vec<Todo>>, draft: &Signal<String>, next_id: &Signal<u64>) {
    let title = draft.get_untracked().trim().to_string();
    if title.is_empty() {
        return;
    }
    let id = next_id.get_untracked();
    next_id.set(id + 1);
    draft.set(String::new());
    todos.update(|items| {
        items.push(Todo {
            id,
            title,
            done: false,
        })
    });
}

fn main() {
    let todos = Signal::new(vec![
        Todo {
            id: 1,
            title: String::from("Read the Sequoia book"),
            done: true,
        },
        Todo {
            id: 2,
            title: String::from("Build a todo list"),
            done: false,
        },
    ]);
    let next_id = Signal::new(3u64);
    let draft = Signal::new(String::new());

    let input = TextField::new(draft.clone())
        .placeholder("What needs doing?")
        .on_submit({
            let todos = todos.clone();
            let draft = draft.clone();
            let next_id = next_id.clone();
            move |_| add_todo(&todos, &draft, &next_id)
        });

    let add = Button::new("Add").style(ButtonStyle::PRIMARY).on_click({
        let todos = todos.clone();
        let draft = draft.clone();
        let next_id = next_id.clone();
        move || add_todo(&todos, &draft, &next_id)
    });

    let item_todos = todos.clone();
    let rows = List::new({
        let todos = todos.clone();
        move || todos.get()
    })
    .key(|todo: &Todo| todo.id)
    .item(move |todo: &Todo| {
        let current = todo.clone();
        let toggle = item_todos.clone();
        let remove = item_todos.clone();
        let done = current.done;

        Row::new()
            .fill_width()
            .gap(10.0)
            .padding_xy(4.0, 8.0)
            .child(
                Button::new(if done { "Done" } else { "Mark done" })
                    .style(ButtonStyle::GHOST)
                    .on_click(move || {
                        let id = current.id;
                        toggle.update(|items| {
                            if let Some(item) = items.iter_mut().find(|item| item.id == id) {
                                item.done = !item.done;
                            }
                        });
                    }),
            )
            .child(
                Text::new(current.title.clone())
                    .size(14.0)
                    .color(if done { DONE } else { TEXT })
                    .strikethrough(done),
            )
            .child(Spacer::flex(1.0))
            .child(
                Button::new("Remove")
                    .style(ButtonStyle::GHOST)
                    .on_click(move || {
                        let id = current.id;
                        remove.update(|items| items.retain(|item| item.id != id));
                    }),
            )
    });

    let count = Text::with({
        let todos = todos.clone();
        move || {
            let items = todos.get();
            let done = items.iter().filter(|item| item.done).count();
            format!("{done} of {} done", items.len())
        }
    })
    .size(13.0)
    .color(MUTED);

    let card = Container::new()
        .width(520.0)
        .padding(24.0)
        .background(SURFACE)
        .radius(16.0)
        .border(1.0, BORDER)
        .child(
            Column::new()
                .fill_width()
                .gap(14.0)
                .child(
                    Row::new()
                        .fill_width()
                        .child(Text::new("Today").size(24.0).bold().color(TEXT))
                        .child(Spacer::flex(1.0))
                        .child(count),
                )
                .child(
                    Row::new()
                        .fill_width()
                        .gap(8.0)
                        .child(Container::new().flex(1.0).child(input))
                        .child(add),
                )
                .child(Divider::horizontal())
                .child(
                    Container::new()
                        .height(260.0)
                        .child(ScrollView::new(rows).vertical()),
                )
                .child(
                    Text::new("Type a task and press Enter or click Add.")
                        .size(12.0)
                        .color(MUTED),
                )
                .child(
                    Row::new()
                        .fill_width()
                        .gap(8.0)
                        .child(
                            Button::new("Clear completed")
                                .style(ButtonStyle::SECONDARY)
                                .on_click({
                                    let todos = todos.clone();
                                    move || todos.update(|items| items.retain(|item| !item.done))
                                }),
                        )
                        .child(Spacer::flex(1.0))
                        .child(Button::new("Reset").style(ButtonStyle::GHOST).on_click({
                            let todos = todos.clone();
                            move || {
                                todos.set(vec![
                                    Todo {
                                        id: 1,
                                        title: String::from("Read the Sequoia book"),
                                        done: true,
                                    },
                                    Todo {
                                        id: 2,
                                        title: String::from("Build a todo list"),
                                        done: false,
                                    },
                                ]);
                            }
                        })),
                ),
        );

    App::new()
        .title("Todo")
        .size(640.0, 600.0)
        .root(
            Stack::new()
                .fill_size()
                .background(APP_BG)
                .padding(24.0)
                .child(card),
        )
        .run()
        .expect("failed to run the todo example");
}
