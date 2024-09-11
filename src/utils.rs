use crate::note::Note;
use std::fs::File;
use std::io::{self, BufRead, Write};

pub fn complete_item(mut line: String) -> String {
    if line.contains("[ ]") {
        line = line.replace("[ ]", "[x]");
    } else {
        line = line.replace("[x]", "[ ]");
    }
    line
}

pub fn get_notes_from_file() -> Option<Vec<Note>> {
    let mut home_path = std::env::var_os("HOME").unwrap_or("/home/sam".into());
    home_path.push("/.config/keep/keep_config.txt");
    let path = std::path::Path::new(&home_path);
    if let Ok(file) = File::open(path) {
        let reader = io::BufReader::new(file).lines();
        let mut vec = Vec::new();
        for line in reader.flatten() {
            vec.push(note_from_line(line));
        }
        Some(vec)
    } else {
        None
    }
}

pub fn note_from_line(line: String) -> Note {
    let mut parts = line.split(';');
    let title = parts.next().unwrap();
    let mut note = Note::new(title.to_string());

    for part in parts {
        note.items.push(part.to_string())
    }

    note
}

pub fn write_notes_to_file(notes: Vec<Note>) -> io::Result<()> {
    let mut home_path = std::env::var_os("HOME").unwrap_or("/home/sam".into());
    home_path.push("/.config/keep/keep_config.txt");
    let mut file = File::create(home_path).unwrap();

    for note in notes {
        let size = note.items.iter().fold(0, |acc, e| acc + e.len());
        let mut content = String::with_capacity(size + note.title.len());
        content.push_str(&(note.title + ";"));

        for item in note.items {
            content.push_str(&item);
            content.push_str(";");
        }

        content.push('\n');

        file.write_all(content.as_bytes())?;
    }
    Ok(())
}
