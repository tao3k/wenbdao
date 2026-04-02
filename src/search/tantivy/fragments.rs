use super::identifier::populate_identifier_boundaries;

pub(super) fn for_each_candidate_fragment<'a>(
    stored_text: &'a str,
    seen_ranges: &mut Vec<(usize, usize)>,
    boundary_scratch: &mut Vec<usize>,
    mut visit: impl FnMut(&'a str),
) {
    seen_ranges.clear();
    push_candidate_fragment(stored_text, stored_text, seen_ranges, &mut visit);
    for fragment in stored_text.split(|ch: char| !ch.is_alphanumeric()) {
        push_candidate_fragment(stored_text, fragment, seen_ranges, &mut visit);
        push_identifier_subfragments(
            stored_text,
            fragment,
            seen_ranges,
            boundary_scratch,
            &mut visit,
        );
    }
}

fn push_candidate_fragment<'a>(
    stored_text: &'a str,
    fragment: &'a str,
    seen_ranges: &mut Vec<(usize, usize)>,
    visit: &mut impl FnMut(&'a str),
) {
    let fragment = fragment.trim();
    if fragment.is_empty() {
        return;
    }
    if seen_ranges
        .iter()
        .any(|&(start, end)| fragment_eq_ignore_case(&stored_text[start..end], fragment))
    {
        return;
    }
    seen_ranges.push(byte_range_in_parent(stored_text, fragment));
    visit(fragment);
}

fn push_identifier_subfragments<'a>(
    stored_text: &'a str,
    fragment: &'a str,
    seen_ranges: &mut Vec<(usize, usize)>,
    boundary_scratch: &mut Vec<usize>,
    visit: &mut impl FnMut(&'a str),
) {
    populate_identifier_boundaries(fragment, boundary_scratch);
    if boundary_scratch.len() <= 2 {
        return;
    }

    for start_idx in 0..(boundary_scratch.len() - 1) {
        for end_idx in (start_idx + 1)..boundary_scratch.len() {
            let start = boundary_scratch[start_idx];
            let end = boundary_scratch[end_idx];
            if start == 0 && end == fragment.len() {
                continue;
            }
            push_candidate_fragment(stored_text, &fragment[start..end], seen_ranges, visit);
        }
    }
}

fn byte_range_in_parent(parent: &str, fragment: &str) -> (usize, usize) {
    let start = fragment.as_ptr() as usize - parent.as_ptr() as usize;
    (start, start + fragment.len())
}

fn fragment_eq_ignore_case(left: &str, right: &str) -> bool {
    left.chars()
        .flat_map(char::to_lowercase)
        .eq(right.chars().flat_map(char::to_lowercase))
}
