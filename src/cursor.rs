/// Pagination cursor, denoted by a cursor ID.
/// 
/// In order to create a cursor, simply call `Cursor::new()`.
/// This will set the cursor ID to "CREATE".
/// 
/// Once you send a request containing the cursor, the WEBWARE server returns a new cursor ID.
/// If there are no more results, the cursor ID will be "CLOSED".
#[derive(Clone)]
pub struct Cursor {
    /// The cursor ID.
    /// 
    /// Is "CREATE" if the cursor has just been created.
    /// Is "CLOSED" if there are no more results.
    pub cursor_id: String,
    /// The maximum amount of results that will be returned.
    pub max_lines: u32
}

impl Default for Cursor {
    fn default() -> Self {
        Self { cursor_id: "CREATE".to_string(), max_lines: 500 }
    }
}

impl Cursor {
    /// Creates a new cursor.
    pub fn new(max_lines: u32) -> Cursor {
        Cursor {
            cursor_id: "CREATE".to_string(),
            max_lines
        }
    }

    /// Returns whether the cursor is closed.
    pub fn closed(&self) -> bool {
        &self.cursor_id == "CLOSED"
    }

    /// Sets the cursor ID.
    pub fn set_cursor_id(&mut self, cursor_id: String) {
        self.cursor_id = cursor_id;
    }
}