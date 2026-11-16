# Open Water Implementation Plan

> **For agentic workers:** Use the approved specification and execute the steps in
> order, checking each deliverable before continuing. The user has authorized
> implementation in this session.

**Goal:** Make the approved close-to-water, rich ASCII sea the default Rust scene.

**Architecture:** Keep the crossterm lifecycle and changed-cell renderer. Isolate
the procedural 3D surface, camera, intersection, and shading in `src/ocean.rs`;
let `src/scene.rs` construct frames and apply palettes.

**Tech Stack:** Rust 2024, existing clap/crossterm/ctrlc dependencies.

**Spec:** `docs/superpowers/specs/2026-11-16-open-water-design.md`.

## Global Constraints

- Always ASCII, no celestial objects or birds; default `terminal`, `low`, `rich`.
- Preserve terminal cleanup and bounded argument parsing; no new dependencies.
- Match the approved preview at seed 42 and the same dimensions/time.

## Review Focus

- Port fidelity: compare against a captured frame from the approved renderer.
- Geometry: first visible intersection and normals must agree with the surface.
- Intensity zero and maximum intensity must avoid division errors or submerged cameras.
- Empty/tiny frames and resizing while paused must remain safe.
- Viewpoint/detail/theme controls must not reset animation time.

## Tasks

- [x] Capture a `91 x 32`, time `5`, low/rich reference frame in
  `tests/fixtures/open-water-low-rich.txt`; add CLI regression tests and see them fail.
- [x] Add `Detail`, `Viewpoint`, and the ocean palette default in `src/options.rs`.
  Implement `Ocean::new(seed, time, sea, intensity, detail)`, `Camera::new`, and
  `Ocean::pixel` in `src/ocean.rs`, consumed by `Scene::render`.
- [x] Replace the old sun/wave renderer; verify reference parity, first-hit
  intersections, normals, flat water, extreme settings, and tiny frame dimensions.
- [x] Connect `v`/`d` controls and HUD/help in `src/app.rs`, use the same settings
  for snapshots in `src/main.rs`, and cover these controls in the PTY check.
- [x] Update README and its preview to the shipped scene.
- [x] Run formatting, Clippy, the full Rust suite, the release build, Linux PTY
  checks, and a representative render-time measurement; inspect the rendered image.
- [x] Apply the subsequent request to follow terminal colors by default, remove
  background painting from every theme and overlay, and verify palette switching.

## Validation

- Formatting and Clippy with warnings denied passed.
- All 21 Rust tests passed, including exact parity with the approved preview.
- Release build and Linux PTY controls, resize, and terminal restoration passed.
- A 120 x 40 rich snapshot with the original ocean palette took 7.04 ms median
  over 24 runs, including startup.
- Inspected `docs/preview.png`, rendered from the release binary.
