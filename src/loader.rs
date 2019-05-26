use super::static_tables;
use super::{FormatDescription, Note, Pattern, Sample, Song};
use std::fs;

fn is_standard_note_period(period: u32) -> bool {
    // treat 0 as  standard note because it is not a playable note
    if period == 0 {
        return true;
    }
    return match static_tables::FREQUENCY_TABLE.binary_search(&period) {
        Ok(_idx) => true,
        Err(_idx) => false,
    };
}

// Go through all the notes to determine if it uses only standard notes
// ( this is a requirement for using table based fine tunes )
fn has_standard_notes_only(patterns: &Vec<Pattern>, pattern_table: &Vec<u8>) -> bool {
    for pattern_idx in pattern_table {
        if *pattern_idx as usize >= patterns.len() {
            continue;
        }
        let pattern = &patterns[*pattern_idx as usize];

        for line in &pattern.lines {
            for note in line {
                if !is_standard_note_period(note.period) {
                    return false;
                }
            }
        }
    }
    return true;
}

/**
 * Identify the mod format version based on the tag. If there is not identifiable that it is assumed to be an original mod.
 */
fn get_format(file_data: &Vec<u8>) -> FormatDescription {
    let format_tag = String::from_utf8_lossy(&file_data[1080..1084]);
    println!("formtat tag: {}", format_tag);
    match format_tag.as_ref() {
        "M.K." | "FLT4" | "M!K!" | "4CHN" => FormatDescription {
            num_channels: 4,
            num_samples: 31,
            has_tag: true,
        },
        "8CHN" => FormatDescription {
            num_channels: 8,
            num_samples: 31,
            has_tag: true,
        },
        "12CH" => FormatDescription {
            num_channels: 12,
            num_samples: 31,
            has_tag: true,
        },
        "CD81" => FormatDescription {
            num_channels: 8,
            num_samples: 31,
            has_tag: true,
        },
        "CD61" => {
            panic!("unhandled tag cd61");
        }
        _ => FormatDescription {
            num_channels: 4,
            num_samples: 15,
            has_tag: false,
        },
    }
}

/// Reads a module music file and returns a song structure ready for playing
///
/// # Arguments
/// * `file_name` - the mod file on disk
///
pub fn read_mod_file(file_name: &str) -> Song {
    let file_data: Vec<u8> = fs::read(file_name).expect(&format!("Cant open file {}", &file_name));

    let song_name = String::from_utf8_lossy(&file_data[0..20]);
    let format = get_format(&file_data);

    let mut samples: Vec<Sample> = Vec::new();
    let mut offset: usize = 20;
    for _sample_num in 0..format.num_samples {
        samples.push(Sample::new(&file_data[offset..(offset + 30) as usize]));
        offset += 30;
    }

    // Figure out where to stop and repeat pos ( with option to repeat in the player )
    let num_used_patterns: u8 = file_data[offset];
    let end_position: u8 = file_data[offset + 1];
    offset += 2;
    let pattern_table: Vec<u8> = file_data[offset..(offset + 128)].to_vec();
    offset += 128;

    // Skip the tag if one has been identified
    if format.has_tag {
        offset += 4;
    }

    // Work out how the total size of the sample data at tbe back od the file
    let mut total_sample_size = 0;
    for sample in &mut samples {
        total_sample_size = total_sample_size + sample.size;
    }

    // The pattern take up all the space that remains after everything else has been accounted for
    let total_pattern_size = file_data.len() as u32 - (offset as u32) - total_sample_size;
    let single_pattern_size = format.num_channels * 4 * 64;
    let mut num_patterns = total_pattern_size / single_pattern_size;
    // Find the highest pattern referenced within the used patter references. This is the minimum number of patterns we must load
    let slc = &pattern_table[0..(num_used_patterns as usize)];
    let min_pattern_required = *slc.iter().max().unwrap() + 1;
    // we must read AT LEAST the max_pattern_required patterns
    if (min_pattern_required as u32) > num_patterns {
        num_patterns = min_pattern_required as u32;
    }

    // Read the patterns
    let mut patterns: Vec<Pattern> = Vec::new();
    for _pattern_number in 0..num_patterns {
        let mut pattern = Pattern::new();
        for line in 0..64 {
            for _channel in 0..format.num_channels {
                let note = Note::new(&file_data[offset..(offset + 4)], &format);
                pattern.lines[line].push(note);
                offset += 4;
            }
        }
        patterns.push(pattern);
    }

    // Some mods have weird garbage between the end of the pattern data and the samples
    // ( and some weird files do not have enough for both patterns ans samples. Effectively some storage is used for both!!)
    // Skip the potential garbage by working out the sample position from the back of the file
    offset = (file_data.len() as u32 - total_sample_size) as usize;

    for sample_number in 0..samples.len() {
        let length = samples[sample_number].size;
        for _idx in 0..length {
            samples[sample_number].samples.push(file_data[offset] as i8);
            offset += 1;
        }
    }

    // there are non standard notes, we cant use table based fine tune
    let has_standard_notes = has_standard_notes_only(&patterns, &pattern_table);

    Song {
        name: String::from(song_name),
        format: format,
        samples: samples,
        patterns: patterns,
        pattern_table: pattern_table,
        num_used_patterns: num_used_patterns as u32,
        end_position: end_position as u32,
        has_standard_notes: has_standard_notes,
    }
}
