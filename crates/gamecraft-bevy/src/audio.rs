//! Sound for the Bevy games (feature `audio`).
//!
//! The clips are synthesised at startup by [`crate::synth`], so there are no asset
//! files. Each game gets a small plugin: it builds its clips once, then every frame asks
//! the game's listener ([`crate::cues`]) what is new and plays one clip per cue. With no
//! sound device Bevy logs a warning and plays nothing; the games still run.

use crate::cues::{Cue, FlockEars, MazeEars};
use crate::maze::MazeRun;
use crate::murmuration::FlockWorld;
use crate::synth;
use bevy::audio::Volume;
use bevy::prelude::*;
use std::collections::HashMap;

/// The synthesised clips of one game, by cue.
#[derive(Resource, Default)]
struct Clips(HashMap<Cue, Handle<AudioSource>>);

impl Clips {
    fn add(&mut self, assets: &mut Assets<AudioSource>, cue: Cue, samples: &[f32]) {
        let bytes = synth::wav_bytes(samples);
        self.0.insert(
            cue,
            assets.add(AudioSource {
                bytes: bytes.into(),
            }),
        );
    }

    fn play(&self, commands: &mut Commands, cue: Cue) {
        if let Some(clip) = self.0.get(&cue) {
            commands.spawn((
                AudioPlayer::new(clip.clone()),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume(cue))),
            ));
        }
    }
}

/// Loudness of each cue relative to full scale.
fn volume(cue: Cue) -> f32 {
    match cue {
        Cue::Screech => 0.5,
        Cue::Flare | Cue::Loss => 0.7,
        Cue::Win => 0.8,
        Cue::Step => 0.35,
        Cue::Bump => 0.6,
    }
}

/// Footsteps, bumps and the win chime of the maze.
#[derive(Debug, Clone, Copy, Default)]
pub struct MazeAudioPlugin;

impl Plugin for MazeAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_maze)
            .add_systems(Update, react_maze);
    }
}

fn load_maze(mut commands: Commands, mut assets: ResMut<Assets<AudioSource>>) {
    let mut clips = Clips::default();
    clips.add(&mut assets, Cue::Step, &synth::blip());
    clips.add(&mut assets, Cue::Bump, &synth::bump());
    clips.add(&mut assets, Cue::Win, &synth::win_chime());
    commands.insert_resource(clips);
}

fn react_maze(
    run: Res<MazeRun>,
    clips: Option<Res<Clips>>,
    mut ears: Local<MazeEars>,
    mut commands: Commands,
) {
    let Some(clips) = clips else {
        return;
    };
    for cue in ears.listen(&run) {
        clips.play(&mut commands, cue);
    }
}

/// Evening wind under the flock, hawk cries, the flare and the result.
#[derive(Debug, Clone, Copy, Default)]
pub struct MurmurationAudioPlugin;

impl Plugin for MurmurationAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_murmuration)
            .add_systems(Update, react_murmuration);
    }
}

fn load_murmuration(mut commands: Commands, mut assets: ResMut<Assets<AudioSource>>) {
    let mut clips = Clips::default();
    clips.add(&mut assets, Cue::Screech, &synth::screech());
    clips.add(&mut assets, Cue::Flare, &synth::flare());
    clips.add(&mut assets, Cue::Win, &synth::win_chime());
    clips.add(&mut assets, Cue::Loss, &synth::loss_tone());
    commands.insert_resource(clips);
    let bed = assets.add(AudioSource {
        bytes: synth::wav_bytes(&synth::ambient()).into(),
    });
    commands.spawn((
        AudioPlayer::new(bed),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.3)),
    ));
}

fn react_murmuration(
    world: Res<FlockWorld>,
    clips: Option<Res<Clips>>,
    mut ears: Local<FlockEars>,
    mut commands: Commands,
) {
    let Some(clips) = clips else {
        return;
    };
    for cue in ears.listen(&world.0.stats(), world.0.flare_age) {
        clips.play(&mut commands, cue);
    }
}

/// Should the game stay silent? `--mute`, `GAMECRAFT_MUTE=1`, or a gate run
/// (`--exit-on-win`), so automated runs never blast the room.
#[must_use]
pub fn muted(args: &[String]) -> bool {
    args.iter().any(|a| a == "--mute" || a == "--exit-on-win")
        || std::env::var("GAMECRAFT_MUTE").is_ok_and(|v| v == "1")
}
