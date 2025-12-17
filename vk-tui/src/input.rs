//! Input helpers for text editing (character-aware positions).

/// Helper to get byte index from char position
pub fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Insert char at character position (not byte position)
pub fn insert_char_at(s: &mut String, char_pos: usize, c: char) {
    let byte_idx = char_to_byte_index(s, char_pos);
    s.insert(byte_idx, c);
}

/// Remove char at character position (not byte position)
pub fn remove_char_at(s: &mut String, char_pos: usize) -> Option<char> {
    let byte_idx = char_to_byte_index(s, char_pos);
    if byte_idx < s.len() {
        Some(s.remove(byte_idx))
    } else {
        None
    }
}

/// Get character at position
pub fn char_at(s: &str, char_pos: usize) -> Option<char> {
    s.chars().nth(char_pos)
}

/// Delete previous word (whitespace + word)
pub fn delete_word(input: &mut String, cursor: &mut usize) {
    while *cursor > 0 && char_at(input, *cursor - 1).is_some_and(|c| c.is_whitespace()) {
        *cursor -= 1;
        remove_char_at(input, *cursor);
    }
    while *cursor > 0 && char_at(input, *cursor - 1).is_some_and(|c| !c.is_whitespace()) {
        *cursor -= 1;
        remove_char_at(input, *cursor);
    }
}
