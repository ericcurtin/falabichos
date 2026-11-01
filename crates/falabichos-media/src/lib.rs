// SPDX-License-Identifier: Apache-2.0
//! Pixel art, fonts, a software canvas and a chiptune synthesiser, all engine-agnostic.
//!
//! Everything visual is defined in compact text files under `assets/art/` (see
//! `docs/ART_PIPELINE.md`) and rendered by this crate into an RGBA [`canvas::Canvas`], so the
//! game can be screenshotted and tested without a GPU.

pub mod bank;
pub mod canvas;
pub mod font;
pub mod sprite;
pub mod synth;

pub use bank::ArtBank;
pub use canvas::{Canvas, Rgba};
pub use sprite::{Palette, Sprite};
