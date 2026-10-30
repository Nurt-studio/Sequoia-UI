# Localization

`I18n` is a small, signal-backed translation catalog. It stores strings by
locale and key, exposes the active locale as a signal, and falls back to a base
locale when a translation is missing.

It deliberately does not choose a translation file format, detect the operating
system locale, interpolate placeholders, or pluralize. Those are application
decisions. This guide shows the built-in behavior and small, explicit helpers
for the common cases.

## Create a catalog

Create the catalog with a fallback locale. The active locale starts as the
fallback. Entries are added with `add`, and clones share the same catalog:

```rust
# use sequoia_ui::prelude::*;
let i18n = I18n::new("en");
i18n.add("en", "save", "Save");
i18n.add("pl", "save", "Zapisz");

assert_eq!(i18n.t_untracked("save"), "Save");

let same_catalog = i18n.clone();
i18n.set_locale("pl");
assert_eq!(same_catalog.t_untracked("save"), "Zapisz");
assert_eq!(i18n.locale(), "pl");
```

`add` takes `&self`, so a shared catalog can be populated through any clone.
Adding entries does not emit a change notification by itself. Load translation
tables before mounting views that read them; adding a new string while the same
locale is active does not rebuild labels that already rendered.

Locale identifiers are free-form strings such as `"en"`, `"pl"`, or `"pt-BR"`.
`I18n` does not canonicalize or validate them.

## Fallback behavior

`t` looks up the active locale first, then the fallback locale, then returns the
key itself:

```rust
# use sequoia_ui::prelude::*;
let i18n = I18n::new("en");
i18n.add("en", "save", "Save");
i18n.set_locale("de");

assert_eq!(i18n.t("save"), "Save", "fallback locale");
assert_eq!(i18n.t("open"), "open", "fallback to the key");
```

This makes a partially translated locale usable immediately. It also means a
missing translation is visible in the UI instead of rendering an empty string.

`has` checks the active locale and the fallback locale, not an arbitrary locale:

```rust
# use sequoia_ui::prelude::*;
let i18n = I18n::new("en");
i18n.add("en", "save", "Save");
i18n.add("en", "open", "Open");
i18n.add("pl", "save", "Zapisz");

assert!(i18n.has("save"));
assert!(!i18n.has("close"));
```

## Validate translations

`missing_in` reports keys that exist in the fallback locale but are absent from
the requested locale. The result is sorted and is useful in tests or a startup
check:

```rust
# use sequoia_ui::prelude::*;
let i18n = I18n::new("en");
i18n.add("en", "save", "Save");
i18n.add("en", "open", "Open");
i18n.add("pl", "save", "Zapisz");

assert_eq!(i18n.missing_in("pl"), vec!["open".to_string()]);
assert!(i18n.missing_in("en").is_empty());
```

The fallback locale is the source of truth. If a key exists only in a target
locale, it resolves there but `missing_in` does not report it. Add every source
key to the fallback table.

## Reactive labels

Reading `t` inside `Text::with` subscribes the label to locale changes:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
# fn texts(h: &Harness) -> Vec<String> {
#     let mut out = Vec::new();
#     if let Some(root) = h.root() {
#         let mut stack = vec![root];
#         while let Some(id) = stack.pop() {
#             if let Some(text) = h.text(id) {
#                 out.push(text);
#             }
#             stack.extend(h.children(id));
#         }
#     }
#     out
# }
let i18n = I18n::new("en");
i18n.add("en", "save", "Save");
i18n.add("pl", "save", "Zapisz");

let label = {
    let i18n = i18n.clone();
    Text::with(move || i18n.t("save"))
};
let h = Harness::new(label);
h.sync(200.0, 40.0);
assert!(texts(&h).iter().any(|text| text == "Save"));

i18n.set_locale("pl");
h.sync(200.0, 40.0);
assert!(texts(&h).iter().any(|text| text == "Zapisz"));
```

`Text::new(i18n.t("save"))` evaluates the lookup once and keeps that string.
Use `Text::new` only for content that is not expected to change when the locale
changes.

Use `t_untracked` in callbacks, logging, window titles, and other non-reactive
code. It reads the current locale without subscribing the caller:

```rust
# use sequoia_ui::prelude::*;
let i18n = I18n::new("en");
i18n.add("en", "save", "Save");
i18n.set_locale("pl");

assert_eq!(i18n.t_untracked("save"), "Save");
```

## Switch locales

`set_locale` updates the signal and schedules views that read `t` or the locale
signal. Use `locale` for a snapshot and `locale_signal` when a selector or effect
needs to subscribe directly:

```rust
# use sequoia_ui::prelude::*;
let i18n = I18n::new("en");
let locale = i18n.locale_signal();

assert_eq!(i18n.locale(), "en");
i18n.set_locale("pl");
assert_eq!(locale.get(), "pl");
```

`I18n` does not inspect the operating system or command line for a preferred
locale. Choose a default, optionally read a stored preference from `Settings` or
`CliArgs`, and call `set_locale` before mounting the interface. Persist the
chosen locale the same way as any other application setting.

## Load translations

The crate does not define a file format. A small `key=value` text format is
enough to demonstrate loading; real applications can use JSON, TOML, Fluent, or
gettext and call `add` for each parsed entry.

```rust
# use sequoia_ui::prelude::*;
fn parse(i18n: &I18n, locale: &str, source: &str) {
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            i18n.add(locale, key.trim(), value.trim());
        }
    }
}

let i18n = I18n::new("en");
parse(&i18n, "en", "save=Save\nopen=Open\n");
parse(&i18n, "pl", "save=Zapisz\n");

assert_eq!(i18n.t("save"), "Save");
i18n.set_locale("pl");
assert_eq!(i18n.t("save"), "Zapisz");
assert_eq!(i18n.missing_in("pl"), vec!["open".to_string()]);
```

Read each file on the UI thread, or parse it on a worker and add the resulting
strings on the UI thread. `I18n` uses `Rc` and `RefCell`, so it is not
`Send`/`Sync`. Load the tables before mounting views so the initial frame already
has complete translations:

```rust,no_run
# use sequoia_ui::prelude::*;
use std::io;

fn load(i18n: &I18n, locale: &str, path: &str) -> io::Result<()> {
    let source = std::fs::read_to_string(path)?;
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            i18n.add(locale, key.trim(), value.trim());
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let i18n = I18n::new("en");
    load(&i18n, "en", "locales/en.txt")?;
    load(&i18n, "pl", "locales/pl.txt")?;
    Ok(())
}
```

This loader ignores malformed lines. For production data, use a parser that
reports the file and line number instead of silently skipping errors.

## Format values manually

`t` returns the translated string without interpreting placeholders. A small
helper can substitute named values:

```rust
# use sequoia_ui::prelude::*;
fn tr(i18n: &I18n, key: &str, args: &[(&str, &str)]) -> String {
    let mut text = i18n.t(key);
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

let i18n = I18n::new("en");
i18n.add("en", "items", "{count} items");
let count = 3.to_string();
assert_eq!(tr(&i18n, "items", &[("count", &count)]), "3 items");
```

Because `tr` calls `t`, using it inside `Text::with` tracks locale changes. The
count signal must also be read inside the same closure:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
# fn texts(h: &Harness) -> Vec<String> {
#     let mut out = Vec::new();
#     if let Some(root) = h.root() {
#         let mut stack = vec![root];
#         while let Some(id) = stack.pop() {
#             if let Some(text) = h.text(id) {
#                 out.push(text);
#             }
#             stack.extend(h.children(id));
#         }
#     }
#     out
# }
# fn tr(i18n: &I18n, key: &str, args: &[(&str, &str)]) -> String {
#     let mut text = i18n.t(key);
#     for (name, value) in args {
#         text = text.replace(&format!("{{{name}}}"), value);
#     }
#     text
# }
let i18n = I18n::new("en");
i18n.add("en", "items", "{count} items");
let count = Signal::new(3usize);

let label = {
    let i18n = i18n.clone();
    let count = count.clone();
    Text::with(move || {
        let value = count.get().to_string();
        tr(&i18n, "items", &[("count", &value)])
    })
};
let h = Harness::new(label);
h.sync(300.0, 100.0);
assert!(texts(&h).iter().any(|text| text == "3 items"));

count.set(5);
h.sync(300.0, 100.0);
assert!(texts(&h).iter().any(|text| text == "5 items"));
```

This helper is intentionally simple. It does not handle escaping, positional
arguments, number formatting, dates, or plural rules. Use separate keys for
singular and plural forms, or a dedicated formatting crate, and keep the
translation catalog as the string source.

## Use translations from Seq

Seq's `Text(t: "key")` looks up a key through `HostBindings::i18n`. Provide the
catalog when loading the application:

```rust
# use sequoia_ui::prelude::*;
# use sequoia_test::Harness;
# fn texts(h: &Harness) -> Vec<String> {
#     let mut out = Vec::new();
#     if let Some(root) = h.root() {
#         let mut stack = vec![root];
#         while let Some(id) = stack.pop() {
#             if let Some(text) = h.text(id) {
#                 out.push(text);
#             }
#             stack.extend(h.children(id));
#         }
#     }
#     out
# }
let source = r#"app Demo {
  title: "Demo"
  size: (240, 100)
  window Main {
    Text(t: "save")
  }
}
"#;
let i18n = I18n::new("en");
i18n.add("en", "save", "Save");
i18n.add("pl", "save", "Zapisz");

let bindings = HostBindings::new().i18n(i18n.clone());
let loaded = SeqLoader::load_str_with("Main.seq", source, &bindings).expect("load Seq");
let h = Harness::new(loaded.view);
h.sync(240.0, 100.0);
assert!(texts(&h).iter().any(|text| text == "Save"));

i18n.set_locale("pl");
h.sync(240.0, 100.0);
assert!(texts(&h).iter().any(|text| text == "Zapisz"));
```

`Text(t:)` reads the catalog reactively, so a locale switch rebuilds the Seq
text without reloading the document. Without `.i18n(...)`, loading a document
that uses `t:` fails with an error that mentions the missing i18n binding.

## Practical workflow

1. Pick a fallback locale and add every source key to it.
2. Load all locale tables before mounting views.
3. Use `missing_in` in tests or a startup check to catch untranslated keys.
4. Read `t` inside `Text::with` for labels that must react to locale changes.
5. Use `t_untracked` outside reactive scopes.
6. Keep formatting and plural rules in application code, not in the catalog.

## Limitations

- No built-in file format, loader, or serializer.
- No placeholder interpolation, pluralization, number formatting, or date
  formatting.
- No automatic locale detection or persistence.
- Locale identifiers are arbitrary strings; `I18n` does not validate them.
- `has` checks only the active and fallback locales.
- `missing_in` compares against keys in the fallback locale and ignores extra
  keys in the target locale.
- Adding entries does not notify views. Load tables before mounting, or trigger
  a rebuild through another signal.
- The catalog is single-threaded. Parse on a worker if needed, but call `add` on
  the UI thread.
- Make sure the selected font covers the scripts used by the translations.

## See also

- [Text](../concepts/text.md)
- [Reactivity](../concepts/reactivity.md)
- [Seq](seq.md)
- [Desktop integration](desktop-integration.md)
- [Testing views](testing.md)
