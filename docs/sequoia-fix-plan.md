# Plan poprawek Sequoii (na podstawie audit-u z 22.11.2026)

Status: do realizacji. Decyzje użytkownika z 22.11.2026:
- automatyczne owijanie wielu dzieci w `Column` w `single_child` **zostaje**, dodajemy dokumentację;
- integrację `GpuSurface`/`AshSurface` **naprawiamy**, nie wycofujemy z API.

Bramki po każdej fazie: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, testy workspace’u Sequoii,
testy workspace’u Loom, uruchomienie `cargo run -p loom_editor` z działającym viewportem.

---

## Faza 1 — stos i limity loweringu (P0)

Problem: w buildzie debug `lower_node_inner` ma ramkę ~140 KB na poziom zagnieżdżenia.
7 poziomów przepełnia 1 MB stosu głównego wątku. Tymczasowe obejście
(`[profile.dev.package.sequoia-seq] opt-level = 1`) jest w `Cargo.toml` Sequoii i Loom.

- [ ] Rozbić `crates/sequoia-seq/src/lower.rs:1715` (`lower_node_inner`, ~1100 linii)
      na mniejsze funkcje per-typ: `lower_container`, `lower_text`, `lower_canvas`,
      `lower_stack` itd. Każda z własnymi lokalami; `#[inline(never)]` tylko jeśli
      profilowanie potwierdzi zysk.
- [ ] Dodać `MAX_LOWER_DEPTH = 64` w `lower_node` (`lower.rs:1459`), analogicznie do
      `MAX_COMP_DEPTH = 32` (`lower.rs:142`). Przekroczenie zwraca `SeqError`,
      nie przepełnienie stosu.
- [ ] `single_child` (`lower.rs:9074`): zostawić automatyczne owijanie >1 dzieci w
      `Column`; dodać komentarz `///` i akapit w `docs/src/guides/seq.md`
      wyjaśniający tę semantykę i jej wpływ na zagnieżdżenia.
- [ ] Dodać test regresyjny w `crates/sequoia-test/tests/seq_layout.rs` (lub nowy
      `seq_stack.rs`): 10–12 zagnieżdżonych `Container`/`Column`, elementy `dynamic:`,
      uruchomione na głównym wątku o 1 MB (bez `RUST_MIN_STACK`), asercja
      `SeqLoader::load_str_with(...).is_ok()`.
- [ ] Po przejściu testu usunąć `opt-level = 1` z `Cargo.toml` Sequoii i z
      `C:\Users\Admin\Documents\Loom\Cargo.toml`. Zostawić tylko jeśli test nadal
      wymaga polisy (wtedy udokumentować dlaczego).

## Faza 2 — surface i backend (P0/P1)

Problem: `GpuSurface`/`AshSurface` psuje rysowanie pozostałego drzewa UI w edytorze;
Loom musiał obejść to przez offscreen renderer. `App::run` nadpisuje backend zmienną
środowiskową (`RenderBackend::from_env()`), a tick viewportu korzysta z
`on_frame_metrics`.

- [ ] Dodać przykład/test reprodukcyjny w `crates/sequoia-ui/examples`: okno z
      `AshSurface`/`GpuSurface` + zwykłe widgety + golden screenshot; ustalić moment,
      w którym surface „zjada” rysowanie UI.
- [ ] Diagnoza i naprawa w `crates/sequoia-render-ash/src` oraz
      `crates/sequoia-platform/src`: sprawdzić stan render passu, viewport/scissor,
      kolejność poleceń i przywrócić stan przed rysowaniem reszty drzewa.
- [ ] `crates/sequoia-platform/src/lib.rs:1023`: `RenderBackend::from_env()` przestaje
      nadpisywać jawną konfigurację. Dodać `App::backend_from_env()` jako wyraźny,
      opcjonalny wybór.
- [ ] Dodać `App::on_frame(|dt| …)` jako osobny hook logiki klatki;
      `on_frame_metrics` (`lib.rs:1002`) zostawić wyłącznie do metryk.
- [ ] W Loom przenieść tick viewportu z `on_frame_metrics` na `on_frame`
      (`crates/loom_editor/src/main.rs`).

## Faza 3 — higiena API (P1)

- [ ] Ujednolicić komunikaty na angielski: `lower.rs` (92 trafienia),
      `check.rs` (11), `eval.rs` (11). Zaktualizować asercje w testach oczekujących
      polskich stringów (`seq_fn_errors.rs`, `seq_imports_parse.rs` i pokrewne).
- [ ] Dokończyć albo usunąć `HostBindings::register_instance` / `instance_ids`
      (`crates/sequoia-seq/src/bindings.rs:186,194`) — martwy kod, warning dead_code.
- [ ] `cargo fmt --all` w repozytorium Sequoia i pełny `clippy -D warnings`.

## Faza 4 — GPU i CI (P1)

- [ ] Dodać test z validation layer: tworzenie i niszczenie `SpritePipeline`,
      command bufferów, render passów i image view; bramka: zero leaked objects.
      Test wyłapał już brak `destroy_descriptor_set_layout` w `rune_backend/src/draw.rs`
      oraz złą kolejność dropów w `loom_viewport/src/offscreen.rs`.
- [ ] Wprowadzić zasadę „dzieci przed device” w `VulkanContext` (asercja w `Drop`,
      że nie pozostały child handles).
- [ ] Dodać golden testy layoutu seq (zagnieżdżenia, `dynamic:`) i dołączyć je do CI
      obok testów determinizmu.

---

## Kontekst — już zrobione (nie powtarzać)

- `1bd1ee2` (Sequoia): `opt-level = 1` dla `sequoia-seq` w profilu dev.
- `f15c90e` (Loom): to samo ustawienie w głównym workspace’ie (Cargo czyta profile
  tylko z roota).
- `ebd9e2b` (Loom): spłaszczenie inspectora w `interface/editor.seq` (obejście
  problemu stosu; może zostać jako profilaktyka).
- `27492b6` (Loom): poprawa kolejności dropów w `loom_viewport/src/offscreen.rs`
  i dodanie `destroy_descriptor_set_layout` w `rune_backend/src/draw.rs`.
