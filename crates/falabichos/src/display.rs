// SPDX-License-Identifier: Apache-2.0
//! The 240x160 frame as one nearest-neighbour sprite with integer scaling and letterboxing,
//! plus the on-screen touch controls.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use falabichos_game::touch_art::{Insets, TouchLayout, dpad_image, pill_button, round_button};
use falabichos_game::{LOGICAL_H, LOGICAL_W};
use falabichos_media::{ArtBank, Canvas};

#[derive(Component)]
pub struct FrameSprite;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum TouchPart {
    Dpad,
    A,
    B,
    Menu,
    Hint,
}

#[derive(Resource)]
pub struct Frame {
    pub handle: Handle<Image>,
    pub canvas: Canvas,
    pub drawn_tick: u64,
}

pub fn make_image(c: &Canvas) -> Image {
    let mut img = Image::new(
        Extent3d {
            width: c.w as u32,
            height: c.h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        c.px.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = ImageSampler::nearest();
    img
}

pub fn frame_resource(images: &mut Assets<Image>) -> Frame {
    let canvas = Canvas::new(LOGICAL_W, LOGICAL_H);
    let handle = images.add(make_image(&canvas));
    Frame {
        handle,
        canvas,
        drawn_tick: u64::MAX,
    }
}

/// Spawn the frame sprite (and touch controls when requested).
pub fn spawn_display(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    frame: &Frame,
    bank: &ArtBank,
    touch: bool,
) {
    commands.spawn((Camera2d, Msaa::Off));
    commands.spawn((
        Sprite::from_image(frame.handle.clone()),
        Transform::default(),
        FrameSprite,
    ));
    if touch {
        // Rendered at a generous resolution and scaled by the layout: crisp on high-DPI phones.
        let dpad = images.add(make_image(&dpad_image(bank, 256)));
        let a = images.add(make_image(&round_button(bank, 128, "A", [232, 85, 60, 150])));
        let b = images.add(make_image(&round_button(bank, 128, "B", [42, 93, 176, 150])));
        let menu = images.add(make_image(&pill_button(bank, 160, 64, "MENU")));
        let hint = images.add(make_image(&pill_button(bank, 160, 64, "HINT")));
        for (part, img) in [
            (TouchPart::Dpad, dpad),
            (TouchPart::A, a),
            (TouchPart::B, b),
            (TouchPart::Menu, menu),
            (TouchPart::Hint, hint),
        ] {
            commands.spawn((Sprite::from_image(img), Transform::from_xyz(0.0, 0.0, 10.0), part));
        }
    }
}

/// Integer physical-pixel scale that fits the frame inside the safe area (never below 1 unless the
/// window is tiny, then fractional so the whole frame stays visible).
pub fn frame_scale(avail_w_px: f32, avail_h_px: f32) -> f32 {
    let fit = (avail_w_px / LOGICAL_W as f32).min(avail_h_px / LOGICAL_H as f32);
    if fit >= 1.0 { fit.floor() } else { fit.max(0.1) }
}

pub fn layout_frame(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut frame_sprite: Query<&mut Transform, (With<FrameSprite>, Without<TouchPart>)>,
    mut touch_sprites: Query<(&TouchPart, &mut Transform, &mut Sprite), Without<FrameSprite>>,
    insets: Res<SafeInsets>,
) {
    let Ok(window) = windows.single() else { return };
    let (w, h, sf) = (window.width(), window.height(), window.scale_factor());
    let safe = insets.0;
    let avail_w = (w - safe.left - safe.right).max(1.0);
    let avail_h = (h - safe.top - safe.bottom).max(1.0);
    let k = frame_scale(avail_w * sf, avail_h * sf);
    if let Ok(mut t) = frame_sprite.single_mut() {
        t.scale = Vec3::new(k / sf, k / sf, 1.0);
        // world origin is the window centre, y up
        t.translation = Vec3::new(
            (safe.left - safe.right) / 2.0,
            (safe.bottom - safe.top) / 2.0,
            0.0,
        );
    }
    let layout = TouchLayout::compute(w, h, safe, sf);
    for (part, mut t, mut sprite) in touch_sprites.iter_mut() {
        let r = match part {
            TouchPart::Dpad => layout.dpad,
            TouchPart::A => layout.a,
            TouchPart::B => layout.b,
            TouchPart::Menu => layout.menu,
            TouchPart::Hint => layout.hint,
        };
        let (cx, cy) = r.center();
        t.translation = Vec3::new(cx - w / 2.0, h / 2.0 - cy, 10.0);
        sprite.custom_size = Some(Vec2::new(r.w, r.h));
    }
}

#[derive(Resource, Clone, Copy)]
pub struct SafeInsets(pub Insets);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_scaling() {
        assert_eq!(frame_scale(960.0, 640.0), 4.0);
        assert_eq!(frame_scale(1000.0, 700.0), 4.0, "floor to an integer");
        assert_eq!(frame_scale(2400.0, 1200.0), 7.0);
        assert_eq!(frame_scale(240.0, 160.0), 1.0);
        let tiny = frame_scale(120.0, 80.0);
        assert!(
            tiny > 0.0 && tiny < 1.0,
            "fractional only when the window is smaller than the frame"
        );
    }
}
