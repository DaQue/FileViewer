pub(crate) fn recompute_count(query: &str, text: &str) -> usize {
    if query.is_empty() { return 0; }
    let q = query.to_ascii_lowercase();
    text.to_ascii_lowercase().matches(&q).count()
}

pub(crate) fn next_index(current: usize, count: usize) -> usize {
    if count == 0 { 0 } else { (current + 1) % count }
}

pub(crate) fn prev_index(current: usize, count: usize) -> usize {
    if count == 0 { 0 } else if current == 0 { count - 1 } else { current - 1 }
}

pub(crate) fn find_target_line(text: &str, query: &str, target_idx: usize) -> Option<usize> {
    if query.is_empty() { return None; }
    let q = query.to_ascii_lowercase();
    let mut global = 0usize;
    for (i, line) in text.lines().enumerate() {
        let mut rest = line.to_ascii_lowercase();
        while let Some(pos) = rest.find(&q) {
            if global == target_idx { return Some(i); }
            global += 1;
            let next = pos + q.len();
            if next >= rest.len() { break; }
            rest = rest[next..].to_string();
        }
    }
    None
}
