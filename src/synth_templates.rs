use std::fs::DirEntry;
use std::{io, fs};
use std::path::Path;

pub fn read_all(operation: &str) -> Vec<String> {
    let path = Path::new("src/scd/synths");

    let mut result: Vec<String> = Vec::new();

    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        let raw_text = fs::read_to_string(path.clone()).unwrap();
        let text = raw_text.replace("{:operation}", operation);

        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

        // Add a postln to the end so that we see a confirmation message in console.
        let with_load_msg = text + &format!("\n\"{} loaded.\".postln;", file_name);

        result.push(with_load_msg);

    }

    result

}

fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}