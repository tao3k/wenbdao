pub(super) fn populate_identifier_boundaries(fragment: &str, boundaries: &mut Vec<usize>) {
    boundaries.clear();
    let mut chars = fragment.char_indices().peekable();
    let Some((_, mut prev)) = chars.next() else {
        boundaries.push(0);
        return;
    };

    boundaries.push(0);
    while let Some((byte_idx, ch)) = chars.next() {
        let next = chars.peek().map(|(_, next)| *next);

        let lower_to_upper = prev.is_lowercase() && ch.is_uppercase();
        let acronym_to_word =
            prev.is_uppercase() && ch.is_uppercase() && next.is_some_and(char::is_lowercase);
        let alpha_to_digit = prev.is_alphabetic() && ch.is_ascii_digit();
        let digit_to_alpha = prev.is_ascii_digit() && ch.is_alphabetic();

        if lower_to_upper || acronym_to_word || alpha_to_digit || digit_to_alpha {
            boundaries.push(byte_idx);
        }
        prev = ch;
    }
    boundaries.push(fragment.len());
}
