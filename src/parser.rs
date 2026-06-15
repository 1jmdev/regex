use crate::ast::{Ast, Class, ClassItem};
use crate::error::Error;

/// The result of a successful parse: an AST and the number of capture groups.
pub struct Parsed {
    pub ast: Ast,
    pub captures: usize,
    pub capture_names: Vec<Option<String>>,
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub unicode: bool,
    pub octal: bool,
    pub nest_limit: Option<u32>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            unicode: true,
            octal: false,
            nest_limit: None,
        }
    }
}

pub fn parse_with_options(pattern: &str, options: Options) -> Result<Parsed, Error> {
    let mut p = Parser {
        chars: pattern.chars().collect(),
        pos: 0,
        captures: 0,
        capture_names: Vec::new(),
        case_insensitive: false,
        multi_line: false,
        dot_matches_new_line: false,
        ignore_whitespace: false,
        swap_greed: false,
        crlf: false,
        unicode: options.unicode,
        octal: options.octal,
        nest_limit: options.nest_limit,
        depth: 0,
    };
    let ast = p.parse_alt()?;
    if p.peek().is_some() {
        return Err(Error::new("unexpected trailing input"));
    }
    Ok(Parsed {
        ast,
        captures: p.captures,
        capture_names: p.capture_names,
    })
}

/// Recursive-descent parser that walks the character stream.
struct Parser {
    chars: Vec<char>,
    pos: usize,
    captures: usize,
    capture_names: Vec<Option<String>>,
    case_insensitive: bool,
    multi_line: bool,
    dot_matches_new_line: bool,
    ignore_whitespace: bool,
    swap_greed: bool,
    crlf: bool,
    unicode: bool,
    octal: bool,
    nest_limit: Option<u32>,
    depth: u32,
}

impl Parser {
    /// Returns the current character without consuming it.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Consumes and returns the current character.
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    /// Consumes the current character if it equals `c`, returning whether it matched.
    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Parses a `|`-separated alternation.
    fn parse_alt(&mut self) -> Result<Ast, Error> {
        let mut parts = vec![self.parse_concat()?];
        while self.eat('|') {
            parts.push(self.parse_concat()?);
        }
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Ast::Alt(parts)
        })
    }

    /// Parses a sequence of atoms until a `|`, `)`, or end of input.
    fn parse_concat(&mut self) -> Result<Ast, Error> {
        let mut parts = Vec::new();
        while let Some(c) = self.peek() {
            if c == ')' || c == '|' {
                break;
            }
            if self.ignore_whitespace && c.is_whitespace() {
                self.bump();
                continue;
            }
            parts.push(self.parse_repeat()?);
        }
        Ok(match parts.len() {
            0 => Ast::Empty,
            1 => parts.remove(0),
            _ => Ast::Concat(parts),
        })
    }

    /// Parses an atom followed by an optional repetition quantifier.
    fn parse_repeat(&mut self) -> Result<Ast, Error> {
        let mut node = self.parse_atom()?;
        loop {
            let (min, max) = match self.peek() {
                Some('*') => {
                    self.bump();
                    (0, None)
                }
                Some('+') => {
                    self.bump();
                    (1, None)
                }
                Some('?') => {
                    self.bump();
                    (0, Some(1))
                }
                Some('{') => self.parse_braces()?,
                _ => break,
            };
            let greedy = !self.eat('?') != self.swap_greed;
            node = Ast::Repeat {
                node: Box::new(node),
                min,
                max,
                greedy,
            };
        }
        Ok(node)
    }

    /// Parses a `{min,max}` repetition bound.
    fn parse_braces(&mut self) -> Result<(usize, Option<usize>), Error> {
        self.bump();
        let min = if self.peek() == Some(',') {
            0
        } else {
            self.number()?
        };
        let max = if self.eat(',') {
            if self.peek() == Some('}') {
                None
            } else {
                Some(self.number()?)
            }
        } else {
            Some(min)
        };
        if !self.eat('}') {
            return Err(Error::new("unclosed repetition"));
        }
        if let Some(max) = max
            && max < min
        {
            return Err(Error::new("invalid repetition range"));
        }
        Ok((min, max))
    }

    /// Parses a decimal integer from the current position.
    fn number(&mut self) -> Result<usize, Error> {
        let start = self.pos;
        let mut n = 0usize;
        while let Some(c) = self.peek() {
            if let Some(d) = c.to_digit(10) {
                self.bump();
                n = n.saturating_mul(10).saturating_add(d as usize);
            } else {
                break;
            }
        }
        if self.pos == start {
            Err(Error::new("expected number"))
        } else {
            Ok(n)
        }
    }

    /// Parses a single atom: literal, metacharacter, group, class, or escape.
    fn parse_atom(&mut self) -> Result<Ast, Error> {
        match self
            .bump()
            .ok_or_else(|| Error::new("unexpected end of pattern"))?
        {
            '.' => Ok(Ast::Dot {
                matches_new_line: self.dot_matches_new_line,
            }),
            '^' => Ok(Ast::StartLine {
                multi_line: self.multi_line,
                crlf: self.crlf,
            }),
            '$' => Ok(Ast::EndLine {
                multi_line: self.multi_line,
                crlf: self.crlf,
            }),
            '(' => {
                if self.eat('?') {
                    return self.parse_special_group();
                }
                self.enter_group()?;
                let index = self.next_capture(None);
                let node = self.parse_alt()?;
                if !self.eat(')') {
                    return Err(Error::new("unclosed group"));
                }
                self.leave_group();
                Ok(Ast::Group {
                    index,
                    node: Box::new(node),
                })
            }
            '[' => self.parse_class(),
            '\\' => self.parse_escape(false),
            ')' | '|' => Err(Error::new("unexpected metacharacter")),
            '*' | '+' | '?' | '{' => Err(Error::new("repetition missing expression")),
            c => Ok(self.literal(c)),
        }
    }

    fn next_capture(&mut self, name: Option<String>) -> usize {
        self.captures += 1;
        self.capture_names.push(name);
        self.captures
    }

    fn parse_special_group(&mut self) -> Result<Ast, Error> {
        if self.eat(':') {
            self.enter_group()?;
            let node = self.parse_alt()?;
            if !self.eat(')') {
                return Err(Error::new("unclosed group"));
            }
            self.leave_group();
            return Ok(node);
        }
        if self.eat('P') && self.eat('<') {
            let name = self.name_until('>')?;
            self.enter_group()?;
            let index = self.next_capture(Some(name));
            let node = self.parse_alt()?;
            if !self.eat(')') {
                return Err(Error::new("unclosed group"));
            }
            self.leave_group();
            return Ok(Ast::Group {
                index,
                node: Box::new(node),
            });
        }
        if self.eat('<') {
            if self.peek() == Some('=') || self.peek() == Some('!') {
                return Err(Error::new("look-around is not supported"));
            }
            let name = self.name_until('>')?;
            self.enter_group()?;
            let index = self.next_capture(Some(name));
            let node = self.parse_alt()?;
            if !self.eat(')') {
                return Err(Error::new("unclosed group"));
            }
            self.leave_group();
            return Ok(Ast::Group {
                index,
                node: Box::new(node),
            });
        }
        if matches!(self.peek(), Some('=') | Some('!')) {
            return Err(Error::new("look-around is not supported"));
        }
        if self.eat('>') {
            return Err(Error::new("atomic groups are not supported"));
        }

        let saved = self.flags();
        let mut saw_flag = false;
        let mut enable = true;
        while let Some(c) = self.peek() {
            match c {
                'i' | 'm' | 's' | 'x' | 'U' | 'R' | 'u' => {
                    saw_flag = true;
                    self.bump();
                    self.set_flag(c, enable);
                }
                '-' => {
                    self.bump();
                    enable = false;
                }
                ':' => {
                    self.bump();
                    if !saw_flag {
                        return Err(Error::new("unsupported group syntax"));
                    }
                    self.enter_group()?;
                    let node = self.parse_alt()?;
                    if !self.eat(')') {
                        return Err(Error::new("unclosed group"));
                    }
                    self.leave_group();
                    self.restore_flags(saved);
                    return Ok(node);
                }
                ')' => {
                    self.bump();
                    if !saw_flag {
                        return Err(Error::new("unsupported group syntax"));
                    }
                    return Ok(Ast::Empty);
                }
                _ => return Err(Error::new("unsupported group syntax")),
            }
        }
        Err(Error::new("unclosed group"))
    }

    fn name_until(&mut self, end: char) -> Result<String, Error> {
        let mut name = String::new();
        while let Some(c) = self.bump() {
            if c == end {
                if name.is_empty() {
                    return Err(Error::new("empty capture name"));
                }
                return Ok(name);
            }
            name.push(c);
        }
        Err(Error::new("unclosed capture name"))
    }

    fn flags(&self) -> (bool, bool, bool, bool, bool, bool, bool) {
        (
            self.case_insensitive,
            self.multi_line,
            self.dot_matches_new_line,
            self.ignore_whitespace,
            self.swap_greed,
            self.crlf,
            self.unicode,
        )
    }

    fn restore_flags(&mut self, flags: (bool, bool, bool, bool, bool, bool, bool)) {
        (
            self.case_insensitive,
            self.multi_line,
            self.dot_matches_new_line,
            self.ignore_whitespace,
            self.swap_greed,
            self.crlf,
            self.unicode,
        ) = flags;
    }

    fn set_flag(&mut self, flag: char, enabled: bool) {
        match flag {
            'i' => self.case_insensitive = enabled,
            'm' => self.multi_line = enabled,
            's' => self.dot_matches_new_line = enabled,
            'x' => self.ignore_whitespace = enabled,
            'U' => self.swap_greed = enabled,
            'R' => self.crlf = enabled,
            'u' => self.unicode = enabled,
            _ => {}
        }
    }

    fn enter_group(&mut self) -> Result<(), Error> {
        self.depth += 1;
        if self.nest_limit.is_some_and(|limit| self.depth > limit) {
            return Err(Error::new("nest limit exceeded"));
        }
        Ok(())
    }

    fn leave_group(&mut self) {
        self.depth -= 1;
    }

    /// Wraps `c` in a two-char class when case-insensitive mode is active.
    fn literal(&self, c: char) -> Ast {
        if self.case_insensitive && c.is_ascii_alphabetic() {
            Ast::Class(Class {
                negated: false,
                items: vec![
                    ClassItem::Char(c.to_ascii_lowercase()),
                    ClassItem::Char(c.to_ascii_uppercase()),
                ],
                intersections: Vec::new(),
            })
        } else {
            Ast::Literal(c)
        }
    }

    /// Parses the character following a `\`.
    fn parse_escape(&mut self, in_class: bool) -> Result<Ast, Error> {
        let c = self.bump().ok_or_else(|| Error::new("dangling escape"))?;
        Ok(match c {
            'd' => Ast::Class(Class {
                negated: false,
                items: vec![ClassItem::Digit],
                intersections: Vec::new(),
            }),
            'D' => Ast::Class(Class {
                negated: true,
                items: vec![ClassItem::Digit],
                intersections: Vec::new(),
            }),
            'w' => Ast::Class(Class {
                negated: false,
                items: vec![ClassItem::Word],
                intersections: Vec::new(),
            }),
            'W' => Ast::Class(Class {
                negated: true,
                items: vec![ClassItem::Word],
                intersections: Vec::new(),
            }),
            's' => Ast::Class(Class {
                negated: false,
                items: vec![ClassItem::Space],
                intersections: Vec::new(),
            }),
            'S' => Ast::Class(Class {
                negated: true,
                items: vec![ClassItem::Space],
                intersections: Vec::new(),
            }),
            'b' if !in_class => Ast::WordBoundary(true),
            'B' if !in_class => Ast::WordBoundary(false),
            'A' if !in_class => Ast::StartText,
            'z' | 'Z' if !in_class => Ast::EndText,
            'G' if !in_class => Ast::StartText,
            'n' => Ast::Literal('\n'),
            'r' => Ast::Literal('\r'),
            't' => Ast::Literal('\t'),
            'x' => Ast::Literal(self.hex_escape()?),
            'u' => {
                if !self.unicode {
                    return Err(Error::new(
                        "unicode escape not allowed when unicode mode is disabled",
                    ));
                }
                Ast::Literal(self.unicode_escape()?)
            }
            'p' | 'P' => {
                if !self.unicode {
                    return Err(Error::new(
                        "unicode class not allowed when unicode mode is disabled",
                    ));
                }
                let negated = c == 'P';
                Ast::Class(Class {
                    negated,
                    items: vec![ClassItem::UnicodeProperty(self.property_name()?)],
                    intersections: Vec::new(),
                })
            }
            '0'..='7' => {
                if !self.octal {
                    return Err(Error::new("octal escape not allowed"));
                }
                Ast::Literal(self.octal_escape(c)?)
            }
            c => self.literal(c),
        })
    }

    fn hex_escape(&mut self) -> Result<char, Error> {
        if self.eat('{') {
            let value = self.hex_until('}')?;
            return char::from_u32(value).ok_or_else(|| Error::new("invalid hex escape"));
        }
        let a = self.hex_digit()?;
        let b = self.hex_digit()?;
        char::from_u32((a << 4) | b).ok_or_else(|| Error::new("invalid hex escape"))
    }

    fn unicode_escape(&mut self) -> Result<char, Error> {
        if self.eat('{') {
            let value = self.hex_until('}')?;
            return char::from_u32(value).ok_or_else(|| Error::new("invalid unicode escape"));
        }
        let mut value = 0;
        for _ in 0..4 {
            value = (value << 4) | self.hex_digit()?;
        }
        char::from_u32(value).ok_or_else(|| Error::new("invalid unicode escape"))
    }

    fn property_name(&mut self) -> Result<String, Error> {
        if self.eat('{') {
            return self.name_until('}');
        }
        self.bump()
            .map(|c| c.to_string())
            .ok_or_else(|| Error::new("missing unicode property"))
    }

    fn hex_until(&mut self, end: char) -> Result<u32, Error> {
        let mut value = 0u32;
        let mut saw = false;
        while self.peek().is_some() && self.peek() != Some(end) {
            saw = true;
            value = (value << 4) | self.hex_digit()?;
        }
        if !saw || !self.eat(end) {
            return Err(Error::new("invalid hex escape"));
        }
        Ok(value)
    }

    fn hex_digit(&mut self) -> Result<u32, Error> {
        self.bump()
            .and_then(|c| c.to_digit(16))
            .ok_or_else(|| Error::new("invalid hex escape"))
    }

    fn octal_escape(&mut self, first: char) -> Result<char, Error> {
        let mut value = first.to_digit(8).unwrap();
        for _ in 0..2 {
            if let Some(c) = self.peek().and_then(|c| c.to_digit(8).map(|d| (c, d))) {
                self.bump();
                value = value * 8 + c.1;
            }
        }
        char::from_u32(value).ok_or_else(|| Error::new("invalid octal escape"))
    }

    /// Parses a `[...]` character class body.
    fn parse_class(&mut self) -> Result<Ast, Error> {
        Ok(Ast::Class(self.parse_class_expr()?))
    }

    fn parse_class_expr(&mut self) -> Result<Class, Error> {
        let mut class = self.parse_class_union()?;
        while self.peek() == Some('&') && self.chars.get(self.pos + 1) == Some(&'&') {
            self.bump();
            self.bump();
            class.intersections.push(self.parse_class_union()?);
        }
        if !self.eat(']') {
            return Err(Error::new("unclosed character class"));
        }
        Ok(class)
    }

    fn parse_class_union(&mut self) -> Result<Class, Error> {
        let negated = self.eat('^');
        let mut items = Vec::new();
        let mut first = true;
        while let Some(c) = self.peek() {
            if c == ']' && !first {
                return Ok(Class {
                    negated,
                    items,
                    intersections: Vec::new(),
                });
            }
            if !first && c == '&' && self.chars.get(self.pos + 1) == Some(&'&') {
                return Ok(Class {
                    negated,
                    items,
                    intersections: Vec::new(),
                });
            }
            first = false;
            let start = self.class_item()?;
            if self.peek() == Some('-') {
                self.bump();
                if self.peek() == Some(']') {
                    items.push(start);
                    items.push(ClassItem::Char('-'));
                    continue;
                }
                let end = self.class_item()?;
                match (start, end) {
                    (ClassItem::Char(a), ClassItem::Char(b)) if a <= b => {
                        self.push_class_range(&mut items, a, b)
                    }
                    _ => return Err(Error::new("invalid character class range")),
                }
            } else {
                self.push_class_item(&mut items, start);
            }
        }
        Err(Error::new("unclosed character class"))
    }

    /// Appends `item` to `items`, expanding both cases when case-insensitive.
    fn push_class_item(&self, items: &mut Vec<ClassItem>, item: ClassItem) {
        match item {
            ClassItem::Char(c) if self.case_insensitive && c.is_ascii_alphabetic() => {
                items.push(ClassItem::Char(c.to_ascii_lowercase()));
                items.push(ClassItem::Char(c.to_ascii_uppercase()));
            }
            item => items.push(item),
        }
    }

    /// Appends a range to `items`, adding the opposite-case range when case-insensitive.
    fn push_class_range(&self, items: &mut Vec<ClassItem>, a: char, b: char) {
        items.push(ClassItem::Range(a, b));
        if self.case_insensitive && a.is_ascii_lowercase() && b.is_ascii_lowercase() {
            items.push(ClassItem::Range(
                a.to_ascii_uppercase(),
                b.to_ascii_uppercase(),
            ));
        } else if self.case_insensitive && a.is_ascii_uppercase() && b.is_ascii_uppercase() {
            items.push(ClassItem::Range(
                a.to_ascii_lowercase(),
                b.to_ascii_lowercase(),
            ));
        }
    }

    /// Parses a single item inside a character class, handling `\` escapes.
    fn class_item(&mut self) -> Result<ClassItem, Error> {
        match self
            .bump()
            .ok_or_else(|| Error::new("unclosed character class"))?
        {
            '\\' => match self.parse_escape(true)? {
                Ast::Class(c) if c.items.len() == 1 && !c.negated && c.intersections.is_empty() => {
                    Ok(c.items[0].clone())
                }
                Ast::Class(c) => Ok(ClassItem::Class(Box::new(c))),
                Ast::Literal(c) => Ok(ClassItem::Char(c)),
                _ => Err(Error::new("unsupported class escape")),
            },
            '[' => Ok(ClassItem::Class(Box::new(self.parse_class_expr()?))),
            c => Ok(ClassItem::Char(c)),
        }
    }
}
