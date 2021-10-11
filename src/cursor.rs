pub struct Cursor {
    pub cursor_id: String,
    pub max_lines: u32
}

impl Default for Cursor {
    fn default() -> Self {
        Self { cursor_id: "CREATE".to_string(), max_lines: 500 }
    }
}

impl Cursor {
    pub fn new(max_lines: u32) -> Cursor {
        Cursor {
            cursor_id: "CREATE".to_string(),
            max_lines
        }
    }

    pub fn closed(&self) -> bool {
        self.cursor_id == *"CLOSED"
    }

    pub fn set_cursor_id(&mut self, cursor_id: String) {
        self.cursor_id = cursor_id;
    }
}