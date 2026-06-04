use crate::ast::{Ast, Class, ClassItem};
use crate::error::Error;

pub struct Parsed {
    pub ast: Ast,
    pub captures: usize,
}

pub fn parse(pattern: &str) -> Result<Parsed, Error> {
    let mut p = Parser {
        chars: pattern.chars().collect(),
        pos: 0,
        captures: 0,
        case_insensitive: false,
    };
    let ast = p.parse_alt()?;
    if p.peek().is_some() {
        return Err(Error::new("unexpected trailing input"));
    }
    Ok(Parsed {
        ast,
        captures: p.captures,
    })
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    captures: usize,
    case_insensitive: bool,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }
    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

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

    fn parse_concat(&mut self) -> Result<Ast, Error> {
        let mut parts = Vec::new();
        while let Some(c) = self.peek() {
            if c == ')' || c == '|' {
                break;
            }
            parts.push(self.parse_repeat()?);
        }
        Ok(match parts.len() {
            0 => Ast::Empty,
            1 => parts.remove(0),
            _ => Ast::Concat(parts),
        })
    }

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
            let greedy = !self.eat('?');
            node = Ast::Repeat {
                node: Box::new(node),
                min,
                max,
                greedy,
            };
        }
        Ok(node)
    }

    fn parse_braces(&mut self) -> Result<(usize, Option<usize>), Error> {
        self.bump();
        let min = self.number()?;
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
        if let Some(max) = max {
            if max < min {
                return Err(Error::new("invalid repetition range"));
            }
        }
        Ok((min, max))
    }

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

    fn parse_atom(&mut self) -> Result<Ast, Error> {
        match self
            .bump()
            .ok_or_else(|| Error::new("unexpected end of pattern"))?
        {
            '.' => Ok(Ast::Dot),
            '^' => Ok(Ast::StartLine),
            '$' => Ok(Ast::EndLine),
            '(' => {
                if self.eat('?') {
                    if self.eat('i') && self.eat(')') {
                        self.case_insensitive = true;
                        return Ok(Ast::Empty);
                    }
                    return Err(Error::new("unsupported group syntax"));
                }
                self.captures += 1;
                let index = self.captures;
                let node = self.parse_alt()?;
                if !self.eat(')') {
                    return Err(Error::new("unclosed group"));
                }
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

    fn literal(&self, c: char) -> Ast {
        if self.case_insensitive && c.is_ascii_alphabetic() {
            Ast::Class(Class {
                negated: false,
                items: vec![
                    ClassItem::Char(c.to_ascii_lowercase()),
                    ClassItem::Char(c.to_ascii_uppercase()),
                ],
            })
        } else {
            Ast::Literal(c)
        }
    }

    fn parse_escape(&mut self, in_class: bool) -> Result<Ast, Error> {
        let c = self.bump().ok_or_else(|| Error::new("dangling escape"))?;
        Ok(match c {
            'd' => Ast::Class(Class {
                negated: false,
                items: vec![ClassItem::Digit],
            }),
            'D' => Ast::Class(Class {
                negated: true,
                items: vec![ClassItem::Digit],
            }),
            'w' => Ast::Class(Class {
                negated: false,
                items: vec![ClassItem::Word],
            }),
            'W' => Ast::Class(Class {
                negated: true,
                items: vec![ClassItem::Word],
            }),
            's' => Ast::Class(Class {
                negated: false,
                items: vec![ClassItem::Space],
            }),
            'S' => Ast::Class(Class {
                negated: true,
                items: vec![ClassItem::Space],
            }),
            'b' if !in_class => Ast::WordBoundary(true),
            'B' if !in_class => Ast::WordBoundary(false),
            'n' => Ast::Literal('\n'),
            'r' => Ast::Literal('\r'),
            't' => Ast::Literal('\t'),
            c => self.literal(c),
        })
    }

    fn parse_class(&mut self) -> Result<Ast, Error> {
        let negated = self.eat('^');
        let mut items = Vec::new();
        let mut first = true;
        while let Some(c) = self.peek() {
            if c == ']' && !first {
                self.bump();
                return Ok(Ast::Class(Class { negated, items }));
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

    fn push_class_item(&self, items: &mut Vec<ClassItem>, item: ClassItem) {
        match item {
            ClassItem::Char(c) if self.case_insensitive && c.is_ascii_alphabetic() => {
                items.push(ClassItem::Char(c.to_ascii_lowercase()));
                items.push(ClassItem::Char(c.to_ascii_uppercase()));
            }
            item => items.push(item),
        }
    }

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

    fn class_item(&mut self) -> Result<ClassItem, Error> {
        match self
            .bump()
            .ok_or_else(|| Error::new("unclosed character class"))?
        {
            '\\' => match self.parse_escape(true)? {
                Ast::Class(c) if c.items.len() == 1 && !c.negated => Ok(c.items[0].clone()),
                Ast::Literal(c) => Ok(ClassItem::Char(c)),
                _ => Err(Error::new("unsupported class escape")),
            },
            c => Ok(ClassItem::Char(c)),
        }
    }
}
