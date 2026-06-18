use crate::consts::{CURSOR_CLOSED, CURSOR_CREATE};

/// Pagination cursor, denoted by a cursor ID.
///
/// In order to create a cursor, simply call `Cursor::new()`.
/// This will set the cursor ID to "CREATE".
///
/// Once you send a request containing the cursor, the WEBWARE server returns a new cursor ID.
/// If there are no more results, the cursor ID will be "CLOSED".
#[derive(Clone, Debug)]
pub(crate) struct Cursor {
    /// The cursor ID.
    ///
    /// Is "CREATE" if the cursor has just been created.
    /// Is "CLOSED" if there are no more results.
    pub cursor_id: String,
}

impl Cursor {
    /// Creates a new cursor, with its id set to "CREATE".
    pub fn new() -> Cursor {
        Cursor {
            cursor_id: CURSOR_CREATE.to_string(),
        }
    }

    /// Returns whether the cursor is closed.
    pub fn closed(&self) -> bool {
        self.cursor_id == CURSOR_CLOSED
    }

    /// Sets the cursor ID.
    pub fn set_cursor_id(&mut self, cursor_id: String) {
        self.cursor_id = cursor_id;
    }
}
