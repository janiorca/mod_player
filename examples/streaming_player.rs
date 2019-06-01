extern crate cpal;
use hound;

use std::sync;
use std::sync::mpsc;
use std::thread;

enum PlayerCommand {
    Stop {},
}

fn setup_stream(song: sync::Arc<mod_player::Song>) -> mpsc::Sender<PlayerCommand> {
    let device = cpal::default_output_device().expect("Failed to get default output device");
    println!("Sound device: {}", device.name());

    let format = device
        .default_output_format()
        .expect("Failed to get default output format");
    let fmt = match format.data_type {
        cpal::SampleFormat::I16 => "i16",
        cpal::SampleFormat::U16 => "u16",
        cpal::SampleFormat::F32 => "f32",
    };
    println!(
        "Sample rate: {}    Sample format: {}       Channels: {}",
        format.sample_rate.0, fmt, format.channels
    );

    let event_loop = cpal::EventLoop::new();
    let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
    event_loop.play_stream(stream_id.clone());

    let mut player_state: mod_player::PlayerState =
        mod_player::PlayerState::new(song.format.num_channels, format.sample_rate.0);
    let mut last_line_pos = 9999;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        event_loop.run(move |_, data| {
            if player_state.current_line != last_line_pos {
                if player_state.current_line == 0 {
                    println!("");
                }
                print!(
                    "{:>2}:{:>2}  ",
                    player_state.song_pattern_position, player_state.current_line
                );
                mod_player::textout::print_line(player_state.get_song_line(&song));
                last_line_pos = player_state.current_line;
            }
            match data {
                cpal::StreamData::Output {
                    buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer),
                } => {
                    for sample in buffer.chunks_mut(format.channels as usize) {
                        let (left, right) = mod_player::next_sample(&song, &mut player_state);
                        sample[0] = left;
                        sample[1] = right;
                    }
                }
                _ => (),
            }
        });
    });
    tx
}

fn main() {
    let song = sync::Arc::new(mod_player::read_mod_file("mod_files/chcknbnk.mod"));

    mod_player::textout::print_song_info(&song);
    let tx = setup_stream(song.clone());
    loop {
        let mut command = String::new();
        std::io::stdin().read_line(&mut command);
        return;
    }
}
