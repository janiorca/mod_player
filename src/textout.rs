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

#[rustfmt::skip]
impl Effect {
    fn to_string(&self) -> String {
        return match self {
            Effect::Arpeggio { chord_offset_1, chord_offset_2 } => format!("Arpgi {:02}{:02}", chord_offset_1, chord_offset_2),
            Effect::SlideUp { speed } => format!("SldUp {:>4}", speed),
            Effect::SlideDown { speed } => format!( "SldDn {:>4}", speed ),
            Effect::TonePortamento { speed } => format!("TonPo {:>4}", speed ),
            Effect::Vibrato { speed, amplitude } => format!("Vibra {:02}{:02}",speed,amplitude),
            Effect::TonePortamentoVolumeSlide { volume_change } => format!( "TPVos {:>4}", volume_change),
            Effect::VibratoVolumeSlide { volume_change } => format!( "ViVoS {:>4}", volume_change),
            Effect::Tremolo { speed, amplitude } => format!("Trmlo {:02}{:02}", speed, amplitude),
            Effect::Pan { position } => format!("Pan   {:>5}.", position ),
            Effect::SetSampleOffset { offset } => format!("Offst {:>4}", offset ),
            Effect::VolumeSlide { volume_change } =>  {format!( "VolSl {:>4}", volume_change)  }
            Effect::PositionJump { next_pattern } => format!( "Jump  {:>4}", next_pattern ),
            Effect::SetVolume { volume } => format!("Volme {:>4}", volume),
            Effect::PatternBreak { next_pattern_pos } => format!( "Break {:>4}", next_pattern_pos),
            Effect::SetSpeed { speed } => format!( "Speed {:>4}", speed),
            Effect::SetHardwareFilter { new_state } => format!( "StHwF {:>4}", new_state ),
            Effect::FinePortaUp { period_change } => format!("FPoUp {:>4}", period_change),
            Effect::FinePortaDown { period_change } => format!("FPoDn {:>4}", period_change),
            Effect::Glissando { use_smooth_slide } => format!("Glsnd {:>4}", use_smooth_slide),
            Effect::PatternLoop { arg } => format!("PtnLp {:>4}", arg),
            Effect::TremoloWaveform { wave } => format!( "TrmWv {:>4}", wave ),
            Effect::CoarsePan { pan_pos } => format!("CrPan {:>4}", pan_pos ),
            Effect::RetriggerSample { retrigger_delay } => format!( "ReTrg {:>4}", retrigger_delay),
            Effect::FineVolumeSlideUp { volume_change } => format!("FVSUp {:>4}", volume_change),
            Effect::FineVolumeSlideDown { volume_change } => format!("FVSDn {:>4}", volume_change),
            Effect::CutNote { delay } => format!( "CutNt {:>4}", delay ),
            Effect::DelayedSample { delay_ticks } => format!( "DlySm {:>4}", delay_ticks),
            Effect::DelayedLine { delay_ticks } => format!( "DlyLn {:>4}", delay_ticks),
            Effect::InvertLoop{ loop_position } => format!( "InvLp {:>4}", loop_position ),
            Effect::SetVibratoWave { wave } => format!("VibWv {:>4}", wave ),
            Effect::SetFineTune { fine_tune } => format!("FnTne {:>4}", fine_tune),
            _ => String::from(".........."),
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
        let sample_string;
        if note.sample_number == 0 {
            sample_string = "..".to_string();
        } else {
            sample_string = note.sample_number.to_string();
        }
        print!(
            "{} {:>2} {}   ",
            note_string(note.period),
            sample_string,
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
    //    for sample in &song.samples {
    for sample_num in 0..song.samples.len() {
        println!("Sample #{}", sample_num + 1);
        song.samples[sample_num].print();
    }

    println!(" num patterns in song: {}", song.patterns.len());
    println!(" end position: {}", song.end_position);
    println!(" uses standard note table: {}", song.has_standard_notes);
}
