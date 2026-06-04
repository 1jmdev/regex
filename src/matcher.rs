use crate::ast::Ast;

pub type Slots = Vec<Option<(usize, usize)>>;

pub fn find(ast: &Ast, haystack: &str, cap_count: usize, start_at: usize) -> Option<Slots> {
    for start in starts(haystack, start_at) {
        let mut slots = vec![None; cap_count + 1];
        slots[0] = Some((start, start));
        for (end, mut out) in matches(ast, haystack, start, slots) {
            out[0] = Some((start, end));
            return Some(out);
        }
    }
    None
}

fn starts(s: &str, start_at: usize) -> Vec<usize> {
    s.char_indices()
        .map(|(i, _)| i)
        .chain(core::iter::once(s.len()))
        .filter(|&i| i >= start_at)
        .collect()
}

fn next_char(s: &str, pos: usize) -> Option<(char, usize)> {
    s.get(pos..)?
        .chars()
        .next()
        .map(|c| (c, pos + c.len_utf8()))
}

fn prev_char(s: &str, pos: usize) -> Option<char> {
    s.get(..pos)?.chars().next_back()
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub fn matches(ast: &Ast, s: &str, pos: usize, slots: Slots) -> Vec<(usize, Slots)> {
    match ast {
        Ast::Empty => vec![(pos, slots)],
        Ast::Literal(c) => match next_char(s, pos) {
            Some((x, end)) if x == *c => vec![(end, slots)],
            _ => Vec::new(),
        },
        Ast::Dot => match next_char(s, pos) {
            Some(('\n', _)) | None => Vec::new(),
            Some((_, end)) => vec![(end, slots)],
        },
        Ast::Class(class) => match next_char(s, pos) {
            Some((c, end)) if class.matches(c) => vec![(end, slots)],
            _ => Vec::new(),
        },
        Ast::StartLine => {
            if pos == 0 {
                vec![(pos, slots)]
            } else {
                Vec::new()
            }
        }
        Ast::EndLine => {
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
