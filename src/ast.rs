#[derive(Clone, Debug)]
pub enum Ast {
    Empty,
    Literal(char),
    Dot,
    Class(Class),
    StartLine,
    EndLine,
    WordBoundary(bool),
    Concat(Vec<Ast>),
    Alt(Vec<Ast>),
    Repeat {
        node: Box<Ast>,
        min: usize,
        max: Option<usize>,
        greedy: bool,
    },
    Group {
        index: usize,
        node: Box<Ast>,
    },
}

#[derive(Clone, Debug)]
pub struct Class {
    pub negated: bool,
    pub items: Vec<ClassItem>,
}

#[derive(Clone, Debug)]
pub enum ClassItem {
    Char(char),
    Range(char, char),
    Digit,
    Word,
    Space,
}

impl Class {
    pub fn matches(&self, c: char) -> bool {
        let hit = self.items.iter().any(|item| match *item {
            ClassItem::Char(x) => x == c,
            ClassItem::Range(a, b) => a <= c && c <= b,
            ClassItem::Digit => c.is_ascii_digit(),
            ClassItem::Word => c.is_ascii_alphanumeric() || c == '_',
            ClassItem::Space => c.is_whitespace(),
        });
        hit != self.negated
    }
}
