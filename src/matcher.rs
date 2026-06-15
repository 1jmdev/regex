use crate::ast::Ast;

/// A slot vector holding `(start, end)` pairs for each capture group (index 0 = whole match).
pub type Slots = Vec<Option<(usize, usize)>>;

/// Searches `haystack` for the leftmost match of `ast` starting no earlier than `start_at`.
///
/// Returns the populated slot vector on success, or `None` if there is no match.
/// `cap_count` must equal the number of capture groups in `ast` (not counting group 0).
/// `prefix` is an optional first-character hint used to skip ahead quickly.
pub fn find(
    ast: &Ast,
    haystack: &str,
    cap_count: usize,
    start_at: usize,
    prefix: Option<char>,
) -> Option<Slots> {
    let mut start = start_at;
    while start <= haystack.len() {
        if let Some(c) = prefix {
            start = next_literal_start(haystack, start, c)?;
        }
        let mut slots = vec![None; cap_count + 1];
        slots[0] = Some((start, start));
        if let Some((end, mut out)) = matches(ast, haystack, start, slots).into_iter().next() {
            out[0] = Some((start, end));
            return Some(out);
        }
        start = advance(haystack, start);
    }
    None
}

/// Finds the next position `>= start` where `prefix` appears in `s`.
fn next_literal_start(s: &str, start: usize, prefix: char) -> Option<usize> {
    if prefix.is_ascii() {
        s.get(start..)?.find(prefix).map(|i| start + i)
    } else {
        s.get(start..)?
            .char_indices()
            .find_map(|(i, c)| (c == prefix).then_some(start + i))
    }
}

/// Advances `pos` past the next UTF-8 character, stepping by 1 at end of string.
fn advance(s: &str, pos: usize) -> usize {
    if pos == s.len() {
        pos + 1
    } else {
        pos + s[pos..].chars().next().map_or(1, char::len_utf8)
    }
}

/// Returns the character at `pos` and the offset of the following character.
fn next_char(s: &str, pos: usize) -> Option<(char, usize)> {
    s.get(pos..)?
        .chars()
        .next()
        .map(|c| (c, pos + c.len_utf8()))
}

/// Returns the character immediately before `pos`, or `None` at the start.
fn prev_char(s: &str, pos: usize) -> Option<char> {
    s.get(..pos)?.chars().next_back()
}

/// Returns `true` if `c` is an ASCII word character (`[a-zA-Z0-9_]`).
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Returns all ways `ast` can match starting at `pos`, each as `(end_pos, slots)`.
pub fn matches(ast: &Ast, s: &str, pos: usize, slots: Slots) -> Vec<(usize, Slots)> {
    match ast {
        Ast::Empty => vec![(pos, slots)],
        Ast::Literal(c) => match next_char(s, pos) {
            Some((x, end)) if x == *c => vec![(end, slots)],
            _ => Vec::new(),
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
                    && s.get(pos + 1..).is_some_and(|rest| rest.starts_with('\n')))
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
fn match_concat(nodes: &[Ast], s: &str, pos: usize, slots: Slots) -> Vec<(usize, Slots)> {
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
    s: &str,
    pos: usize,
    slots: Slots,
    min: usize,
    max: Option<usize>,
    greedy: bool,
) -> Vec<(usize, Slots)> {
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
    s: &str,
    pos: usize,
    slots: Slots,
    min: usize,
    max: Option<usize>,
    count: usize,
    out: &mut Vec<(usize, Slots)>,
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
