use crc::{crc64, Hasher64};
use mod_player;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::io;
use std::time;
use std::{fs, str};

#[derive(Serialize, Deserialize)]
struct ExpectedChecksums {
    song_checksums: HashMap<String, u64>,
}

fn get_expected_results() -> io::Result<ExpectedChecksums> {
    let expected_json = fs::read_to_string("reg_expected.json")?;
    let result: ExpectedChecksums = serde_json::from_str(&expected_json)?;
    Ok(result)
}

#[test]
// Calculates a checksum for all songs and checks them against expected checksums
// produces a
// Use the following command to run just this test with full output
//  cargo test --test regression -- --nocapture
// Use the following to get release timings
//  cargo test --test regression --release -- --nocapture
fn reg_test() {
    let test_songs = [
        "CHIP_SLAYER!.MOD",
        "BUBBLE_BOBBLE.MOD",
        "cream_of_the_earth.mod",
        "switchback.mod",
        "stardstm.MOD",
        "overload.mod",
        "BOG_WRAITH.mod",
        "wasteland.mod",
        "1 step further.MOD",
        "BALLI.MOD",
        "ballade_pour_adeline.MOD",
        "sarcophaser.mod",
        "chcknbnk.mod",
        "GSLINGER.MOD",
        "19xx.mod",
    ];
    let mut song_checksums: HashMap<String, u64> = HashMap::new();
    let expected_results = get_expected_results().ok();
    if expected_results.is_none() {
        println!("No expected results read. Will produce actuals output for all and then fail")
    }

    for test_song in &test_songs {
        let song = mod_player::read_mod_file(&format!("mod_files/{}", test_song));
        let mut player_state: mod_player::PlayerState =
            mod_player::PlayerState::new(song.format.num_channels, 48100);

        const SOUND_BUFFER_SIZE: usize = 48100 * 2 * 1000;
        let mut sound_data = vec![0.0f32; SOUND_BUFFER_SIZE];
        let before_play = time::Instant::now();
        let mut played_song_length = (SOUND_BUFFER_SIZE as f32) / (2.0f32 * 48100.0f32);
        for pos in (0..SOUND_BUFFER_SIZE).step_by(2) {
            let (left, right) = mod_player::next_sample(&song, &mut player_state);
            sound_data[pos] = left;
            sound_data[pos + 1] = right;
            if player_state.song_has_ended || player_state.has_looped {
                played_song_length = pos as f32 / (2.0f32 * 48100.0f32);
                break;
            }
        }
        let after_play = time::Instant::now();
        let play_time = after_play.duration_since(before_play);
        println!("time for {} is {} uSecs", test_song, play_time.as_micros());
        println!(
            "playspeed: {}",
            played_song_length * 1000_000.0 / (play_time.as_micros() as f32)
        );

        let mut digest = crc64::Digest::new(crc64::ECMA);
        for pos in 0..SOUND_BUFFER_SIZE {
            let sample = sound_data[pos];
            let sample_as_bytes: [u8; 4] = sample.to_bits().to_le_bytes();
            digest.write(&sample_as_bytes);
        }

        let value = digest.sum64();
        song_checksums.insert(test_song.to_string(), value);
        if let Some(ref expected) = expected_results {
            assert_eq!(
                expected.song_checksums.get(*test_song),
                Some(&value),
                "song {} produces the same checksum as before",
                test_song
            );
        }
    }
    let actuals_checksums = ExpectedChecksums { song_checksums };
    let serialized = serde_json::to_string_pretty(&actuals_checksums).unwrap();
    fs::write("reg_actual.json", &serialized).expect("Cant write actual results");

    if expected_results.is_none() {
        println!("Calculated following results. Failing because no expected data found");
        println!("{}", serialized);
        assert!(false);
    }
}
