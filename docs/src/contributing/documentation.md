# Documentation guide

This page defines how the Sequoia book is structured, written, and verified.
Follow it for every new page and review.

## Audience and goals

The book serves two audiences:

- **Application developers** who want to build a Sequoia application and need
  task-oriented guides.
- **Contributors** who work on the framework and need to understand how the
  crates fit together.

Pages should help a reader finish a task, not merely enumerate an API. API
details belong in rustdoc; the book explains when and why to use an API and
shows complete, working examples.

## Book layout

```text
docs/
├── book.toml
└── src/
    ├── SUMMARY.md
    ├── introduction.md
    ├── assets/              # images referenced by pages
    ├── concepts/            # cross-cutting ideas
    ├── guides/              # subsystem guides
    ├── cookbook/            # end-to-end recipes
    └── contributing/        # contributor documentation
```

Every content page must be listed in `SUMMARY.md`. Use kebab-case file names
and group pages into directories by topic. Keep the source tree flat unless a
topic already has several pages.

The book is built from the workspace root:

```text
cargo install mdbook --locked
mdbook build docs
mdbook serve docs --open
```

`mdbook build` fails when a page is missing from the source tree, so it is also
a link check for the table of contents.

## Crate map

Sequoia is a workspace. The facade crate `sequoia-ui` re-exports the modules
below, and each module maps to one crate.

| Module | Crate | Responsibility |
| --- | --- | --- |
| `core` | `sequoia-core` | Geometry, color, time, render lists, shared primitives |
| `reactive` | `sequoia-reactive` | Signals, stores, effects, memos, undo, i18n |
| `view` | `sequoia-view` | View tree, runtime, containers, overlays, docking views |
| `layout` | `sequoia-layout` | Flex, grid, wrap, masonry, and track-based layout |
| `style` | `sequoia-style` | Styles, themes, visual states, effects, gradients |
| `elements` | `sequoia-elements` | Built-in widgets: inputs, forms, menus, toasts, and more |
| `input` | `sequoia-input` | Keyboard, pointer, and IME event types |
| `text` | `sequoia-text` | Text shaping, wrapping, and rich text |
| `draw` | `sequoia-draw` | Vector paths, fills, strokes, and rasterization |
| `render` | `sequoia-render` | `wgpu` renderer and surface management |
| `platform` | `sequoia-platform` | Application loop, windows, dialogs, settings |
| `animation` | `sequoia-animation` | Tweens, keyframes, springs, and state machines |
| `assets` | `sequoia-assets` | Asset registry, icons, fonts, and photo decoding |
| `async_bridge` | `sequoia-async` | Futures bridged to the UI thread, timers, resources |
| `dock` | `sequoia-dock` | Headless dock model for IDE-style panels |
| `buffer` | `sequoia-buffer` | Text buffer engine with cursors and undo |
| `seq` | `sequoia-seq` | `.seq` parser, checker, and lowering to views |
| `export` | `sequoia-export` | Headless HiDPI PNG export |
| `hotkey` | `sequoia-hotkey` | OS-global hotkeys |
| `ui` | `sequoia-ui` | Facade crate and prelude |
| `preview` | `sequoia-preview` | `.seq` preview runner |
| `seq-lsp` | `sequoia-seq-lsp` | Language server for `.seq` |
| `test` | `sequoia-test` | Internal headless test harness |

Additional workspace members are tools and examples rather than framework
modules: `sequoia-seq-lsp`, `sequoia-preview`, and the `example/paint` binary.

## Page template

Use this outline unless the topic needs a different shape:

```text
# Page title

One paragraph explaining what the page covers and why it matters.

## When to use it
## Minimal example
## How it works
## Common options
## Pitfalls
## Related pages
```

Not every heading is mandatory. Omit sections that would be empty, and avoid
duplicating a full API listing.

## Writing style

- Use American English, second person, and present tense.
- Use sentence case for headings: `## Custom views`, not `## Custom Views`.
- Write "Sequoia" in prose and `sequoia_ui` in code.
- Introduce a term before using it; link to the concept page instead of
  repeating an explanation.
- Prefer short paragraphs and task-oriented lists.
- Explain *why* a step matters, not only *what* to type.
- Wrap prose at roughly 80 columns to keep diffs readable.
- Use relative links between pages and absolute links only for external sites.

## Code examples

Rust examples are compiled by `mdbook test` and should stay in sync with the
workspace:

- Use `rust` blocks for complete, compiling code.
- Use `rust,no_run` for applications that open a window or need a GPU.
- Use `text` for `.seq`, TOML, and shell snippets.
- Use hidden `#` lines for imports and boilerplate that distract from the
  example, but keep the visible code runnable in context.
- Use `ignore` only for intentionally incomplete fragments, and say what is
  omitted.
- Prefer APIs re-exported by the prelude in introductory examples. Mention the
  fully qualified path when a type is not in the prelude.

Run the checks from the workspace root after changing examples:

```text
cargo build -p sequoia-ui
mdbook test docs -L target/debug/deps
cargo doc --workspace --no-deps
```

The first command makes the facade crate available to the Rust code blocks.
The last command catches broken intra-doc links and malformed documentation.
When an example depends on an additional crate, add it to the test setup
instead of marking the block as ignored.

## Images

Store images under `docs/src/assets/` and reference them with relative paths.
Use PNG for screenshots and prefer a light or dark UI consistently within one
page. Keep images small and meaningful; a screenshot should show state that is
hard to describe in prose.

## Review checklist

Before committing a page, confirm that:

- [ ] The page is listed in `SUMMARY.md`.
- [ ] Every Rust block compiles or is explicitly marked `no_run`/`ignore`.
- [ ] Commands are copy-pasteable and use the current workspace layout.
- [ ] New public types are mentioned in the relevant guide and in rustdoc.
- [ ] Internal links resolve, and external links point to stable pages.
- [ ] The page explains a task and links to related pages.

## Rustdoc conventions

Rustdoc complements the book. For public modules, types, and functions:

- Start modules with `//!` paragraphs explaining their role.
- Put a short summary on the first line, then details.
- Add an `# Examples` section with a compilable snippet for entry points and
  non-obvious APIs.
- Document panics, errors, and platform limitations in `# Panics`, `# Errors`,
  and `# Platform` sections where applicable.
- Link to the corresponding book page with an intra-doc link when a concept
  needs a longer explanation.
