use crate::ast::Ast;

/// Searches `haystack` for the leftmost match of `ast` starting no earlier than `start_at`.
///
/// Returns the populated slot vector on success, or `None` if there is no match.
/// `cap_count` must equal the number of capture groups in `ast` (not counting group 0).
/// `prefix` is an optional first-byte hint used to skip ahead quickly.
pub fn find(
    ast: &Ast,
    haystack: &[u8],
    cap_count: usize,
    start_at: usize,
    prefix: Option<u8>,
) -> Option<Vec<Option<(usize, usize)>>> {
    let mut start = start_at;
    while start <= haystack.len() {
        if let Some(b) = prefix {
            start = memchr::memchr(b, haystack.get(start..)?)? + start;
        }
        let mut slots = vec![None; cap_count + 1];
        slots[0] = Some((start, start));
        if let Some((end, mut out)) = matches(ast, haystack, start, slots).into_iter().next() {
            out[0] = Some((start, end));
            return Some(out);
        }
        start += 1;
    }
    None
}

/// Returns all ways `ast` can match starting at `pos`, each as `(end_pos, slots)`.
fn matches(
    ast: &Ast,
    s: &[u8],
    pos: usize,
    slots: Vec<Option<(usize, usize)>>,
) -> Vec<(usize, Vec<Option<(usize, usize)>>)> {
    match ast {
        Ast::Empty => vec![(pos, slots)],
        Ast::Literal(c) => match literal_end(*c, s, pos) {
            Some(end) => vec![(end, slots)],
            None => Vec::new(),
        },
        Ast::Dot { matches_new_line } => match next_char(s, pos) {
            Some(('\n', _)) if !matches_new_line => Vec::new(),
            None => Vec::new(),
            Some((_, end)) => vec![(end, slots)],
        },
        Ast::Class(class) => match next_char(s, pos) {
            Some((c, end)) if class.matches(c) => vec![(end, slots)],
            _ => Vec::new(),
        },
        Ast::StartLine { multi_line, crlf } => {
            if pos == 0
                || (*multi_line
                    && prev_char(s, pos) == Some('\n')
                    && (!*crlf || next_char(s, pos).is_none_or(|(c, _)| c != '\r')))
            {
                vec![(pos, slots)]
            } else {
                Vec::new()
            }
        }
        Ast::EndLine { multi_line, crlf } => {
            if pos == s.len()
                || (*multi_line && next_char(s, pos).is_some_and(|(c, _)| c == '\n'))
                || (*crlf
                    && next_char(s, pos).is_some_and(|(c, _)| c == '\r')
                    && s.get(pos + 1) == Some(&b'\n'))
            {
                vec![(pos, slots)]
            } else {
                Vec::new()
            }
        }
        Ast::StartText => {
            if pos == 0 {
                vec![(pos, slots)]
            } else {
                Vec::new()
            }
        }
        Ast::EndText => {
            if pos == s.len() {
                vec![(pos, slots)]
            } else {
                Vec::new()
            }
        }
        Ast::WordBoundary(want) => {
            let left = prev_char(s, pos).is_some_and(is_word);
            let right = next_char(s, pos).is_some_and(|(c, _)| is_word(c));
            if (left != right) == *want {
                vec![(pos, slots)]
            } else {
                Vec::new()
            }
        }
        Ast::Concat(nodes) => match_concat(nodes, s, pos, slots),
        Ast::Alt(nodes) => nodes
            .iter()
            .flat_map(|n| matches(n, s, pos, slots.clone()))
            .collect(),
        Ast::Repeat {
            node,
            min,
            max,
            greedy,
        } => repeat(node, s, pos, slots, *min, *max, *greedy),
        Ast::Group { index, node } => matches(node, s, pos, slots)
            .into_iter()
            .map(|(end, mut out)| {
                out[*index] = Some((pos, end));
                (end, out)
            })
            .collect(),
    }
}

/// Matches a sequence of nodes left-to-right, threading slots through each step.
fn match_concat(
    nodes: &[Ast],
    s: &[u8],
    pos: usize,
    slots: Vec<Option<(usize, usize)>>,
) -> Vec<(usize, Vec<Option<(usize, usize)>>)> {
    if let Some((first, rest)) = nodes.split_first() {
        matches(first, s, pos, slots)
            .into_iter()
            .flat_map(|(p, sl)| match_concat(rest, s, p, sl))
            .collect()
    } else {
        vec![(pos, slots)]
    }
}

/// Entry point for repetition: collects results then reverses order for greedy matches.
fn repeat(
    node: &Ast,
    s: &[u8],
    pos: usize,
    slots: Vec<Option<(usize, usize)>>,
    min: usize,
    max: Option<usize>,
    greedy: bool,
) -> Vec<(usize, Vec<Option<(usize, usize)>>)> {
    let mut out = Vec::new();
    repeat_inner(node, s, pos, slots, min, max, 0, &mut out);
    if greedy {
        out.reverse();
    }
    out
}

/// Recursive helper that enumerates all valid repetition counts into `out`.
fn repeat_inner(
    node: &Ast,
    s: &[u8],
    pos: usize,
    slots: Vec<Option<(usize, usize)>>,
    min: usize,
    max: Option<usize>,
    count: usize,
    out: &mut Vec<(usize, Vec<Option<(usize, usize)>>)>,
) {
    if count >= min {
        out.push((pos, slots.clone()));
    }
    if max.is_some_and(|m| count >= m) {
        return;
    }
    for (next, next_slots) in matches(node, s, pos, slots) {
        if next == pos {
            return;
        }
        repeat_inner(node, s, next, next_slots, min, max, count + 1, out);
    }
}

/// Returns the end position if literal `c` matches its UTF-8 bytes at `pos`.
fn literal_end(c: char, s: &[u8], pos: usize) -> Option<usize> {
    let mut buf = [0; 4];
    let lit = c.encode_utf8(&mut buf).as_bytes();
    s.get(pos..pos + lit.len())
        .is_some_and(|b| b == lit)
        .then_some(pos + lit.len())
}

/// Returns the character at `pos` and the offset of the following character.
fn next_char(s: &[u8], pos: usize) -> Option<(char, usize)> {
    let first = *s.get(pos)?;
    let len = if first < 0x80 {
        1
    } else if first & 0b1110_0000 == 0b1100_0000 {
        2
    } else if first & 0b1111_0000 == 0b1110_0000 {
        3
    } else if first & 0b1111_1000 == 0b1111_0000 {
        4
    } else {
        return None;
    };
    let text = core::str::from_utf8(s.get(pos..pos + len)?).ok()?;
    text.chars().next().map(|c| (c, pos + c.len_utf8()))
}

/// Returns the character immediately before `pos`, or `None` at the start.
fn prev_char(s: &[u8], pos: usize) -> Option<char> {
    let start = pos.saturating_sub(4);
    for i in start..pos {
        if let Some((c, end)) = next_char(s, i)
            && end == pos
        {
            return Some(c);
        }
    }
    None
}

/// Returns `true` if `c` is an ASCII word character (`[a-zA-Z0-9_]`).
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Returns the first literal byte of `ast` if the pattern starts with one.
pub fn literal_prefix_byte(ast: &Ast) -> Option<u8> {
    match ast {
        Ast::Literal(c) if c.is_ascii() => Some(*c as u8),
        Ast::Concat(nodes) => nodes.first().and_then(literal_prefix_byte),
        Ast::Group { node, .. } => literal_prefix_byte(node),
        _ => None,
    }
}
