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
//!     let song = mod_player::read_mod_file("BUBBLE_BOBBLE.MOD");
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

use std::fs;

pub mod textout;

const CLOCK_TICKS_PERS_SECOND: f32 = 3579545.0;      // Amiga hw clcok ticks per second

static VIBRATO_TABLE: [ i32; 64] = [0,24,49,74,97,120,141,161, 180,197,212,224,235,244,250,253,255,253,250,244,235,224,212,197,180,161,141,120,97,74,49,24,
    -0,-24,-49,-74,-97,-120,-141,-161, -180,-197,-212,-224,-235,-244,-250,-253,-255,-253,-250,-244,-235,-224,-212,-197,-180,-161,-141,-120,-97,-74,-49,-24];
static FREQUENCY_TABLE: [u32; 60] = [
//    B    A#   A    G#    G   F#   F    E    D#   D   C#   C    
    57,    60,  64,  67,  71,  76,  80,  85,  90,  95, 101, 107,     
    113,   120, 127, 135, 143, 151, 160, 170, 180, 190, 202, 214,
    226,   240, 254, 269, 285, 302, 320, 339, 360, 381, 404, 428, 
    453,   480, 508, 538, 570, 604, 640, 678, 720, 762, 808, 856, 
    907,   961, 1017, 1077, 1141, 1209, 1281, 1357, 1440, 1525, 1616, 1712
];

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

impl Sample{
    fn new( sample_info : &[u8] ) -> Sample {
        let sample_name = String::from_utf8_lossy(&sample_info[0..22]);
        let sample_size: u32 = ((sample_info[23] as u32) + (sample_info[22] as u32) * 256) * 2;
        let fine_tune = sample_info[24];
        let volume = sample_info[25];

        // the repeat offset appears to be in bytes ...
        let mut repeat_offset: u32 = ((sample_info[27] as u32) + (sample_info[26] as u32) * 256 )*2;
        // .. but the size is in word? 
        let repeat_size: u32 = ((sample_info[29] as u32) + (sample_info[28] as u32) * 256 )*2;

        if sample_size > 0 {
            if repeat_offset + repeat_size > sample_size {
                repeat_offset -= ( repeat_offset + repeat_size ) -  sample_size;
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

enum Effect{  
/*  Tremolo = 7,
    SetPanningPosition = 8,
    ExtendedEffects = 14,   //e
*/    
    None, // 0
    Arpeggio{ chord_offset_1 : u8, chord_offset_2 : u8 },
    SlideUp{ speed : u8  },             // 1
    SlideDown{ speed: u8  },            // 2
    TonePortamento{ speed: u8 },        // 3 
    Vibrato{ speed : u8, amplitude : u8 },      // 4
    TonePortamentoVolumeSlide{ volume_change : i8 }, //5
    VibratoVolumeSlide{ volume_change : i8 },   // 6
    SetSampleOffset{ offset : u8 },          // 9
    VolumeSlide{ volume_change : i8 },          // 10
    PositionJump{ next_pattern : u8 },  // 11,
    SetVolume{ volume : u8 },           // 12
    PatternBreak{ next_pattern_pos : u8  },     //13
    SetSpeed{ speed : u8 },             // 15

    SetHardwareFilter{ new_state: u8 },         //E0
    FinePortaUp{ period_change : u8 },          //E1 
    FinePortaDown{ period_change : u8 },        //E2 
    PatternLoop{ arg: u8 },                     //E6 
    RetriggerSample{ retrigger_delay : u8 },    //E9 
    FineVolumeSlideUp{ volume_change : u8 },        //EA 
    FineVolumeSlideDown{ volume_change : u8 },        //EB 
    DelayedSample{ delay_ticks : u8 },        //ED 
    SetVibratoWave{ wave : u8 }
}

impl Effect{
    fn new( effect_number : u8, effect_argument : i8 ) -> Effect {
        match effect_number  {
            0 => match effect_argument {
                0 => Effect::None,
                _ => Effect::Arpeggio{ chord_offset_1 : effect_argument as u8 >> 4, chord_offset_2 : effect_argument as u8 & 0x0f },
//                _ => panic!( format!( "unhandled arpeggio effect: {}", effect_number ) )
            },
            1 => Effect::SlideUp{ speed : effect_argument as u8 },          // decrease period, increase frequency, higher note
            2 => Effect::SlideDown{ speed : effect_argument as u8 },
            3 => Effect::TonePortamento{ speed : effect_argument as u8 },
            4 => Effect::Vibrato{ speed : effect_argument as u8 >> 4, amplitude : effect_argument as u8 & 0x0f  },
            5 => {
                if (effect_argument as u8 & 0xf0) != 0 {
                    Effect::TonePortamentoVolumeSlide{ volume_change : effect_argument >> 4 }
                } else {
                    Effect::TonePortamentoVolumeSlide{ volume_change :  -effect_argument }
                }
            },
            6 => {
                if (effect_argument as u8 & 0xf0) != 0 {
                    Effect::VibratoVolumeSlide{ volume_change : effect_argument >> 4 }
                } else {
                    Effect::VibratoVolumeSlide{ volume_change :  -effect_argument }
                }
            },
            9 => Effect::SetSampleOffset{ offset : effect_argument as u8 },
            10 => {
                if (effect_argument as u8 & 0xf0) != 0 {
                    Effect::VolumeSlide{ volume_change : effect_argument >> 4 }
                } else {
                    Effect::VolumeSlide{ volume_change :  -effect_argument }
                }
            }
            11 => Effect::PositionJump{ next_pattern : effect_argument as u8 },
            12 => Effect::SetVolume{ volume : effect_argument as u8 },
            13 => Effect::PatternBreak{ next_pattern_pos : ((0xf0&( effect_argument as u32 ))*10 + ( effect_argument as u32 & 0x0f)) as u8 },
            14 => {
                let extended_effect = ( effect_argument as u8 ) >> 4;
                let extended_argument = ( effect_argument as u8 ) &0x0f;
                match extended_effect {
                    0 => Effect::SetHardwareFilter{ new_state : extended_argument as u8 },
                    1 => Effect::FinePortaUp{ period_change : extended_argument as u8 },
                    6 => {
                        match extended_argument {
                            0 => Effect::PatternLoop{ arg: 0},
                            _ => panic!( "unhandled PATTERN LOOp Trigger" )
                        }    
                    }
                    9 => Effect::RetriggerSample{ retrigger_delay : extended_argument as u8 },
                    10 => Effect::FineVolumeSlideUp{ volume_change : extended_argument as u8 },
                    11 => Effect::FineVolumeSlideDown{ volume_change : extended_argument as u8 },
                    13 => Effect::DelayedSample{ delay_ticks : extended_argument as u8 },
                    _ => panic!( format!( "unhandled extended effect number: {}", extended_effect ) )
                }
            },
            // 14 => {
            //     println!("unhandled extended effect number {}",effect_argument );
            //     Effect::None
            // }
            15 => Effect::SetSpeed{ speed : effect_argument as u8 }, 
            _ => panic!( format!( "unhandled effect number: {}", effect_number ) )
        }
    }
}

/// Describes what sound sample to play and an effect (if any) that should be applied. 
pub struct Note{
    sample_number: u8,
    period: u32,            /// how many clock ticks each sample is held for
    effect: Effect,
}

fn change_note( current_period : u32, change : i32 ) -> u32 {
    // find note in frequency table
    let mut result = current_period as i32 + change;
    if result > 856 { result = 856; }
    if result < 113 { result = 113; }
    result as u32
}

impl Note{
    fn new( note_data : &[u8], format_description : &FormatDescription) -> Note {
        let mut sample_number = ( (note_data[2] & 0xf0) >> 4 )  + ( note_data[ 0 ] &0xf0);
        if format_description.num_samples == 15 {
            sample_number = sample_number & 0x0f;
        } else {
            sample_number = sample_number & 0x1f;
        }
        let period = ((note_data[0] & 0x0f) as u32) * 256 + (note_data[1] as u32);
        let effect_argument = note_data[3] as i8;
        let effect_number = note_data[ 2] & 0x0f;
        let effect = Effect::new(effect_number, effect_argument);
        Note{
            sample_number, period, effect
        }
    }
}

pub struct Pattern {
    lines: Vec<Vec<Note>>       // outer vector is the lines (64). Inner vector holds the notes for the line             
}

impl Pattern{
    fn new( ) -> Pattern {
        let mut lines : Vec<Vec<Note>> = Vec::new();
        for _line in 0..64 {
            lines.push( Vec::new() );
        }
        Pattern{ lines }
    }
}

/// The features of the song
pub struct FormatDescription{
    pub num_channels : u32,
    pub num_samples : u32,  
    /// Is the format description based on a tag. Most mod file have a tag descriptor that makes it possible to identify the file with some
    /// certainty. The very earliest files do not have a tag but are assumed to support 4 channels and 15 samples.
    pub has_tag : bool          
}

/// Contains the entire mod song
pub struct Song {
    /// The name of the song as specified in the mod file    
    pub name: String,               
    /// Features of the song
    pub format : FormatDescription,      
    /// The audio samples used by the song
    pub samples: Vec<Sample>,           
    /// Patterns contain all the note data
    pub patterns: Vec<Pattern>,
    /// Specifies the order in whcih the patterns should be played in. The same pattern may played several times in the same song
    pub pattern_table: Vec<u8>,
    /// How many patterns in the pattern table should be used. The pattern table is a fixed length can usually longer than the song
    pub num_used_patterns : u32,
    /// Which pattern should be played after the last pattern in the pattern_table. Used for infinitely looping repeating songs 
    pub end_position : u32,
}

struct ChannelInfo {
    sample_num: u8,         // which sample is playing 
    sample_pos: f32,         
    period : u32,           //
    size : u32,
    volume: f32,            // max 1.0
    volume_change: f32,     // max 1.0
    note_change : i32,
    period_target : u32,      // note portamento target

    base_period : u32,
    vibrato_pos : u32,
    vibrato_speed : u32,
    vibrato_depth : i32,
    
    retrigger_delay : u32,
    retrigger_counter : u32,

    arpeggio_counter : u32,
    arpeggio_offsets : [u32;2],
}

impl ChannelInfo{
    fn new() -> ChannelInfo {
        ChannelInfo {
            sample_num: 0,
            sample_pos: 0.0,
            period : 0,
            size : 0,
            volume: 0.0,
            volume_change: 0.0,
            note_change : 0,
            period_target : 0,

            base_period : 0,
            vibrato_pos : 0,
            vibrato_speed : 0,
            vibrato_depth : 0,

            retrigger_delay : 0,
            retrigger_counter : 0,
            arpeggio_counter : 0,
            arpeggio_offsets : [ 0, 0] ,
        }
    }
}
/// Keeps track of all the dynamic state required for playing the song. 
pub struct PlayerState{
    channels: Vec<ChannelInfo>,
    song_pattern_position: u32,             // where in the pattern table are we currently
    /// current line position in the pattern. Every pattern has 64 lines
    pub current_line: u32,
    /// set when the song stops playing
    pub song_has_ended : bool,
    /// set when the song loops. The player does not unset this flag after it has been set. To detect subsequent loops the flag to be manually unset by the client
    pub has_looped : bool,
    device_sample_rate : u32,
    song_speed: u32,                        // in vblanks
    current_vblank : u32,                   // how many vblanks since last play line
    samples_per_vblank: u32,                // how many device samples per 'vblank'
    clock_ticks_per_device_sample : f32,    // how many amiga hardware clock ticks per device sample
    current_vblank_sample : u32,            // how many device samples have we played for the current 'vblank'

    next_pattern_pos : i32,                 // on  next line if == -1 do nothing else  go to next pattern on line next_pattern_pos
    next_position : i32,                    // on next line if == 1 do nothing else go to beginning of the this pattern
}

impl PlayerState{
    pub fn new( num_channels : u32, device_sample_rate : u32 ) -> PlayerState {
        let mut channels = Vec::new();
        for _channel in 0..num_channels {
            channels.push(ChannelInfo::new())
        }
        PlayerState{
            channels,
            song_pattern_position : 0,
            current_line: 0,
            current_vblank : 0,             
            current_vblank_sample : 0,      
            device_sample_rate : device_sample_rate,
            song_speed: 6,                  
            samples_per_vblank: device_sample_rate / 50,
            clock_ticks_per_device_sample : CLOCK_TICKS_PERS_SECOND / device_sample_rate as f32,
            next_pattern_pos : -1,
            next_position : -1,
            song_has_ended : false, 
            has_looped :false

        }
    }

    pub fn get_song_line<'a>( &self, song : &'a Song ) -> &'a Vec<Note> {
        let pattern_idx = song.pattern_table[self.song_pattern_position as usize];
        let pattern = &song.patterns[ pattern_idx as usize];
        let line = &pattern.lines[ self.current_line as usize ];
        line
    }

}

fn play_note(note: &Note, player_state: &mut PlayerState, channel_num: usize, song: &Song) {
    let old_period = player_state.channels[ channel_num ].period;
    let old_period_target = player_state.channels[ channel_num ].period_target;
    let old_vibrato_pos = player_state.channels[channel_num].vibrato_pos;
    let old_vibrato_speed = player_state.channels[channel_num].vibrato_speed;
    let old_vibrato_depth = player_state.channels[channel_num].vibrato_depth;
    let old_note_change = player_state.channels[channel_num].note_change;

    if note.sample_number > 0 {
        // sample number 0, means that the sample keeps playing. The sample indices starts at one, so subtract 1 to get to 0 based index
        let current_sample: &Sample = &song.samples[(note.sample_number - 1) as usize];
        player_state.channels[channel_num].volume = current_sample.volume as f32;    // Get volume from sample
//        player_state.channels[channel_num].size =  current_sample.repeat_size + current_sample.repeat_offset;
        player_state.channels[channel_num].size = song.samples[(note.sample_number-1) as usize].size;
        player_state.channels[channel_num].sample_num = note.sample_number;
    }

    player_state.channels[channel_num].volume_change = 0.0;
    player_state.channels[channel_num].note_change = 0;
    player_state.channels[channel_num].retrigger_delay = 0;

    player_state.channels[channel_num].vibrato_pos = 0;
    player_state.channels[channel_num].vibrato_speed = 0;
    player_state.channels[channel_num].vibrato_depth = 0;

    player_state.channels[channel_num].arpeggio_offsets[ 0 ] = 0;
    player_state.channels[channel_num].arpeggio_offsets[ 1 ] = 0;
    if note.period != 0 {
        player_state.channels[channel_num].period = note.period as u32;
        player_state.channels[channel_num].sample_pos = 0.0;
    }

    match note.effect {
        Effect::SetSpeed{ speed } => {
            // depending on argument the speed is either sets as VBI counts or Beats Per Minute
            if speed <= 31 {
                // VBI countsa
                player_state.song_speed = speed as u32;
            } else {
                // BPM changes the timing between ticks ( easiest way to do that is to )
                // default is 125 bpm => 500 => ticks per minute ( by default each tick is 6 vblanks ) = > 3000 vblanks per minute or 50 vblanks per sec
                // new BPM * 4 => ticks per minute * 6 / 60 => vblanks per sec = BPM * 0.4
                let vblanks_per_sec = speed as f32 * 0.4;
                player_state.samples_per_vblank = ( player_state.device_sample_rate as f32 / vblanks_per_sec ) as u32
            }
        }
        Effect::Arpeggio{ chord_offset_1, chord_offset_2 } => {
            player_state.channels[channel_num].base_period = player_state.channels[channel_num].period;
            player_state.channels[channel_num].arpeggio_offsets[ 0 ] = chord_offset_1 as u32;
            player_state.channels[channel_num].arpeggio_offsets[ 1 ] = chord_offset_2 as u32;
            player_state.channels[channel_num].arpeggio_counter = 0;
        }
        Effect::SlideUp{ speed } => {
            player_state.channels[channel_num].note_change = -(speed as i32);
        }
        Effect::SlideDown{ speed } => {
            player_state.channels[channel_num].note_change = speed as i32;
        }
        Effect::TonePortamento{ speed } => {
            // if a new sound was played ( period was so on the note ) that is the new target. otherwise carry on with old target
            player_state.channels[channel_num].period = old_period;
            if note.period != 0 {
                player_state.channels[channel_num].period_target = note.period;
            }  else {
                player_state.channels[channel_num].period_target = old_period_target;
            }
            // only change speed if it non-zero. ( zero means to carry on with the effects as before)
            if speed != 0 {
                player_state.channels[channel_num].note_change = speed as i32;
            } else {
                player_state.channels[channel_num].note_change = old_note_change;
            }
        }
        Effect::Vibrato{ speed, amplitude } => {
            player_state.channels[channel_num].base_period = player_state.channels[channel_num].period;
            player_state.channels[channel_num].vibrato_speed = speed as u32;
            player_state.channels[channel_num].vibrato_depth = amplitude as i32;
        }
        Effect::TonePortamentoVolumeSlide{ volume_change } => {
            // Continue 
            player_state.channels[channel_num].volume_change = volume_change as f32;
            player_state.channels[channel_num].period_target = old_period_target;
            player_state.channels[channel_num].period = old_period;
            player_state.channels[channel_num].note_change = old_note_change;
        }
        Effect::VibratoVolumeSlide{ volume_change } => {
            player_state.channels[channel_num].volume_change = volume_change as f32;
            player_state.channels[channel_num].vibrato_pos = old_vibrato_pos as u32;
            player_state.channels[channel_num].vibrato_speed = old_vibrato_speed as u32;
            player_state.channels[channel_num].vibrato_depth = old_vibrato_depth as i32;
        }
        Effect::SetSampleOffset{ offset } => {
            player_state.channels[channel_num].sample_pos = ( offset as f32 )* 256.0;
        } 
        Effect::VolumeSlide{ volume_change } => {
            player_state.channels[channel_num].volume_change = volume_change as f32;
        }
        Effect::SetVolume{ volume } => {
            player_state.channels[channel_num].volume = volume as f32;
        }
        Effect::PatternBreak{ next_pattern_pos } => {
            player_state.next_pattern_pos = next_pattern_pos as i32;
        }
        Effect::PositionJump{ next_pattern } => {
            if next_pattern as u32 <= player_state.song_pattern_position  {
                player_state.has_looped = true;
            }
            player_state.next_position = next_pattern as i32;       
        }
        Effect::FinePortaUp{ period_change } => {
            player_state.channels[channel_num].period = change_note(player_state.channels[channel_num].period, -( period_change as i32 ) );

        }
        Effect::RetriggerSample{ retrigger_delay } => {
            player_state.channels[ channel_num ].retrigger_delay = retrigger_delay as u32; 
            player_state.channels[ channel_num ].retrigger_counter = 0; 
        }
        Effect::FineVolumeSlideUp{ volume_change } => {
            player_state.channels[channel_num].volume = player_state.channels[channel_num].volume + volume_change as f32;
            if player_state.channels[channel_num].volume > 64.0 {
                player_state.channels[channel_num].volume = 64.0;
            }
        }
        Effect::FineVolumeSlideDown{ volume_change } => {
            player_state.channels[channel_num].volume = player_state.channels[channel_num].volume - volume_change as f32;
            if player_state.channels[channel_num].volume < 0.0 {
                player_state.channels[channel_num].volume = 0.0;
            }
        }
        Effect::SetHardwareFilter{ new_state } => {
            // not much to do. only works on the a500            
        }
        Effect::None => {}
        _ => {
            println!("Unhandled effect" );
        }
    }
}


fn play_line(song: &Song, player_state: &mut PlayerState ) {
    // is a pattern break active

    if player_state.next_pattern_pos != -1 {
        player_state.song_pattern_position += 1;
        player_state.current_line = player_state.next_pattern_pos as u32;
        player_state.next_pattern_pos = -1;
    } else if player_state.next_position != -1  {
        player_state.song_pattern_position = player_state.next_position as u32;
        player_state.current_line = 0;
        player_state.next_position = -1;
    }

//    player_state.song_pattern_position = 2;

    let line = player_state.get_song_line( song );
    for channel_number in 0..line.len(){
        play_note(&line[ channel_number as usize ], player_state, channel_number, song);
    }

    player_state.current_line += 1;
    if player_state.current_line >= 64 {
        if player_state.song_pattern_position == song.num_used_patterns {
            player_state.song_has_ended = true;
        }
        player_state.song_pattern_position += 1;
        player_state.current_line = 0;
    }
}

fn update_effects(song: &Song, player_state: &mut PlayerState ){
    for channel in &mut player_state.channels {
        if channel.sample_num != 0 {
            if channel.retrigger_delay > 0 {
                channel.retrigger_counter +=1;
                if channel.retrigger_delay == channel.retrigger_counter {
                    channel.sample_pos = 0.0;
                    channel.retrigger_counter = 0;
                }
            }            
            channel.volume += channel.volume_change;
            if channel.volume < 0.0 { channel.volume = 0.0 }
            if channel.volume > 64.0 { channel.volume = 64.0 }

            if channel.arpeggio_offsets[ 0] != 0 || channel.arpeggio_offsets[ 1 ] != 0 {
                let index : u32 = FREQUENCY_TABLE.binary_search( &channel.base_period ).expect( "Unexpected period value") as u32;
                if channel.arpeggio_counter > 0 {
                    let note_offset  = ( index + channel.arpeggio_offsets[ channel.arpeggio_counter as usize]) as usize;
                    channel.period = FREQUENCY_TABLE[ note_offset-1 ];
                } else {
                    channel.period = channel.base_period;
                }

                channel.arpeggio_counter += 1;
                if channel.arpeggio_counter >= 2 {
                    channel.arpeggio_counter = 0;
                } 
            }
            if channel.vibrato_depth > 0 {
                channel.period = ( ( channel.base_period as i32 ) + ( VIBRATO_TABLE[(channel.vibrato_pos&63) as usize] * channel.vibrato_depth ) / 32 ) as u32;
                channel.vibrato_pos += channel.vibrato_speed;
            }
            else if channel.note_change != 0 {
                // changing note to a target
                if channel.period_target != 0 {
                    if channel.period_target > channel.period {
                        channel.period = change_note(channel.period, channel.note_change);
                        if channel.period >= channel.period_target {
                            channel.period = channel.period_target;
                            channel.note_change = 0;
                        }
                    } else {
                        channel.period = change_note(channel.period, -channel.note_change);
                        if channel.period <= channel.period_target {
                            channel.period = channel.period_target;
                            channel.note_change = 0;
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

        update_effects(song,player_state);

        // Is it time to play a new note line either by VBI counting or BPM counting
        if player_state.current_vblank >= player_state.song_speed {
            player_state.current_vblank = 0;
            play_line( song, player_state );
        }
        // apply on every vblank but only after the line has been processed
        player_state.current_vblank += 1;
    }
    player_state.current_vblank_sample += 1;


    for channel_number in 0..player_state.channels.len() {
//        if channel_number != 1 { continue };
        let channel_info: &mut ChannelInfo = &mut player_state.channels[channel_number];
        if channel_info.size > 2 {
            let current_sample: &Sample = &song.samples[(channel_info.sample_num - 1) as usize];

            // Grab the sample, no filtering
            if channel_info.sample_pos as u32 == 4356 && current_sample.samples.len() == 3900 {
                println!("FFFFFFF");
            }
            let mut channel_value: f32 = current_sample.samples[(channel_info.sample_pos as u32) as usize] as f32;   // [ -127, 127 ] 

        //     let left_pos = channel_info.sample_pos as u32;
        //     let left_weight: f32 = 1.0 - (channel_info.sample_pos - left_pos as f32);
        //     let mut channel_value: f32 = current_sample.samples[ left_pos as usize] as f32;   // [ -127, 127 ] 
        //     if left_pos < (current_sample.size - 1) as u32 {
        //        let right_value = current_sample.samples[(left_pos + 1) as usize] as f32;
        //        channel_value = left_weight * channel_value + (1.0 - left_weight) * right_value;
        //    }

            // max channel vol (64), sample range [ -128,127] scaled to [-1,1]    
            channel_value *= channel_info.volume / (128.0*64.0);

            // update position and check if we have reached the end of the sample
            channel_info.sample_pos +=  player_state.clock_ticks_per_device_sample / channel_info.period as f32;

            if channel_info.sample_pos >= channel_info.size as f32 {
                let overflow : f32 = channel_info.sample_pos - channel_info.size as f32;
                channel_info.sample_pos = current_sample.repeat_offset as f32 + overflow;
                channel_info.size = current_sample.repeat_size + current_sample.repeat_offset;
            }

            let channel_selector = ( channel_number as u8 ) & 0x0003; 
            if channel_selector == 0 || channel_number as u32 == 0 || channel_number == 3 {
                left += channel_value;
            } else {
                right += channel_value;
            }
        }
    }
    (left, right )
}

/**
 * Identify the mod format version based on the tag. If there is not identifiable that it is assumed to be an original mod.
 */
fn get_format(file_data: &Vec<u8> ) -> FormatDescription {
    let format_tag = String::from_utf8_lossy(&file_data[1080..1084]);
    match format_tag.as_ref() {
        "M.K." | "FLT4" | "M!K!" | "4CHN" => FormatDescription{ num_channels : 4, num_samples : 31, has_tag : true },
        _ => FormatDescription{ num_channels : 4, num_samples : 15, has_tag : false }
    }
}

/// Reads a module music file and returns a song structure ready for playing
/// 
/// # Arguments
/// * `file_name` - the mod file on disk
/// 
pub fn read_mod_file(file_name: &str) -> Song {
    let file_data: Vec<u8> = fs::read(file_name).expect( &format!(  "Cant open file {}", &file_name ) );

    let song_name = String::from_utf8_lossy(&file_data[0..20]);
    let format = get_format(&file_data);

    let mut samples: Vec<Sample> = Vec::new();
    let mut offset : usize = 20;
    for _sample_num in 0..format.num_samples {
        samples.push(Sample::new( &file_data[ offset  .. ( offset + 30 ) as usize  ]));
        offset += 30;
    }

    // Figure out whe / how to stop and repeat pos ( with option to repeat in the player )

    let num_used_patterns: u8 = file_data[offset];
    let end_position: u8 = file_data[offset + 1];
    offset += 2;
    let pattern_table: Vec<u8> = file_data[offset..(offset + 128)].to_vec();
    offset += 128;

    // Skip the tag if one has been identified
    if format.has_tag { offset += 4; }

    // Work out how the total size of the sample data at tbe back od the file 
    let mut total_sample_size = 0;
    for sample in &mut samples {
        total_sample_size = total_sample_size + sample.size;
    }

    // The pattern take up all the space that remains after everything else has been accounted for
    let total_pattern_size = file_data.len() as u32  - (offset as u32) - total_sample_size;
    let single_pattern_size = format.num_channels *  4 * 64;
    let num_patterns = total_pattern_size / single_pattern_size;
    // The pattern space should account for all the remaining space
    if total_pattern_size % single_pattern_size != 0 {
        panic!( "Unrecognized file format. Pattern space does not match expected size")
    }

    // Read the patterns
    let mut patterns: Vec<Pattern> = Vec::new();    
    for _pattern_number in 0..num_patterns {
        let mut pattern = Pattern::new();
        for line in 0..64 {
            for _channel in 0..format.num_channels {
                let note = Note::new( &file_data[ offset..(offset+4)], &format);
                pattern.lines[ line ].push( note );
                offset += 4;
            }
        }
        patterns.push(pattern);
    }

    //Read the sample data
    for sample_number in 0..samples.len() {
        let length = samples[sample_number].size;
        for _idx in 0..length {
            samples[sample_number].samples.push(file_data[offset] as i8);
            offset += 1;
        }
    }

    Song {
        name: String::from(song_name),
        format : format,
        samples: samples,
        patterns: patterns,
        pattern_table: pattern_table,
        num_used_patterns : num_used_patterns as u32,
        end_position: end_position as u32 
    }
}

