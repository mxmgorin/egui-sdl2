# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-07-21

### Changed

- Updated `egui`, `egui_glow`, and `egui-wgpu` from 0.34 to 0.35.
- Migrated to the egui-wgpu 0.35 API: `WgpuConfiguration`'s `present_mode` and
  `desired_maximum_frame_latency` now live under the new `surface` field, and the new
  `SurfaceErrorAction::Reconfigure` variant is handled.
- Bumped `pollster` (dev) to 1.0, `bytemuck` to 1.25, and `log` to 0.4.33.

### Fixed

- The published crate now includes the `LICENSE-APACHE` and `LICENSE-MIT` files
  (the previous `include` paths pointed outside the package).

### Documentation

- docs.rs now builds with every rendering backend, so the `wgpu` module is documented.
- Rewrote the README and fixed a broken intra-doc link in the glow module.

## [0.4.0] - 2026-06-16

### Added

- `State::get_drawable_size` helper.

### Changed

- `wgpu-backend` is now opt-in; the default features cover the Canvas and glow backends.
  Examples are gated behind their respective backend features.

### Fixed

- glow backend now uses the drawable (physical) size for the GL viewport.

### Performance

- Pointer coordinates are mapped via a cached points-per-pixel value.
- Canvas backend reuses its vertex buffer and de-duplicates clip-rect calls.

## Earlier releases

See the [GitHub releases](https://github.com/mxmgorin/egui-sdl2/releases) and
[tags](https://github.com/mxmgorin/egui-sdl2/tags) for versions 0.3.2 and earlier.

[Unreleased]: https://github.com/mxmgorin/egui-sdl2/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/mxmgorin/egui-sdl2/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/mxmgorin/egui-sdl2/compare/v0.3.2...v0.4.0
