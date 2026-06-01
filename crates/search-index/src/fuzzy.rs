pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    if let Some(first_row) = matrix.first_mut() {
        for (j, cell) in first_row.iter_mut().enumerate().take(b_len + 1) {
            *cell = j;
        }
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

pub fn fuzzy_match(query: &str, target: &str, max_distance: usize) -> Option<f64> {
    let distance = levenshtein_distance(query, target);
    if distance > max_distance {
        return None;
    }
    let max_len = query.len().max(target.len());
    if max_len == 0 {
        return Some(1.0);
    }
    let similarity = 1.0 - (distance as f64 / max_len as f64);
    Some(similarity)
}

pub fn prefix_match(prefix: &str, term: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    term.to_lowercase().starts_with(&prefix.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_strings() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
    }

    #[test]
    fn test_same_string() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_single_edit() {
        assert_eq!(levenshtein_distance("cat", "bat"), 1);
        assert_eq!(levenshtein_distance("cat", "car"), 1);
        assert_eq!(levenshtein_distance("cat", "at"), 1);
        assert_eq!(levenshtein_distance("cat", "cats"), 1);
    }

    #[test]
    fn test_multiple_edits() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_fuzzy_match_perfect() {
        let result = fuzzy_match("test", "test", 0);
        assert_eq!(result, Some(1.0));
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        let result = fuzzy_match("abc", "xyz", 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_fuzzy_match_within_threshold() {
        let result = fuzzy_match("hello", "hallo", 2);
        assert!(result.is_some());
        let score = result.unwrap();
        assert!(score > 0.5);
    }

    #[test]
    fn test_prefix_match_basic() {
        assert!(prefix_match("hel", "hello"));
        assert!(!prefix_match("xyz", "hello"));
    }

    #[test]
    fn test_prefix_match_empty() {
        assert!(prefix_match("", "anything"));
        assert!(prefix_match("", ""));
    }

    #[test]
    fn test_prefix_match_case_insensitive() {
        assert!(prefix_match("HEL", "hello"));
        assert!(prefix_match("hel", "HELLO"));
    }
}
