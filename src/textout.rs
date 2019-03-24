/// Utility functions for printing out informationa about mods.
use super::{Note,Song,Effect};
use super::Sample;

static NOTE_FREQUENCY_STRINGS : [ (u32, &str ); 60 ]= [
( 57,  "B-6" ), ( 60,  "A#6" ), ( 64,  "A-6" ),( 67,  "G#6" ), ( 71,  "G-6" ), ( 76,  "F#6" ), ( 80,  "F-6" ), ( 85 , "E-6" ), ( 90,  "D#6" ), ( 95 , "D-6" ), ( 101, "C#6" ), ( 107, "C-6"), 
( 113, "B-5" ),( 120, "A#5" ), ( 127, "A-5" ), ( 135, "G#5" ), ( 143, "G-5" ), ( 151, "F#5" ),  ( 160, "F-5" ), ( 170, "E-5" ),( 180, "D#5" ), ( 190, "D-5" ), ( 202, "C#5" ), ( 214, "C-5"), 
( 226, "B-4" ),( 240, "A#4" ),( 254, "A-4" ),  ( 269, "G#4" ), ( 285, "G-4" ),( 302, "F#4" ), ( 320, "F-4" ),( 339, "E-4" ),  ( 360, "D#4" ),( 381, "D-4" ),  ( 404, "C#4" ), ( 428, "C-4"), 
( 453, "B-3" ),( 480, "A#3" ), ( 508, "A-3" ), ( 538, "G#3" ),  ( 570, "G-3" ),( 604, "F#3" ), ( 640, "F-3" ),( 678, "E-3" ),( 720, "D#3" ),( 762, "D-3" ), ( 808, "C#3" ), ( 856, "C-3"),
( 907, "B-2" ),( 961, "A#2" ), ( 1017,"A-2" ), ( 1077,"G#2" ),( 1141,"G-2" ), ( 1209,"F#2" ), ( 1281,"F-2" ),( 1357,"E-3" ),( 1440,"D#2" ), ( 1525,"D-2" ), ( 1616,"C#2" ), ( 1712,"C-2"), 
];

impl Effect{
    fn to_string( &self ) -> String {
        return match self {
            Effect::Arpeggio{ chord_offset_1, chord_offset_2 } => { String::from( "Arpgi" ) },
            Effect::SlideUp{ speed } => { String::from( "SldUp" ) },
            Effect::SlideDown{ speed } => { String::from( "SldDn" ) },
            Effect::TonePortamento{ speed } => { String::from( "TonPo" ) }, 
            Effect::Vibrato{ speed, amplitude } => { String::from( "Vibra" ) },
            Effect::TonePortamentoVolumeSlide{ volume_change } => { String::from( "TPVos") }
            Effect::VibratoVolumeSlide{ volume_change } => { String::from( "ViVoS" ) },
            Effect::SetSampleOffset{ offset } => { String::from( "Offst" ) },
            Effect::VolumeSlide{ volume_change } => { String::from( "VolSl" ) },
            Effect::PositionJump{ next_pattern } => { String::from( "Jump." ) },
            Effect::SetVolume{ volume } => { String::from( "Volme" ) },
            Effect::PatternBreak{ next_pattern_pos } => { String::from( "Break" ) },
            Effect::SetSpeed{ speed } => { String::from( "Speed" ) },
            Effect::SetHardwareFilter{  new_state } => { String::from( "StHwF")}
            Effect::FinePortaUp{ period_change : u8 } => { String::from( "FPoUp")},
            Effect::FinePortaDown{ period_change : u8 } => { String::from( "FPoDn")},
            Effect::FineVolumeSlideUp{ volume_change } => { String::from( "FVSUp")}
            Effect::FineVolumeSlideDown{ volume_change } => { String::from( "FVSDn")}
            Effect::SetVibratoWave{ wave } => { String::from( "VibWv" ) }
            _ =>  { String::from( "....." ) }
        }
    }
}

//impl crate::mod_player::Sample{
//impl super::Sample{
impl Sample{
    fn print(&self) {
        println!("   sample Name: {}", self.name);
        println!("   sample Size: {}", self.size);
        println!("   sample volume: {}, fine tune {}", self.volume, self.fine_tune);
        println!("   repeat Offset: {}, repeat Size {}", self.repeat_offset, self.repeat_size);
    }
}

fn note_string( period : u32 ) -> &'static str{
    let idx = NOTE_FREQUENCY_STRINGS.binary_search_by( | val | val.0.cmp( &period) );
    if idx.is_ok() {
        return  NOTE_FREQUENCY_STRINGS[ idx.unwrap() ].1;
    } else { "..." }
}

/// Prints out one line of note data
pub fn print_line( line :  &Vec<Note> ) {
    for note in line.iter() {
        print!("{} {:02X} {}   ",  note_string( note.period  ), note.sample_number, note.effect.to_string()  );
    }
    println!(""); 
}

/// Print out general info about the song
pub fn print_song_info( song : &Song ) {
    println!("Song: {}", song.name);

    println!("Number of channels: {}", song.format.num_channels);
    println!("Number of samples: {}", song.format.num_samples);
    for sample in &song.samples {
        sample.print()
    }

    println!(" num patterns in song: {}", song.patterns.len());
    println!(" end position: {}", song.end_position);

}