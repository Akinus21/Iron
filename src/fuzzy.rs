// src/fuzzy.rs
/// Lightweight fuzzy + substring matching for command/history filtering.

/// Score a query against a target.
/// Returns a positive score for matches, 0 for no match.
pub fn score(query: &str, target: &str) -> i32 {
    let q = query.to_lowercase();
    let t = target.to_lowercase();

    if q.is_empty() {
        return 1;
    }
    if t.is_empty() {
        return 0;
    }

    // --- 1. Substring match (highest score) ---
    if let Some(pos) = t.find(&q) {
        let mut score = 1000i32;
        // Bonus for shorter targets (more precise match)
        score -= t.len() as i32 / 10;
        // Bonus for earlier match position
        score -= pos as i32 / 5;
        // Extra bonus for match at start of string or after a separator
        if pos == 0 {
            score += 200;
        } else if t
            .as_bytes()
            .get(pos.saturating_sub(1))
            .map_or(false, |c| is_word_boundary(*c))
        {
            score += 100;
        }
        return score.max(1);
    }

    // --- 2. Character-sequence fallback ---
    let mut t_chars = t.chars().peekable();
    let mut q_chars = q.chars();
    let mut score = 0i32;
    let mut consecutive = 0i32;
    let mut prev_was_boundary = true;

    if let Some(first_q) = q_chars.next() {
        // Find first character
        while let Some(tc) = t_chars.next() {
            if tc == first_q {
                score += if prev_was_boundary { 50 } else { 20 };
                consecutive = 1;
                break;
            }
            prev_was_boundary = is_word_boundary_char(tc);
        }

        // Find remaining characters in order
        for qc in q_chars {
            let mut found = false;
            while let Some(tc) = t_chars.next() {
                if tc == qc {
                    score += if prev_was_boundary { 50 } else { 20 };
                    consecutive += 1;
                    // Bonus for consecutive matches
                    if consecutive > 1 {
                        score += consecutive * 10;
                    }
                    found = true;
                    break;
                }
                prev_was_boundary = is_word_boundary_char(tc);
                consecutive = 0;
            }
            if !found {
                return 0; // Could not match all query characters
            }
        }
    }

    if score > 0 {
        // Penalize for target length (shorter is better)
        score -= t.len() as i32 / 20;
        score.max(1)
    } else {
        0
    }
}

/// Filter a list of strings by fuzzy matching, returning top `limit` matches sorted by score.
pub fn filter<'a>(items: &[&'a str], query: &str, limit: usize) -> Vec<&'a str> {
    if query.is_empty() {
        return items.iter().copied().take(limit).collect();
    }

    let mut scored: Vec<(i32, &'a str)> = items
        .iter()
        .copied()
        .map(|item| (score(query, item), item))
        .filter(|(s, _)| *s > 0)
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.len().cmp(&b.1.len())));
    scored.into_iter().map(|(_, item)| item).take(limit).collect()
}

fn is_word_boundary(c: u8) -> bool {
    matches!(c, b'/' | b'.' | b'-' | b'_' | b' ' | b'@' | b':')
}

fn is_word_boundary_char(c: char) -> bool {
    matches!(c, '/' | '.' | '-' | '_' | ' ' | '@' | ':')
}
