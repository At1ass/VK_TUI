//! Search and filtering utilities

/// Simple fuzzy matching algorithm
/// Returns true if all characters from needle appear in haystack in order (case-insensitive)
/// Also calculates a score for ranking results
pub fn fuzzy_match(haystack: &str, needle: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(0);
    }

    let haystack_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();

    let mut score = 0;
    let mut haystack_chars = haystack_lower.chars().peekable();
    let mut last_match_pos = 0;

    for (needle_idx, needle_char) in needle_lower.chars().enumerate() {
        let mut found = false;
        let mut pos = last_match_pos;

        while let Some(&hay_char) = haystack_chars.peek() {
            pos += 1;
            haystack_chars.next();

            if hay_char == needle_char {
                found = true;
                last_match_pos = pos;

                // Bonus for consecutive matches
                if needle_idx > 0 && pos == last_match_pos {
                    score += 10;
                }

                // Bonus for matching at word boundaries
                if pos == 1
                    || haystack_lower
                        .chars()
                        .nth(pos - 2)
                        .map(|c| c.is_whitespace() || c == '_' || c == '-')
                        .unwrap_or(false)
                {
                    score += 15;
                }

                score += 1;
                break;
            }
        }

        if !found {
            return None;
        }
    }

    Some(score)
}

/// Filter and rank chats by fuzzy matching against their titles
pub fn filter_chats(chats: &[crate::state::Chat], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..chats.len()).collect();
    }

    let mut matches: Vec<(usize, i32)> = chats
        .iter()
        .enumerate()
        .filter_map(|(idx, chat)| fuzzy_match(&chat.title, query).map(|score| (idx, score)))
        .collect();

    // Sort by score (descending)
    matches.sort_by(|a, b| b.1.cmp(&a.1));

    matches.into_iter().map(|(idx, _)| idx).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_basic() {
        assert!(fuzzy_match("hello world", "hlo").is_some());
        assert!(fuzzy_match("hello world", "hw").is_some());
        assert!(fuzzy_match("hello world", "hew").is_some());
        assert!(fuzzy_match("hello world", "xyz").is_none());
    }

    #[test]
    fn test_fuzzy_match_empty() {
        assert_eq!(fuzzy_match("hello", ""), Some(0));
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        assert!(fuzzy_match("Hello World", "hlo").is_some());
        assert!(fuzzy_match("HELLO", "hel").is_some());
    }

    #[test]
    fn test_fuzzy_match_scoring() {
        let score1 = fuzzy_match("hello world", "hel").unwrap();
        let score2 = fuzzy_match("hello world", "hw").unwrap();
        // Consecutive matches should score higher
        assert!(score1 > score2);
    }
}
