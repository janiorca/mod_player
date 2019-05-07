use hound;

fn main() {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create("out.wav", spec).unwrap();
    let song = mod_player::read_mod_file("mod_files/CHIP_SLAYER!.MOD");
    mod_player::textout::print_song_info(&song);
    let mut player_state: mod_player::PlayerState =
        mod_player::PlayerState::new(song.format.num_channels, spec.sample_rate);
    loop {
        let (left, right) = mod_player::next_sample(&song, &mut player_state);
        writer.write_sample(left);
        writer.write_sample(right);
        if player_state.song_has_ended || player_state.has_looped {
            break;
        }
    }
}
