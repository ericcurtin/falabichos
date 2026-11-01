// SPDX-License-Identifier: Apache-2.0
//! Audio: procedural chiptune music/SFX (rendered at runtime by `falabichos-media`) and optional
//! recorded pronunciation clips (`assets/audio/pt-PT/*.ogg`). Missing clips are silently skipped.

use bevy::audio::{AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;
use falabichos_assets::Assets as AssetBundle;
use falabichos_core::save::Settings;
use falabichos_game::GameEvent;
use falabichos_media::synth::{SAMPLE_RATE, Song, cry, wav_bytes};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Component)]
pub struct MusicMarker;

#[derive(Resource)]
pub struct Sound {
    assets: Arc<AssetBundle>,
    cache: HashMap<String, Option<(Handle<AudioSource>, bool)>>,
    music: Option<Entity>,
    music_name: Option<String>,
}

fn gain(v: u8) -> f32 {
    let x = v.min(10) as f32 / 10.0;
    x * x * 0.7
}

impl Sound {
    pub fn new(assets: Arc<AssetBundle>) -> Sound {
        Sound {
            assets,
            cache: HashMap::new(),
            music: None,
            music_name: None,
        }
    }

    /// Render (once) and cache a song / sfx by name. Returns the handle and whether it loops.
    fn song(&mut self, name: &str, sources: &mut Assets<AudioSource>) -> Option<(Handle<AudioSource>, bool)> {
        if let Some(c) = self.cache.get(name) {
            return c.clone();
        }
        let entry = self
            .assets
            .text(&format!("music/{name}.song"))
            .and_then(|t| Song::parse(t).ok())
            .map(|s| {
                let wav = wav_bytes(&s.render(), SAMPLE_RATE);
                (sources.add(AudioSource { bytes: wav.into() }), s.looped)
            });
        self.cache.insert(name.to_string(), entry.clone());
        entry
    }

    fn cry(&mut self, hz: u16, sources: &mut Assets<AudioSource>) -> Handle<AudioSource> {
        let key = format!("cry:{hz}");
        if let Some(Some((h, _))) = self.cache.get(&key) {
            return h.clone();
        }
        let h = sources.add(AudioSource {
            bytes: wav_bytes(&cry(hz), SAMPLE_RATE).into(),
        });
        self.cache.insert(key, Some((h.clone(), false)));
        h
    }

    fn clip(&mut self, name: &str, sources: &mut Assets<AudioSource>) -> Option<Handle<AudioSource>> {
        let key = format!("clip:{name}");
        if let Some(c) = self.cache.get(&key) {
            return c.as_ref().map(|(h, _)| h.clone());
        }
        let entry = self
            .assets
            .get(&format!("audio/pt-PT/{name}.ogg"))
            .map(|b| (sources.add(AudioSource { bytes: Arc::from(b) }), false));
        self.cache.insert(key, entry.clone());
        entry.map(|(h, _)| h)
    }

    pub fn handle(
        &mut self,
        ev: &GameEvent,
        commands: &mut Commands,
        sources: &mut Assets<AudioSource>,
        settings: &Settings,
    ) {
        match ev {
            GameEvent::Sfx(name) => {
                if settings.sfx_volume == 0 {
                    return;
                }
                if let Some((h, _)) = self.song(name, sources) {
                    commands.spawn((
                        AudioPlayer::new(h),
                        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(gain(settings.sfx_volume))),
                    ));
                }
            }
            GameEvent::Cry(hz) if settings.sfx_volume > 0 => {
                let h = self.cry(*hz, sources);
                commands.spawn((
                    AudioPlayer::new(h),
                    PlaybackSettings::DESPAWN.with_volume(Volume::Linear(gain(settings.sfx_volume))),
                ));
            }
            GameEvent::Clip(name) => {
                if let Some(h) = self.clip(name, sources) {
                    commands.spawn((
                        AudioPlayer::new(h),
                        PlaybackSettings::DESPAWN
                            .with_volume(Volume::Linear(gain(settings.sfx_volume.max(4)))),
                    ));
                }
            }
            GameEvent::Music(name) => {
                if let Some(e) = self.music.take() {
                    commands.entity(e).despawn();
                }
                self.music_name = name.clone();
                if let Some(n) = name
                    && let Some((h, looped)) = self.song(n, sources)
                {
                    let base = if looped {
                        PlaybackSettings::LOOP
                    } else {
                        PlaybackSettings::DESPAWN
                    };
                    let e = commands
                        .spawn((
                            AudioPlayer::new(h),
                            base.with_volume(Volume::Linear(gain(settings.music_volume))),
                            MusicMarker,
                        ))
                        .id();
                    self.music = Some(e);
                }
            }
            _ => {}
        }
    }

    pub fn apply_volume(&self, sinks: &mut Query<&mut AudioSink, With<MusicMarker>>, settings: &Settings) {
        for mut s in sinks.iter_mut() {
            s.set_volume(Volume::Linear(gain(settings.music_volume)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_is_monotonic_and_bounded() {
        assert_eq!(gain(0), 0.0);
        assert!(gain(5) < gain(6));
        assert!(gain(10) <= 0.7 + 1e-6);
        assert_eq!(gain(200), gain(10));
    }
}
