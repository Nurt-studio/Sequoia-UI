# Introduction

Sequoia is a native, reactive GUI framework for Rust built on
[`wgpu`](https://wgpu.rs). It uses a retained view tree and fine-grained
signals: there is no virtual DOM and no embedded web view.

The framework ships as a family of crates behind a single facade,
`sequoia-ui`, so an application can depend on one package and reach the whole
stack through `sequoia_ui::prelude`.

## What you can build

- Desktop applications with native windows, GPU rendering, and OS integration.
- Declarative UIs in Rust, or in the bundled `.seq` language.
- IDE-style layouts with docks, tabs, split panels, and floating windows.
- Animated interfaces with springs, timelines, and state-driven transitions.
- Async workflows bridged back to the UI thread without a separate runtime.
- Headless screenshots and tests without opening a window.

## How this book is organized

The book is a set of focused guides rather than a single tutorial:

- **Getting started** walks through installation and a first application.
- **Core concepts** explains reactivity, the view tree, layout, events, and
  styling.
- **Guides** cover individual subsystems such as widgets, animation, assets,
  `.seq`, docking, and testing.
- **Cookbook** collects complete recipes for common tasks.
- **Contributing** documents how to write and verify pages in this book.

API reference documentation lives on [docs.rs](https://docs.rs/sequoia-ui) and
in the comments of the source tree.

## Prerequisites

- Rust 1.88 or newer.
- A platform supported by `winit` and `wgpu` with a working GPU driver.
- No web toolchain is required.

## Version

This book targets Sequoia `0.1.x`. APIs may still change between minor
releases; every example is written for the version in the repository.

## Getting help

- Open an issue in the
  [Sequoia repository](https://github.com/Nurt-studio/Sequoia-UI/issues).
- Read the crate documentation on [docs.rs](https://docs.rs/sequoia-ui).
