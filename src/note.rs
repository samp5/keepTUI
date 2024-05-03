use ratatui::style::Color;

pub struct Note {
    pub title: String,
    pub items: Vec<String>,
    pub focused: bool,
    pub color: Color,
}

impl Note {
    pub fn new(title: String) -> Note {
        Note {
            title,
            items: Vec::new(),
            focused: false,
            color: Color::LightBlue,
        }
    }

    pub fn get_note_text(&self) -> String {
        let mut ret = String::new();
        for item in &self.items {
            ret += &item;
            ret += "\n";
        }
        ret
    }

    pub fn get_note_text_vec(&self) -> Vec<String> {
        self.items.clone()
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }

    pub fn unfocus(&mut self) {
        self.focused = false;
    }
}
