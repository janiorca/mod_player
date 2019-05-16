/*
    Scans for mods in the directory and tries to decode all of them. Used to quickly find problem mod files
*/
use std::env;
use std::path::Path;

fn find_mods(path: &Path) -> Vec<String> {
    let mut mods: Vec<String> = Vec::new();
    for entry in path.read_dir().expect("can't get path") {
        let clean_entry = entry.expect("not found");
        let path = clean_entry.path();
        if path.is_dir() {
            let mut sub_path_mods = find_mods(path.as_path());
            mods.append(&mut sub_path_mods);
        } else {
            let path_str = path.to_str().expect("Bad string");
            let path_string = String::from(path_str);
            let parts: Vec<&str> = path_string.split('.').collect();
            if parts.len() > 1 {
                let extension = parts.last().expect("cant get file extension");
                if extension.eq_ignore_ascii_case("MOD") {
                    mods.push(path_string);
                }
            }
        }
    }
    mods
}

fn main() {
    //    let path = env::current_dir().expect("failed to get current directory");
    let path = std::path::PathBuf::from("C:/work/mods");
    //    let path = std::path::PathBuf::from("C:/work/crate/mod_player/mod_files");
    println!("The current directory is {}", path.display());
    let mods: Vec<String> = find_mods(path.as_path());
    for mod_name in mods {
        println!("Processing: {}", mod_name);
        let song = mod_player::read_mod_file(&mod_name);
        mod_player::textout::print_song_info(&song);
        //    mod_player::textout::
        let mut f = 0.0;
        let mut player_state: mod_player::PlayerState =
            mod_player::PlayerState::new(song.format.num_channels, 48100);
        println!("Start play loop for: {}", mod_name);
        loop {
            let (left, right) = mod_player::next_sample(&song, &mut player_state);
            f += left + right;
            if player_state.song_has_ended || player_state.has_looped {
                break;
            }
        }
    }
}
