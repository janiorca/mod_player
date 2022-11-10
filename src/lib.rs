//! # mod_player
//!
//! mod_player is a crate that reads and plays back mod audio files. The mod_player decodes the audio one sample pair (left,right) at a time
//! that can be streamed to an audio device or a file.
//!
//! For playback, only two functions are needed;
//! * read_mod_file to read the file into a Song structure
//! * next_sample to get the next sample
//!
//! To use the library to decode a mod file and save it to disk ( using the hound audio crate for WAV saving )
//!
//! ```rust
//! use hound;
//!  
//! fn main() {
//!     let spec = hound::WavSpec {
//!     channels: 2,
//!         sample_rate: 48100,
//!         bits_per_sample: 32,
//!         sample_format: hound::SampleFormat::Float,
//!     };
//!
//!     let mut writer = hound::WavWriter::create( "out.wav", spec).unwrap();
//!     let song = mod_player::read_mod_file("mod_files/BUBBLE_BOBBLE.MOD");
//!     let mut player_state : mod_player::PlayerState = mod_player::PlayerState::new(
//!                                 song.format.num_channels, spec.sample_rate );
//!     loop {
//!         let ( left, right ) = mod_player::next_sample(&song, &mut player_state);
//!         writer.write_sample( left  );
//!         writer.write_sample( right  );
//!         if player_state.song_has_ended || player_state.has_looped {
//!             break;
//!         }
//!     }
//!  }
//! ```

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

mod loader;
pub use loader::read_mod_file;
pub use loader::read_mod_file_slice;
mod static_tables;
pub mod textout;

const CLOCK_TICKS_PERS_SECOND: f32 = 3579545.0; // Amiga hw clcok ticks per second
                                                // const CLOCK_TICKS_PERS_SECOND: f32 = 3579545.0; // NTSC

fn fine_tune_period(period: u32, fine_tune: u32, use_fine_tune_table: bool) -> u32 {
    if use_fine_tune_table {
        let index: i32 = static_tables::FREQUENCY_TABLE
            .binary_search(&period)
            .expect("Unexpected period value") as i32;
        return static_tables::FINE_TUNE_TABLE[fine_tune as usize][index as usize];
    } else {
        return (period as f32 * static_tables::SCALE_FINE_TUNE[fine_tune as usize]) as u32;
    }
}

/// Holds the info and sample data for a sample
pub struct Sample {
    name: String,
    size: u32,
    volume: u8,
    fine_tune: u8,
    repeat_offset: u32,
    repeat_size: u32,
    samples: Vec<i8>,
}

impl Sample {
    fn new(sample_info: &[u8]) -> Sample {
        let sample_name = String::from_utf8_lossy(&sample_info[0..22]);
        let sample_size: u32 = ((sample_info[23] as u32) + (sample_info[22] as u32) * 256) * 2;
        let fine_tune = sample_info[24];
        let volume = sample_info[25];

        // the repeat offset appears to be in bytes ...
        let mut repeat_offset: u32 =
            ((sample_info[27] as u32) + (sample_info[26] as u32) * 256) * 2;
        // .. but the size is in word?
        let repeat_size: u32 = ((sample_info[29] as u32) + (sample_info[28] as u32) * 256) * 2;

        if sample_size > 0 {
            if repeat_offset + repeat_size > sample_size {
                repeat_offset -= (repeat_offset + repeat_size) - sample_size;
            }
        }

        Sample {
            name: String::from(sample_name),
            size: sample_size,
            volume: volume,
            fine_tune: fine_tune,
            repeat_offset: repeat_offset,
            repeat_size: repeat_size,
            samples: Vec::new(),
        }
    }
}

enum Effect {
    /*  SetPanningPosition = 8,
     */
    None, // 0
    Arpeggio {
        chord_offset_1: u8,
        chord_offset_2: u8,
    },
    SlideUp {
        speed: u8,
    }, // 1
    SlideDown {
        speed: u8,
    }, // 2
    TonePortamento {
        speed: u8,
    }, // 3
    Vibrato {
        speed: u8,
        amplitude: u8,
    }, // 4
    TonePortamentoVolumeSlide {
        volume_change: i8,
    }, //5
    VibratoVolumeSlide {
        volume_change: i8,
    }, // 6
    Tremolo {
        speed: u8,
        amplitude: u8,
    }, // 7
    Pan {
        position: u8,
    }, // 8
    SetSampleOffset {
        offset: u8,
    }, // 9
    VolumeSlide {
        volume_change: i8,
    }, // 10
    PositionJump {
        next_pattern: u8,
    }, // 11,
    SetVolume {
        volume: u8,
    }, // 12
    PatternBreak {
        next_pattern_pos: u8,
    }, //13
    SetSpeed {
        speed: u8,
    }, // 15

    SetHardwareFilter {
        new_state: u8,
    }, //E0
    FinePortaUp {
        period_change: u8,
    }, //E1
    FinePortaDown {
        period_change: u8,
    }, //E2
    Glissando {
        use_smooth_slide: bool,
    }, //E2
    PatternLoop {
        arg: u8,
    }, //E6
    TremoloWaveform {
        wave: u8,
    }, // E7
    CoarsePan {
        pan_pos: u8,
    }, //E8
    RetriggerSample {
        retrigger_delay: u8,
    }, //E9
    FineVolumeSlideUp {
        volume_change: u8,
    }, //EA
    FineVolumeSlideDown {
        volume_change: u8,
    }, //EB
    CutNote {
        delay: u8,
    }, //EC
    DelayedSample {
        delay_ticks: u8,
    }, //ED
    DelayedLine {
        delay_ticks: u8,
    }, //EE
    InvertLoop {
        loop_position: u8,
    }, //EF
    SetVibratoWave {
        wave: u8,
    },
    SetFineTune {
        fine_tune: u8,
    },
}

impl Effect {
    fn new(effect_number: u8, effect_argument: i8) -> Effect {
        match effect_number {
            0 => match effect_argument {
                0 => Effect::None,
                _ => Effect::Arpeggio {
                    chord_offset_1: effect_argument as u8 >> 4,
                    chord_offset_2: effect_argument as u8 & 0x0f,
                },
                //                _ => panic!( format!( "unhandled arpeggio effect: {}", effect_number ) )
            },
            1 => Effect::SlideUp {
                speed: effect_argument as u8,
            }, // decrease period, increase frequency, higher note
            2 => Effect::SlideDown {
                speed: effect_argument as u8,
            },
            3 => Effect::TonePortamento {
                speed: effect_argument as u8,
            },
            4 => Effect::Vibrato {
                speed: effect_argument as u8 >> 4,
                amplitude: effect_argument as u8 & 0x0f,
            },
            5 => {
                if (effect_argument as u8 & 0xf0) != 0 {
                    Effect::TonePortamentoVolumeSlide {
                        volume_change: effect_argument >> 4,
                    }
                } else {
                    Effect::TonePortamentoVolumeSlide {
                        volume_change: -effect_argument,
                    }
                }
            }
            6 => {
                if (effect_argument as u8 & 0xf0) != 0 {
                    Effect::VibratoVolumeSlide {
                        volume_change: effect_argument >> 4,
                    }
                } else {
                    Effect::VibratoVolumeSlide {
                        volume_change: -effect_argument,
                    }
                }
            }
            7 => Effect::Tremolo {
                speed: effect_argument as u8 >> 4,
                amplitude: effect_argument as u8 & 0x0f,
            },
            8 => Effect::Pan {
                position: effect_argument as u8,
            },
            9 => Effect::SetSampleOffset {
                offset: effect_argument as u8,
            },
            10 => {
                if (effect_argument as u8 & 0xf0) != 0 {
                    Effect::VolumeSlide {
                        volume_change: effect_argument >> 4,
                    }
                } else {
                    Effect::VolumeSlide {
                        volume_change: -effect_argument,
                    }
                }
            }
            11 => Effect::PositionJump {
                next_pattern: effect_argument as u8,
            },
            12 => Effect::SetVolume {
                volume: effect_argument as u8,
            },
            13 => Effect::PatternBreak {
                next_pattern_pos: (((0xf0 & (effect_argument as u32)) >> 4) * 10
                    + (effect_argument as u32 & 0x0f)) as u8,
            },
            14 => {
                let extended_effect = (effect_argument as u8) >> 4;
                let extended_argument = (effect_argument as u8) & 0x0f;
                match extended_effect {
                    0 => Effect::SetHardwareFilter {
                        new_state: extended_argument as u8,
                    },
                    1 => Effect::FinePortaUp {
                        period_change: extended_argument as u8,
                    },
                    2 => Effect::FinePortaDown {
                        period_change: extended_argument as u8,
                    },
                    3 => Effect::Glissando {
                        use_smooth_slide: extended_argument != 0,
                    },
                    4 => Effect::SetVibratoWave {
                        wave: extended_argument,
                    },
                    5 => Effect::SetFineTune {
                        fine_tune: extended_argument,
                    },
                    6 => Effect::PatternLoop {
                        arg: extended_argument as u8,
                    },
                    7 => Effect::TremoloWaveform {
                        wave: extended_argument as u8,
                    },
                    8 => Effect::CoarsePan {
                        pan_pos: extended_argument as u8,
                    },
                    9 => Effect::RetriggerSample {
                        retrigger_delay: extended_argument as u8,
                    },
                    10 => Effect::FineVolumeSlideUp {
                        volume_change: extended_argument as u8,
                    },
                    11 => Effect::FineVolumeSlideDown {
                        volume_change: extended_argument as u8,
                    },
                    12 => Effect::CutNote {
                        delay: extended_argument as u8,
                    },
                    13 => Effect::DelayedSample {
                        delay_ticks: extended_argument as u8,
                    },
                    14 => Effect::DelayedLine {
                        delay_ticks: extended_argument as u8,
                    },
                    15 => Effect::InvertLoop {
                        loop_position: extended_argument as u8,
                    },
                    _ => panic!(format!(
                        "unhandled extended effect number: {}",
                        extended_effect
                    )),
                }
            }
            15 => Effect::SetSpeed {
                speed: effect_argument as u8,
            },
            _ => panic!(format!("unhandled effect number: {}", effect_number)),
        }
    }
}

/// Describes what sound sample to play and an effect (if any) that should be applied.
pub struct Note {
    sample_number: u8,
    period: u32,
    /// how many clock ticks each sample is held for
    effect: Effect,
}

fn change_note(current_period: u32, change: i32) -> u32 {
    // find note in frequency table
    let mut result = current_period as i32 + change;
    if result > 856 {
        result = 856;
    }
    if result < 113 {
        result = 113;
    }
    result as u32
}

impl Note {
    fn new(note_data: &[u8], format_description: &FormatDescription) -> Note {
        let mut sample_number = ((note_data[2] & 0xf0) >> 4) + (note_data[0] & 0xf0);
        if format_description.num_samples == 15 {
            sample_number = sample_number & 0x0f;
        } else {
            sample_number = sample_number & 0x1f;
        }
        let period = ((note_data[0] & 0x0f) as u32) * 256 + (note_data[1] as u32);
        let effect_argument = note_data[3] as i8;
        let effect_number = note_data[2] & 0x0f;
        let effect = Effect::new(effect_number, effect_argument);

        Note {
            sample_number,
            period,
            effect,
        }
    }
}

pub struct Pattern {
    lines: Vec<Vec<Note>>, // outer vector is the lines (64). Inner vector holds the notes for the line
}

impl Pattern {
    fn new() -> Pattern {
        let mut lines: Vec<Vec<Note>> = Vec::new();
        for _line in 0..64 {
            lines.push(Vec::new());
        }
        Pattern { lines }
    }
}

/// The features of the song
pub struct FormatDescription {
    pub num_channels: u32,
    pub num_samples: u32,
    /// Is the format description based on a tag. Most mod file have a tag descriptor that makes it possible to identify the file with some
    /// certainty. The very earliest files do not have a tag but are assumed to support 4 channels and 15 samples.
    pub has_tag: bool,
}

/// Contains the entire mod song
pub struct Song {
    /// The name of the song as specified in the mod file    
    pub name: String,
    /// Features of the song
    pub format: FormatDescription,
    /// The audio samples used by the song
    pub samples: Vec<Sample>,
    /// Patterns contain all the note data
    pub patterns: Vec<Pattern>,
    /// Specifies the order in whcih the patterns should be played in. The same pattern may played several times in the same song
    pub pattern_table: Vec<u8>,
    /// How many patterns in the pattern table should be used. The pattern table is a fixed length can usually longer than the song
    pub num_used_patterns: u32,
    /// Which pattern should be played after the last pattern in the pattern_table. Used for infinitely looping repeating songs
    pub end_position: u32,
    /// Set to true if all the notes are standard notes (i.e. conforming to the standard period table)
    pub has_standard_notes: bool,
}

struct ChannelInfo {
    sample_num: u8, // which sample is playing
    sample_pos: f32,
    period: u32, //
    fine_tune: u32,
    size: u32,
    volume: f32,        // max 1.0
    volume_change: f32, // max 1.0
    note_change: i32,
    period_target: u32,     // note portamento target
    last_porta_speed: i32,  // last portamento to note and speed parameters
    last_porta_target: u32, // must be tracked separately to porta up and down ( and these may be referred to several lines later)

    base_period: u32, // the untuned period. The same value as the last valid note period value
    vibrato_pos: u32,
    vibrato_speed: u32,
    vibrato_depth: i32,

    tremolo_pos: u32,
    tremolo_speed: u32,
    tremolo_depth: i32,

    retrigger_delay: u32,
    retrigger_counter: u32,

    cut_note_delay: u32,
    arpeggio_counter: u32,
    arpeggio_offsets: [u32; 2],
}

impl ChannelInfo {
    fn new() -> ChannelInfo {
        ChannelInfo {
            sample_num: 0,
            sample_pos: 0.0,
            period: 0,
            fine_tune: 0,
            size: 0,
            volume: 0.0,
            volume_change: 0.0,
            note_change: 0,
            period_target: 0,
            last_porta_speed: 0,
            last_porta_target: 0,

            base_period: 0,
            vibrato_pos: 0,
            vibrato_speed: 0,
            vibrato_depth: 0,

            tremolo_pos: 0,
            tremolo_speed: 0,
            tremolo_depth: 0,

            retrigger_delay: 0,
            retrigger_counter: 0,
            cut_note_delay: 0,
            arpeggio_counter: 0,
            arpeggio_offsets: [0, 0],
        }
    }
}
/// Keeps track of all the dynamic state required for playing the song.
pub struct PlayerState {
    channels: Vec<ChannelInfo>,
    // where in the pattern table are we currently
    pub song_pattern_position: u32,
    /// current line position in the pattern. Every pattern has 64 lines
    pub current_line: u32,
    /// set when the song stops playing
    pub song_has_ended: bool,
    /// set when the song loops. The player does not unset this flag after it has been set. To detect subsequent loops the flag to be manually unset by the client
    pub has_looped: bool,
    device_sample_rate: u32,
    song_speed: u32,                    // in vblanks
    current_vblank: u32,                // how many vblanks since last play line
    samples_per_vblank: u32,            // how many device samples per 'vblank'
    clock_ticks_per_device_sample: f32, // how many amiga hardware clock ticks per device sample
    current_vblank_sample: u32, // how many device samples have we played for the current 'vblank'

    next_pattern_pos: i32, // on  next line if == -1 do nothing else  go to next pattern on line next_pattern_pos
    next_position: i32, // on next line if == 1 do nothing else go to beginning of the this pattern
    delay_line: u32,    // how many extra ticks to delay before playing next line

    pattern_loop_position: Option<u32>, // set if we have a good position to loop to
    pattern_loop: i32,
    set_pattern_position: bool, // set to jump
}

impl PlayerState {
    pub fn new(num_channels: u32, device_sample_rate: u32) -> PlayerState {
        let mut channels = Vec::new();
        for _channel in 0..num_channels {
            channels.push(ChannelInfo::new())
        }
        PlayerState {
            channels,
            song_pattern_position: 0,
            current_line: 0,
            current_vblank: 0,
            current_vblank_sample: 0,
            device_sample_rate: device_sample_rate,
            song_speed: 6,
            samples_per_vblank: device_sample_rate / 50,
            clock_ticks_per_device_sample: CLOCK_TICKS_PERS_SECOND / device_sample_rate as f32,
            next_pattern_pos: -1,
            next_position: -1,
            delay_line: 0,
            song_has_ended: false,
            has_looped: false,

            pattern_loop_position: None,
            pattern_loop: 0,
            set_pattern_position: false,
        }
    }

    pub fn get_song_line<'a>(&self, song: &'a Song) -> &'a Vec<Note> {
        let pattern_idx = song.pattern_table[self.song_pattern_position as usize];
        let pattern = &song.patterns[pattern_idx as usize];
        let line = &pattern.lines[self.current_line as usize];
        line
    }
}

fn play_note(note: &Note, player_state: &mut PlayerState, channel_num: usize, song: &Song) {
    let channel = &mut player_state.channels[channel_num];

    let old_period = channel.period;
    let old_vibrato_pos = channel.vibrato_pos;
    let old_vibrato_speed = channel.vibrato_speed;
    let old_vibrato_depth = channel.vibrato_depth;
    let old_tremolo_speed = channel.tremolo_speed;
    let old_tremolo_depth = channel.tremolo_depth;
    let old_sample_pos = channel.sample_pos;
    let old_sample_num = channel.sample_num;

    if note.sample_number > 0 {
        // sample number 0, means that the sample keeps playing. The sample indices starts at one, so subtract 1 to get to 0 based index
        let current_sample: &Sample = &song.samples[(note.sample_number - 1) as usize];
        channel.volume = current_sample.volume as f32; // Get volume from sample
                                                       //        channel.size =  current_sample.repeat_size + current_sample.repeat_offset;
        channel.size = current_sample.size;
        channel.sample_num = note.sample_number;
        channel.fine_tune = current_sample.fine_tune as u32;
    }

    channel.volume_change = 0.0;
    channel.note_change = 0;
    channel.retrigger_delay = 0;
    channel.vibrato_speed = 0;
    channel.vibrato_depth = 0;
    channel.tremolo_speed = 0;
    channel.tremolo_depth = 0;

    channel.arpeggio_counter = 0;
    channel.arpeggio_offsets[0] = 0;
    channel.arpeggio_offsets[1] = 0;
    if note.period != 0 {
        channel.period = fine_tune_period(note.period, channel.fine_tune, song.has_standard_notes);
        channel.base_period = note.period;
        channel.sample_pos = 0.0;
        // If a note period was played we need to reset the size to start playing from the start
        // ( and redo any sample loops.  sample.size changes as the sample repeats )
        if channel.sample_num > 0 {
            let current_sample: &Sample = &song.samples[(channel.sample_num - 1) as usize];
            channel.size = current_sample.size;
        }
    }

    match note.effect {
        Effect::SetSpeed { speed } => {
            // depending on argument the speed is either sets as VBI counts or Beats Per Minute
            if speed <= 31 {
                // VBI countsa
                player_state.song_speed = speed as u32;
            } else {
                // BPM changes the timing between ticks ( easiest way to do that is to )
                // default is 125 bpm => 500 => ticks per minute ( by default each tick is 6 vblanks ) = > 3000 vblanks per minute or 50 vblanks per sec
                // new BPM * 4 => ticks per minute * 6 / 60 => vblanks per sec = BPM * 0.4
                let vblanks_per_sec = speed as f32 * 0.4;
                player_state.samples_per_vblank =
                    (player_state.device_sample_rate as f32 / vblanks_per_sec) as u32
            }
        }
        Effect::Arpeggio {
            chord_offset_1,
            chord_offset_2,
        } => {
            channel.arpeggio_offsets[0] = chord_offset_1 as u32;
            channel.arpeggio_offsets[1] = chord_offset_2 as u32;
            channel.arpeggio_counter = 0;
        }
        Effect::SlideUp { speed } => {
            channel.note_change = -(speed as i32);
        }
        Effect::SlideDown { speed } => {
            channel.note_change = speed as i32;
        }
        Effect::TonePortamento { speed } => {
            // if a new sound was played ( period was so on the note ) that is the new target. otherwise carry on with old target
            if note.period != 0 {
                channel.period_target = channel.period; // use channel.period which has already been fine-tuned
            } else {
                if channel.last_porta_target != 0 {
                    // use the last porta target if it has been set ( some mod tunes set tone porta without history or note)
                    channel.period_target = channel.last_porta_target;
                } else {
                    // if no note available, use current period ( making this a no-op)
                    channel.period_target = old_period;
                }
            }
            channel.period = old_period; // reset back to old after we used it
            if speed != 0 {
                // only change speed if it non-zero. ( zero means to carry on with the effects as before)
                channel.note_change = speed as i32;
            } else {
                channel.note_change = channel.last_porta_speed;
            }
            // store porta values. Much later portamento effects could still depend on them
            channel.last_porta_speed = channel.note_change;
            channel.last_porta_target = channel.period_target;
            // If the portamento effect happens on the same sample, keep position
            if old_sample_num == channel.sample_num {
                channel.sample_pos = old_sample_pos;
            }
        }
        Effect::Vibrato { speed, amplitude } => {
            if speed == 0 {
                channel.vibrato_speed = old_vibrato_speed;
            }
            if amplitude == 0 {
                channel.vibrato_depth = old_vibrato_depth;
            }
        }
        Effect::TonePortamentoVolumeSlide { volume_change } => {
            // Continue
            channel.volume_change = volume_change as f32;
            if note.period != 0 {
                channel.period_target = channel.period;
            } else {
                channel.period_target = channel.last_porta_target;
            }
            channel.period = old_period;
            channel.sample_pos = old_sample_pos;
            channel.last_porta_target = channel.period_target;
            channel.note_change = channel.last_porta_speed;
        }
        Effect::VibratoVolumeSlide { volume_change } => {
            channel.volume_change = volume_change as f32;
            channel.vibrato_pos = old_vibrato_pos as u32;
            channel.vibrato_speed = old_vibrato_speed as u32;
            channel.vibrato_depth = old_vibrato_depth as i32;
        }
        Effect::Tremolo { speed, amplitude } => {
            if speed == 0 && amplitude == 0 {
                channel.tremolo_depth = old_tremolo_depth;
                channel.tremolo_speed = old_tremolo_speed;
            } else {
                channel.tremolo_depth = amplitude as i32;
                channel.tremolo_speed = speed as u32;
            }
        }
        Effect::SetSampleOffset { offset } => {
            // Ignore, unless we are also playing a new sound
            if note.period != 0 && channel.sample_num > 0 {
                channel.sample_pos = (offset as f32) * 256.0;
                // Does the offset go past the end of the sound
                let current_sample: &Sample = &song.samples[(channel.sample_num - 1) as usize];
                if channel.sample_pos as u32 > current_sample.size {
                    channel.sample_pos = (channel.sample_pos as u32 % current_sample.size) as f32
                }
            }
        }
        Effect::VolumeSlide { volume_change } => {
            channel.volume_change = volume_change as f32;
        }
        Effect::SetVolume { volume } => {
            channel.volume = volume as f32;
        }
        Effect::PatternBreak { next_pattern_pos } => {
            player_state.next_pattern_pos = next_pattern_pos as i32;
            if player_state.next_pattern_pos > 63 {
                // only possible to jump to index 63 at most. Anything highrt interpreted as jumping to beginning of next pattern
                player_state.next_pattern_pos = 0;
            }
        }
        Effect::PositionJump { next_pattern } => {
            if next_pattern as u32 <= player_state.song_pattern_position {
                player_state.has_looped = true;
            }
            player_state.next_position = next_pattern as i32;
        }
        Effect::FinePortaUp { period_change } => {
            channel.period = change_note(channel.period, -(period_change as i32));
        }
        Effect::FinePortaDown { period_change } => {
            channel.period = change_note(channel.period, period_change as i32);
        }
        Effect::PatternLoop { arg } => {
            if arg == 0 {
                // arg 0 marks the loop start position
                player_state.pattern_loop_position = Some(player_state.current_line);
            } else {
                if player_state.pattern_loop == 0 {
                    player_state.pattern_loop = arg as i32;
                } else {
                    player_state.pattern_loop -= 1;
                }
                if player_state.pattern_loop > 0 && player_state.pattern_loop_position.is_some() {
                    player_state.set_pattern_position = true;
                } else {
                    // Double loops ( loops start followed by two or more loops can confuse the player. Once a loop is passed. invalidate the loop marker)
                    player_state.pattern_loop_position = None;
                }
            }
        }
        Effect::TremoloWaveform { wave: _ } => {
            // println!("set tremolo wave");
        }
        Effect::CoarsePan { pan_pos: _ } => {
            // Skip pan for now
        }

        Effect::RetriggerSample { retrigger_delay } => {
            channel.retrigger_delay = retrigger_delay as u32;
            channel.retrigger_counter = 0;
        }
        Effect::FineVolumeSlideUp { volume_change } => {
            channel.volume = channel.volume + volume_change as f32;
            if channel.volume > 64.0 {
                channel.volume = 64.0;
            }
        }
        Effect::FineVolumeSlideDown { volume_change } => {
            channel.volume = channel.volume - volume_change as f32;
            if channel.volume < 0.0 {
                channel.volume = 0.0;
            }
        }
        Effect::CutNote { delay } => {
            channel.cut_note_delay = delay as u32;
        }
        Effect::SetHardwareFilter { new_state: _ } => {
            // not much to do. only works on the a500
        }
        Effect::DelayedLine { delay_ticks } => {
            player_state.delay_line = delay_ticks as u32;
        }
        Effect::InvertLoop { loop_position: _ } => {
            //Ignore for now
        }
        Effect::None => {}
        _ => {
            //            println!("Unhandled effect");
        }
    }
}

fn play_line(song: &Song, player_state: &mut PlayerState) {
    // is a pattern break active
    if player_state.next_pattern_pos != -1 {
        player_state.song_pattern_position += 1;
        player_state.current_line = player_state.next_pattern_pos as u32;
        player_state.next_pattern_pos = -1;
    } else if player_state.next_position != -1 {
        player_state.song_pattern_position = player_state.next_position as u32;
        player_state.current_line = 0;
        player_state.next_position = -1;
    }

    // We could have been place past the end of the song
    if player_state.song_pattern_position >= song.num_used_patterns {
        if song.end_position < song.num_used_patterns {
            player_state.song_pattern_position = song.end_position;
            player_state.has_looped = true;
        } else {
            player_state.song_has_ended = true;
        }
    }

    let line = player_state.get_song_line(song);
    for channel_number in 0..line.len() {
        play_note(
            &line[channel_number as usize],
            player_state,
            channel_number,
            song,
        );
    }

    if player_state.set_pattern_position && player_state.pattern_loop_position.is_some() {
        // jump to pattern loop position of the pattern loop was triggered
        player_state.set_pattern_position = false;
        player_state.current_line = player_state.pattern_loop_position.unwrap();
    } else {
        // othwerwise advance to next pattern
        player_state.current_line += 1;
        if player_state.current_line >= 64 {
            player_state.song_pattern_position += 1;
            if player_state.song_pattern_position >= song.num_used_patterns {
                player_state.song_has_ended = true;
            }
            player_state.current_line = 0;
        }
    }
}

fn update_effects(player_state: &mut PlayerState, song: &Song) {
    for channel in &mut player_state.channels {
        if channel.sample_num != 0 {
            if channel.cut_note_delay > 0 {
                channel.cut_note_delay -= 1;
                if channel.cut_note_delay == 0 {
                    channel.cut_note_delay = 0;
                    // set size of playing sample to zero to indicate nothing is playing
                    channel.size = 0;
                }
            }

            if channel.retrigger_delay > 0 {
                channel.retrigger_counter += 1;
                if channel.retrigger_delay == channel.retrigger_counter {
                    channel.sample_pos = 0.0;
                    channel.retrigger_counter = 0;
                }
            }
            channel.volume += channel.volume_change;
            if channel.tremolo_depth > 0 {
                let base_volume = song.samples[(channel.sample_num - 1) as usize].volume as i32;
                let tremolo_size: i32 = (static_tables::VIBRATO_TABLE
                    [(channel.tremolo_pos & 63) as usize]
                    * channel.tremolo_depth)
                    / 64;
                let volume = base_volume + tremolo_size;
                channel.tremolo_pos += channel.tremolo_speed;
                channel.volume = volume as f32;
            }
            if channel.volume < 0.0 {
                channel.volume = 0.0
            }
            if channel.volume > 64.0 {
                channel.volume = 64.0
            }

            if channel.arpeggio_offsets[0] != 0 || channel.arpeggio_offsets[1] != 0 {
                let new_period: u32;
                let index = static_tables::FREQUENCY_TABLE
                    .binary_search(&channel.base_period)
                    .expect(&format!(
                        "Unexpected period value at arpeggio {}, {}:{}",
                        channel.base_period,
                        player_state.song_pattern_position,
                        player_state.current_line
                    )) as i32;
                if channel.arpeggio_counter > 0 {
                    let mut note_offset = index
                        - channel.arpeggio_offsets[(channel.arpeggio_counter - 1) as usize] as i32;
                    if note_offset < 0 {
                        note_offset = 0;
                    }
                    new_period = static_tables::FREQUENCY_TABLE[note_offset as usize];
                } else {
                    new_period = channel.base_period;
                }
                channel.period =
                    fine_tune_period(new_period, channel.fine_tune, song.has_standard_notes);

                channel.arpeggio_counter += 1;
                if channel.arpeggio_counter >= 3 {
                    channel.arpeggio_counter = 0;
                }
            }
            if channel.vibrato_depth > 0 {
                let period = fine_tune_period(
                    channel.base_period,
                    channel.fine_tune,
                    song.has_standard_notes,
                );
                channel.period = ((period as i32)
                    + (static_tables::VIBRATO_TABLE[(channel.vibrato_pos & 63) as usize]
                        * channel.vibrato_depth)
                        / 32) as u32;
                channel.vibrato_pos += channel.vibrato_speed;
            } else if channel.note_change != 0 {
                // changing note to a target
                if channel.period_target != 0 {
                    if channel.period_target > channel.period {
                        channel.period = change_note(channel.period, channel.note_change);
                        if channel.period >= channel.period_target {
                            channel.period = channel.period_target;
                        }
                    } else {
                        channel.period = change_note(channel.period, -channel.note_change);
                        if channel.period <= channel.period_target {
                            channel.period = channel.period_target;
                        }
                    }
                } else {
                    // or just moving it
                    channel.period = change_note(channel.period, channel.note_change);
                }
            }
        }
    }
}

/// Calculates the next sample pair (left, right) to be played from the song. The returned samples have the range [-1, 1]
pub fn next_sample(song: &Song, player_state: &mut PlayerState) -> (f32, f32) {
    let mut left = 0.0;
    let mut right = 0.0;

    // Have we reached a new vblank
    if player_state.current_vblank_sample >= player_state.samples_per_vblank {
        player_state.current_vblank_sample = 0;

        update_effects(player_state, song);

        // Is it time to play a new note line either by VBI counting or BPM counting
        if player_state.current_vblank >= player_state.song_speed {
            if player_state.delay_line > 0 {
                player_state.delay_line -= 1;
            } else {
                player_state.current_vblank = 0;
                play_line(song, player_state);
            }
        }
        // apply on every vblank but only after the line has been processed
        player_state.current_vblank += 1;
    }
    player_state.current_vblank_sample += 1;

    for channel_number in 0..player_state.channels.len() {
        let channel_info: &mut ChannelInfo = &mut player_state.channels[channel_number];
        if channel_info.size > 2 {
            let current_sample: &Sample = &song.samples[(channel_info.sample_num - 1) as usize];

            //  check if we have reached the end of the sample ( do this before getting the sample as some note data can change the
            // postions past available data.  )
            if channel_info.sample_pos >= channel_info.size as f32 {
                let overflow: f32 = channel_info.sample_pos - channel_info.size as f32;
                channel_info.sample_pos = current_sample.repeat_offset as f32 + overflow;
                channel_info.size = current_sample.repeat_size + current_sample.repeat_offset;
                if channel_info.size <= 2 {
                    continue;
                }
            }

            // Grab the sample, no filtering
            let mut channel_value: f32 =
                current_sample.samples[(channel_info.sample_pos as u32) as usize] as f32; // [ -127, 127 ]

            //     let left_pos = channel_info.sample_pos as u32;
            //     let left_weight: f32 = 1.0 - (channel_info.sample_pos - left_pos as f32);
            //     let mut channel_value: f32 = current_sample.samples[ left_pos as usize] as f32;   // [ -127, 127 ]
            //     if left_pos < (current_sample.size - 1) as u32 {
            //        let right_value = current_sample.samples[(left_pos + 1) as usize] as f32;
            //        channel_value = left_weight * channel_value + (1.0 - left_weight) * right_value;
            //    }

            // max channel vol (64), sample range [ -128,127] scaled to [-1,1]
            channel_value *= channel_info.volume / (128.0 * 64.0);

            // update position
            channel_info.sample_pos +=
                player_state.clock_ticks_per_device_sample / channel_info.period as f32;

            let channel_selector = (channel_number as u8) & 0x0003;
            if channel_selector == 0 || channel_selector == 3 {
                left += channel_value;
            } else {
                right += channel_value;
            }
        }
    }
    (left, right)
}
