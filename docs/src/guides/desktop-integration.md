# Desktop integration

This guide covers the parts of a desktop application around the view tree:
window configuration, runtime window management, persisted settings, native
file dialogs, command-line arguments, and single-instance locking. The
Application lifecycle concept covers the startup sequence and close handlers.

## Windows

`App` configures the primary window. Use `WindowSpec` for additional windows:

```rust
# use sequoia_ui::prelude::*;
let app = App::new()
    .title("Main")
    .size(900.0, 600.0)
    .root(Text::new("Main window"))
    .window(
        WindowSpec::new("Inspector", Text::new("Inspector"))
            .size(360.0, 480.0)
            .min_size(240.0, 320.0)
            .resizable(false)
            .always_on_top(true),
    );

assert_eq!(app.window_count(), 2);
```

`WindowSpec` accepts `title`, `size`, `root`, `on_close_request`, `min_size`,
`max_size`, `position`, `resizable`, `maximized`, `fullscreen`,
`always_on_top`, `decorations`, `transparent`, `visible`, `window_icon`,
`window_icon_tint`, `present_mode`, `power_preference`, `surface_format`,
`fps_limit`, `show_fps`, `scale_factor`, `cursor_visible`, and `cursor_grab`.
The corresponding `App` methods configure the primary window.

`present_mode`, `power_preference`, and `surface_format` are rendering
settings. The defaults are `PresentMode::Vsync` and
`PowerPreference::HighPerformance`; unsupported surface formats fall back to
the default sRGB format with a warning. Use `fps_limit` with
`RedrawMode::Continuous` when a window needs paced animation.

## Managing windows at runtime

The platform provides a `Windows` service in the shared `Environment`. The
runtime owns the service; views obtain it from an event context:

```rust
# use sequoia_ui::prelude::*;
fn open_inspector(ctx: &EventCtx<'_>) {
    if let Some(windows) = ctx.environment.get::<Windows>() {
        windows.open(WindowSpec::new("Inspector", Text::new("Inspector")));
    }
}
```

Call this from a custom view's `on_event`, not from `paint`. `Windows::open`
queues the request and returns a `WindowId`; the native window is created after
the event loop processes the queue.

The service also supports `close`, `set_title`, `minimize`,
`toggle_maximize`, and `begin_drag`. `contains`, `count`, `ids`, and `title_of`
inspect the registered windows. `WindowId::PRIMARY` identifies the first
window; `EventCtx::window` is the window that received the event. Closing the
last window stops the event loop.

The service can also be exercised without a native event loop:

```rust
# use sequoia_ui::prelude::*;
let windows = Windows::new();
let id = windows.open(WindowSpec::new("Inspector", Text::new("Inspector")));
assert!(windows.contains(id));
assert_eq!(windows.title_of(id).as_deref(), Some("Inspector"));
assert!(windows.close(id));
assert!(!windows.contains(id));
```

## Settings

`Settings` is a small JSON-backed map of string keys to string values. It is
intended for preferences, not for large data or a database:

```rust
# use sequoia_ui::prelude::*;
let dir = std::env::temp_dir().join(format!(
    "sequoia-settings-doc-{}",
    std::process::id()
));
let settings = Settings::with_dir(dir.clone());
settings.load().unwrap();

settings.set("theme", "light");
settings.set("editor.font_size", "15");
settings.save().unwrap();
assert_eq!(settings.get("theme").as_deref(), Some("light"));
assert_eq!(settings.get_or("missing", "dark"), "dark");

let reopened = Settings::with_dir(dir.clone());
reopened.load().unwrap();
assert_eq!(reopened.get_or("theme", "dark"), "light");

let _ = std::fs::remove_dir_all(dir);
```

Use `Settings::for_app(qualifier, organization, application)` to place the file
in the platform config directory. `with_dir` is useful for tests and portable
applications. The default file name is `settings.json`; change it with
`file_name`.

`load` treats a missing file as empty, but reports malformed JSON and I/O
errors as `SettingsError`. `save` creates the parent directory and writes
pretty JSON. `set` only updates the in-memory map; call `save` when the value
should persist. `remove` returns whether the key existed. `len` and `is_empty`
report the number of entries.

`watch` returns a `Signal<String>` for one key and default:

```rust
# use sequoia_ui::prelude::*;
let settings = Settings::with_dir(std::env::temp_dir());
let theme = settings.watch("theme".to_string(), "dark".to_string());
# let _ = theme;
```

The same key and default return the same signal, so a view can call `watch`
during construction. A default is used only when the key is absent; it does not
overwrite an existing value. In an application, the watcher updates when
`set` changes the map. In tests, flush reactive effects before asserting on the
signal.

Keep values small and encode structured data explicitly. `Settings` has no
schema, migration, or typed values; validate strings when reading them.

## Recent files

`RecentFiles` builds a most-recent-first list on top of `Settings`:

```rust
# use sequoia_ui::prelude::*;
let dir = std::env::temp_dir().join(format!(
    "sequoia-recent-doc-{}",
    std::process::id()
));
let settings = Settings::with_dir(dir.clone());
let recent = RecentFiles::new(settings.clone(), 8).key("recent_files");
recent.load();

recent.push("notes.seq");
recent.push("board.seq");
recent.push("notes.seq");
assert_eq!(recent.files().len(), 2);
assert_eq!(recent.files()[0], std::path::PathBuf::from("notes.seq"));

let list = recent.files_signal();
# let _ = list;
recent.remove(&std::path::PathBuf::from("notes.seq"));
recent.clear();

let _ = std::fs::remove_dir_all(dir);
```

`push` removes duplicates, inserts at the front, and truncates to the configured
maximum. It persists after every change. `load` reads the stored list and
truncates it to the maximum; malformed stored JSON becomes an empty list.
`files_signal` is convenient for a menu or sidebar. Use the same `Settings`
instance for the rest of the application so saves do not race each other.

## Native dialogs

`NativeFileDialog` wraps the platform file picker. Dialogs are blocking calls,
so invoke them from a button or menu callback, not from `paint`:

```rust,no_run
# use sequoia_ui::prelude::*;
let path = NativeFileDialog::open_file("Open project")
    .directory(".")
    .filter("Seq files", &["seq"])
    .filter("All files", &["*"])
    .pick();

if let Some(path) = path {
    // Load the selected file.
    let _ = path;
}
```

`pick_many` returns every selected path, or an empty vector on cancel.
`save_file` creates a save dialog and supports `file_name`; call `.save()` on
the builder. `pick_folder` and `.pick_dir()` select directories.

`FileFilter` is also usable on its own. Matching is case-insensitive, and the
`*` extension matches every file:

```rust
# use sequoia_ui::prelude::*;
let filter = FileFilter::new("Rust", &["rs"]);
assert!(filter.matches("main.RS"));
assert!(!filter.matches("main.txt"));
assert!(FileFilter::new("All", &["*"]).matches("anything.bin"));
```

`native_message(title, description, MessageKind::Info | Warning | Error)` opens
a blocking native message dialog. It returns nothing, so use it for
notifications rather than decisions that need a result.

## Command-line arguments

`CliArgs` parses process arguments without a third-party parser:

```rust
# use sequoia_ui::prelude::*;
let args = CliArgs::parse_from(
    ["--watch", "--theme", "light", "Main.seq"]
        .into_iter()
        .map(String::from),
);
assert!(args.has("watch"));
assert_eq!(args.get("theme"), Some("light"));
assert_eq!(args.first_file(), Some("Main.seq"));
assert_eq!(args.positionals(), &["Main.seq".to_string()]);
```

`parse()` reads the process arguments and skips the executable name.
`parse_from` expects the same arguments after the executable. Long options
accept `--key value` and `--key=value`. Short flags can be grouped (`-abc`),
and a lone `-` is positional. A dangling `--flag` is recorded with no value.
`help_text(program, options)` formats a simple usage block.

## Single instance

`SingleInstance::acquire` creates a lock file in the temporary directory. A
second acquire for the same ID fails until the guard is dropped:

```rust
# use sequoia_ui::prelude::*;
let app_id = format!("sequoia-doc-{}", std::process::id());
let guard = SingleInstance::acquire(&app_id).expect("first instance");
assert!(SingleInstance::acquire(&app_id).is_err());
drop(guard);
assert!(SingleInstance::acquire(&app_id).is_ok());
```

Use this before starting the application when only one process should own the
settings file or a device. The lock is removed on drop; a process that crashes
may leave a stale lock file behind. `lock_path_for` returns the sanitized path
if you need to inspect it.

## Practical notes

- Configure static windows with `App` and `WindowSpec`; use `Windows` only for
  windows opened or closed after startup.
- Persist settings explicitly. A `set` call is not durable until `save`.
- Native dialogs block the UI thread. Keep the surrounding work short and move
  large file reads to the async executor after the dialog returns.
- Window and dialog APIs are desktop-oriented. Keep platform-specific policy
  in the host application rather than in reusable view code.
- The examples above use a temporary directory so they can run in doctests;
  production code normally uses `Settings::for_app`.
