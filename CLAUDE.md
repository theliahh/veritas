# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Veritas is a damage logger/meter for Honkai: Star Rail, built as a Windows-only `cdylib` (`veritas.dll`) that gets injected into the game process. It hooks game functions via IL2CPP, aggregates battle data, draws an egui overlay on the game's DirectX 11 swapchain, and broadcasts events to external clients over Socket.IO. User-facing install/usage docs live in the GitHub wiki, not in this repo.

## Build & test

- Toolchain is pinned to `nightly-2025-05-17` (`rust-toolchain.toml`); the crate uses nightly features (`windows_process_extensions_show_window`) and edition 2024.
- Build: `cargo build --release` → `target/release/veritas.dll`. This is the only thing CI runs (on PRs to `main`); there is no lint/fmt step in CI. Requires MSVC Build Tools (C++ workload): `build.rs` compiles `src/guard.c` with the `cc` crate.
- `Cargo.toml` has a `[patch]` pointing `edio11` at a local checkout in `../edio11-rs` (branch `fix-thread-race`); the build fails without that directory until the fix is upstreamed and the pinned `rev` bumped.
- Debug builds allocate a console window on injection (`AllocConsole` under `debug_assertions`) and log at Debug level (the debug log grows ~10 MB per session).
- Tests: `tests::egui_main` in `src/lib.rs` runs the overlay UI standalone in an `eframe` window (no game needed; blocks until closed) — `cargo test egui_main`. `cargo test --lib guarded_call` runs the IL2CPP exception-guard test. The DLL entry point is `#[cfg(not(test))]`, so tests don't try to hook anything.
- Debugging against the game: `.vscode/launch.json` has an LLDB "Attach to Process" config.
- Releases: pushing a tag triggers `publish.yml`, which uploads `veritas.dll`; tags on commits in `origin/beta` are marked prerelease. Bump `version` in `Cargo.toml` and add an entry to `CHANGELOG.MD` (it's `include_str!`'d into the binary and shown in the UI/updater).

## Architecture

Data flows one way: **game hook → `Event` → `BattleContext` → `Packet` → (Socket.IO clients + overlay UI reads state)**.

1. **Startup (`entry.rs`)**: a `#[ctor]` spawns `init()` on DLL load. It waits for `GameAssembly`/`UnityPlayer` modules, locates the IL2CPP API table (pattern scan in UnityPlayer: `get_il2cpp_table_offset`), initializes `il2cpp_runtime` with a hardcoded `ApiIndexTable` of export indices, installs hooks, starts the server thread, then initializes the overlay. Hook setup failure is caught (panic + SEH) and reported as a toast, leaving the overlay running with core disabled.
2. **Game bindings (`kreide/`)**: `kreide/types.rs` declares game types using `il2cpp-runtime` proc macros (`#[il2cpp_ref_type("RPG.GameCore.X")]`, `#[il2cpp_field(name = ...)]`, `#[il2cpp_enum_type]`, etc.) — fields/methods resolve by name at runtime. Obfuscated game symbols (e.g. `GBOAGIMFJCK`) appear as-is. `kreide/helpers.rs` has conversion helpers (e.g. `fixpoint_to_raw`).
3. **Hooks (`subscribers/battle.rs`)**: `retour::static_detour!` declares detours; `subscribe()` resolves target methods (sometimes by scanning the type table for an obfuscated class with a matching signature via `find_method`) and registers each with `subscribe_function!`; `enable_subscribers!` enables them all afterwards. Every detour calls the original first, then wraps its logic in `safe_call!` (catches `Result` errors, Rust panics, and hardware SEH faults such as access violations — see the caveat below) and ends by calling `BattleContext::handle_event(...)`. Functions using `safe_call!` must be `#[named]` (`function_name` crate). Hooks run on the game's main thread.
4. **State (`battle.rs`)**: `BattleContext` is a global `Mutex` singleton (`BattleContext::get_instance()`). `handle_event` dispatches each `models::events::Event` to a `handle_on_*_event` method that mutates state and returns a `Packet`, which is sent to `server::broadcast`.
5. **Server (`server.rs`)**: axum + socketioxide on `127.0.0.1:1305`, using the shared tokio `RUNTIME` from `lib.rs`. `Packet` (defined via the `packet!` macro in `models/packets.rs`) emits with its variant name as the event name. **`docs/API.md` documents this wire protocol — update it when changing packets or `models/types.rs`.**
6. **Overlay (`overlay.rs`, `ui/`)**: hooks D3D11 present via the `edio11` crate and renders `ui::app::App` with egui. UI widgets read `BattleContext::get_instance()` directly each frame. Config/data persisted under `directories::ProjectDirs` for `veritas`. Threading: `Present`/`ResizeBuffers` run on Unity's **render thread**, while edio11's WndProc hook (input, and `App::window_process`) runs on the game's **main thread**; edio11 serializes its shared state with a lock that is released before calling back into the game, since each thread can block on the other.

`MANIFEST.md` has a per-module responsibility map.

## Calling into IL2CPP

- IL2CPP throws managed exceptions as MSVC C++ exceptions. **Neither `safe_call!` nor `microseh` can catch them**: one that unwinds into Rust aborts the whole game (`catch_unwind` aborts on foreign exceptions; microseh's `extern "C"` frames abort with `panic_cannot_unwind`). For a game call that can throw, go through `guarded_call2` in `kreide/helpers.rs`, which calls the method pointer from `src/guard.c` inside `__try/__except` and turns the exception into an `Err` carrying the managed exception type and message.
- On game 4.5.0, strings created with `Il2CppString::new` arrive empty in managed code, so `System.Enum.Parse` always throws. Look up enum members with `enum_value(type_name, member)` (a name → value table built via `Enum.ToObject` + `Enum.GetName`), not `Enum.Parse`.
- Unity engine constants (e.g. `RenderTextureFormat`) are hardcoded with their documented values rather than resolved at runtime.

## Game-update fixes

Most commits are "Fixed for X.Y.51" after a game patch. Breakage typically comes from: renamed obfuscated identifiers in `kreide/types.rs` / `subscribers/battle.rs`, changed method signatures in `static_detour!` / `find_method` arg lists, the UnityPlayer byte pattern in `entry.rs`, or shifted indices in the `ApiIndexTable`. The `il2cpp-runtime` git dependency (`hessiser/il2cpp-rust`) is pinned to a `rev` in `Cargo.toml` and sometimes needs updating alongside the game.

Diagnosing a crash in the game:
- The game loads the DLL from `<game dir>\plugins\veritas.dll`; logs are `veritas.log` / `veritas.debug.log` in the game directory (truncated on each launch).
- Windows keeps a dump per crash in `%LOCALAPPDATA%\CrashDumps\StarRail.exe.<pid>.dmp`, and the Application event log records the faulting module and exception code; Unity writes `error.log` stack traces (unsymbolized for `veritas`) under `%LOCALAPPDATA%\Temp\Cognosphere\Star Rail\Crashes\`.
- Exception `0xC0000409` with parameter `7` in `veritas.dll` is a Rust abort — usually a game exception that reached Rust (see above). `0xC0000374` is heap corruption. Symbolize against the `veritas.pdb` of the exact build that crashed.

## Localization

Uses `rust-i18n` with `t!("English text")`. Keys are minified (`minify-key = true`, 12 chars) so `locales/v2.yml` is keyed by hashes, each entry listing all locales. Locale list must stay in sync between `Cargo.toml` `[package.metadata.i18n]` and the `LOCALES` map in `lib.rs`.
