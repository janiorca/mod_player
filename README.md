mod_player is a crate that reads and plays back mod audio files. The mod_player decodes the audio one sample pair (left,right) at a time
that can be streamed to an audio device or a file. 

For playback, only two functions are needed; 
* read_mod_file to read the file into a Song structure
* next_sample to get the next sample

To use the library to decode a mod file and save it to disk ( using the hound audio crate for WAV saving ) 

```rust
use hound;
 
fn main() {
    let spec = hound::WavSpec { 
    channels: 2,
        sample_rate: 48100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create( "out.wav", spec).unwrap();
    let song = mod_player::read_mod_file("BUBBLE_BOBBLE.MOD");
    let mut player_state : mod_player::PlayerState = mod_player::PlayerState::new( 
                                song.format.num_channels, spec.sample_rate );
    loop {
        let ( left, right ) = mod_player::next_sample(&song, &mut player_state);
        writer.write_sample( left  );
        writer.write_sample( right  );
        if player_state.song_has_ended || player_state.has_looped { 
            break;
        }
    }
 }
```