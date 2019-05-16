//! # for printing information about the mod song
//!
//! text_out contains utility functions for printing out information about mods. Primarily intended to be used for debugging and understanding the progress of the playback
use super::Sample;
use super::{Effect, Note, Song};

static NOTE_FREQUENCY_STRINGS: [(u32, &str); 60] = [
    (57, "B-6"),
    (60, "A#6"),
    (64, "A-6"),
    (67, "G#6"),
    (71, "G-6"),
    (76, "F#6"),
    (80, "F-6"),
    (85, "E-6"),
    (90, "D#6"),
    (95, "D-6"),
    (101, "C#6"),
    (107, "C-6"),
    (113, "B-5"),
    (120, "A#5"),
    (127, "A-5"),
    (135, "G#5"),
    (143, "G-5"),
    (151, "F#5"),
    (160, "F-5"),
    (170, "E-5"),
    (180, "D#5"),
    (190, "D-5"),
    (202, "C#5"),
    (214, "C-5"),
    (226, "B-4"),
    (240, "A#4"),
    (254, "A-4"),
    (269, "G#4"),
    (285, "G-4"),
    (302, "F#4"),
    (320, "F-4"),
    (339, "E-4"),
    (360, "D#4"),
    (381, "D-4"),
    (404, "C#4"),
    (428, "C-4"),
    (453, "B-3"),
    (480, "A#3"),
    (508, "A-3"),
    (538, "G#3"),
    (570, "G-3"),
    (604, "F#3"),
    (640, "F-3"),
    (678, "E-3"),
    (720, "D#3"),
    (762, "D-3"),
    (808, "C#3"),
    (856, "C-3"),
    (907, "B-2"),
    (961, "A#2"),
    (1017, "A-2"),
    (1077, "G#2"),
    (1141, "G-2"),
    (1209, "F#2"),
    (1281, "F-2"),
    (1357, "E-3"),
    (1440, "D#2"),
    (1525, "D-2"),
    (1616, "C#2"),
    (1712, "C-2"),
];

impl Effect {
    fn to_string(&self) -> String {
        return match self {
            Effect::Arpeggio {
                chord_offset_1: _,
                chord_offset_2: _,
            } => String::from("Arpgi"),
            Effect::SlideUp { speed: _ } => String::from("SldUp"),
            Effect::SlideDown { speed: _ } => String::from("SldDn"),
            Effect::TonePortamento { speed: _ } => String::from("TonPo"),
            Effect::Vibrato {
                speed: _,
                amplitude: _,
            } => String::from("Vibra"),
            Effect::TonePortamentoVolumeSlide { volume_change: _ } => String::from("TPVos"),
            Effect::VibratoVolumeSlide { volume_change: _ } => String::from("ViVoS"),
            Effect::Tremolo {
                speed: _,
                amplitude: _,
            } => String::from("Trmlo"),
            Effect::SetSampleOffset { offset: _ } => String::from("Offst"),
            Effect::VolumeSlide { volume_change: _ } => String::from("VolSl"),
            Effect::PositionJump { next_pattern: _ } => String::from("Jump."),
            Effect::SetVolume { volume: _ } => String::from("Volme"),
            Effect::PatternBreak {
                next_pattern_pos: _,
            } => String::from("Break"),
            Effect::SetSpeed { speed: _ } => String::from("Speed"),
            Effect::SetHardwareFilter { new_state: _ } => String::from("StHwF"),
            Effect::FinePortaUp { period_change: _ } => String::from("FPoUp"),
            Effect::FinePortaDown { period_change: _ } => String::from("FPoDn"),
            Effect::PatternLoop { arg: _ } => String::from("PtnLp"),
            Effect::RetriggerSample { retrigger_delay: _ } => String::from("ReTrg"),
            Effect::FineVolumeSlideUp { volume_change: _ } => String::from("FVSUp"),
            Effect::FineVolumeSlideDown { volume_change: _ } => String::from("FVSDn"),
            Effect::CutNote { delay: _ } => String::from("CutNt"),
            Effect::DelayedSample { delay_ticks: _ } => String::from("DlySm"),
            Effect::DelayedLine { delay_ticks: _ } => String::from("DlyLn"),
            Effect::SetVibratoWave { wave: _ } => String::from("VibWv"),
            _ => String::from("....."),
        };
    }
}

//impl crate::mod_player::Sample{
//impl super::Sample{
impl Sample {
    fn print(&self) {
        println!("   sample Name: {}", self.name);
        println!("   sample Size: {}", self.size);
        println!(
            "   sample volume: {}, fine tune {}",
            self.volume, self.fine_tune
        );
        println!(
            "   repeat Offset: {}, repeat Size {}",
            self.repeat_offset, self.repeat_size
        );
    }
}

fn note_string(period: u32) -> &'static str {
    if period == 0 {
        return "...";
    }
    let ret = NOTE_FREQUENCY_STRINGS.binary_search_by(|val| val.0.cmp(&period));
    return match ret {
        Ok(idx) => return NOTE_FREQUENCY_STRINGS[idx].1,
        Err(idx) => {
            if idx == 0 {
                NOTE_FREQUENCY_STRINGS[0].1
            } else if idx == NOTE_FREQUENCY_STRINGS.len() {
                NOTE_FREQUENCY_STRINGS[NOTE_FREQUENCY_STRINGS.len() - 1].1
            } else {
                // Pick the one that is is closer to
                if period - NOTE_FREQUENCY_STRINGS[idx - 1].0
                    < NOTE_FREQUENCY_STRINGS[idx].0 - period
                {
                    NOTE_FREQUENCY_STRINGS[idx - 1].1
                } else {
                    NOTE_FREQUENCY_STRINGS[idx].1
                }
            }
        }
    };
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_string() {
        assert_eq!(note_string(57), "B-6", "Match first note");
        assert_eq!(note_string(1712), "C-2", "Match first note");
        assert_eq!(note_string(2000), "C-2", "Values past lowest map to C-2");
        assert_eq!(note_string(12), "B-6", "Values past highest map to C-2");
        assert_eq!(note_string(0), "...", "zero maps to ellipsis");
        assert_eq!(note_string(128), "A-5", "Umatched go to nearest");
        assert_eq!(note_string(133), "G#5", "Umatched go to nearest");
        assert_eq!(note_string(184), "D#5", "Umatched go to nearest");
        assert_eq!(note_string(185), "D-5", "Umatched go to nearest");
        assert_eq!(note_string(185), "D-5", "Umatched go to nearest");
    }
}

/// Prints out one line of note data
pub fn print_line(line: &Vec<Note>) {
    for note in line.iter() {
        print!(
            "{} {:02X} {}   ",
            note_string(note.period),
            note.sample_number,
            note.effect.to_string()
        );
    }
    println!("");
}

/// Print out general info about the song
pub fn print_song_info(song: &Song) {
    println!("Song: {}", song.name);

    println!("Number of channels: {}", song.format.num_channels);
    println!("Number of samples: {}", song.format.num_samples);
    for sample in &song.samples {
        sample.print()
    }

    println!(" num patterns in song: {}", song.patterns.len());
    println!(" end position: {}", song.end_position);
    println!(" uses standard note table: {}", song.has_standard_notes);
}
