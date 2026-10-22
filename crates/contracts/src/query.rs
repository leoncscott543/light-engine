//! Structured particle queries and their text form.
//!
//! ```text
//! [select <field>, ... | *]
//! [where <cond> [and <cond> ...]]
//! [order by <field> [asc | desc]]
//! [limit <n>] [offset <n>]
//!
//! cond := <field> <op> <number>        op: < <= > >= = == != <>
//!       | <field> between <lo> and <hi>
//! ```
//!
//! Keywords and field names are case-insensitive. Any clause may be omitted;
//! an empty query selects [`ParticleQuery::DEFAULT_COLUMNS`].

use std::fmt;

use crate::particles::{ParticleField, ParticleRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl CmpOp {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Eq => "=",
            Self::Ne => "!=",
        }
    }

    pub fn eval(self, lhs: f64, rhs: f64) -> bool {
        match self {
            Self::Lt => lhs < rhs,
            Self::Le => lhs <= rhs,
            Self::Gt => lhs > rhs,
            Self::Ge => lhs >= rhs,
            Self::Eq => lhs == rhs,
            Self::Ne => lhs != rhs,
        }
    }
}

/// A single predicate; a query matches when all of its filters match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Filter {
    Compare {
        field: ParticleField,
        op: CmpOp,
        value: f64,
    },
    /// Inclusive on both ends.
    Between {
        field: ParticleField,
        lo: f64,
        hi: f64,
    },
}

impl Filter {
    pub fn field(&self) -> ParticleField {
        match self {
            Self::Compare { field, .. } | Self::Between { field, .. } => *field,
        }
    }

    pub fn matches(&self, row: &ParticleRow) -> bool {
        match *self {
            Self::Compare { field, op, value } => op.eval(row[field.index()], value),
            Self::Between { field, lo, hi } => {
                let v = row[field.index()];
                lo <= v && v <= hi
            }
        }
    }
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compare { field, op, value } => write!(f, "{field} {} {value}", op.symbol()),
            Self::Between { field, lo, hi } => write!(f, "{field} between {lo} and {hi}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    pub fn flip(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParticleQuery {
    /// Columns reported in rows and aggregated in stats.
    pub columns: Vec<ParticleField>,
    pub filters: Vec<Filter>,
    pub sort: Option<(ParticleField, SortOrder)>,
    pub offset: u64,
    /// Maximum rows returned; stats always cover every matching particle.
    pub limit: u32,
}

impl ParticleQuery {
    pub const DEFAULT_COLUMNS: [ParticleField; 7] = [
        ParticleField::Id,
        ParticleField::X,
        ParticleField::Y,
        ParticleField::Z,
        ParticleField::Speed,
        ParticleField::Energy,
        ParticleField::Activity,
    ];
    pub const DEFAULT_LIMIT: u32 = 200;
    pub const MAX_LIMIT: u32 = 10_000;

    pub fn matches(&self, row: &ParticleRow) -> bool {
        self.filters.iter().all(|f| f.matches(row))
    }

    pub fn parse(text: &str) -> Result<Self, QueryError> {
        Parser::new(text)?.query()
    }
}

impl Default for ParticleQuery {
    fn default() -> Self {
        Self {
            columns: Self::DEFAULT_COLUMNS.to_vec(),
            filters: Vec::new(),
            sort: None,
            offset: 0,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

/// Canonical text form; parses back to an equal query.
impl fmt::Display for ParticleQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("select ")?;
        if self.columns.len() == ParticleField::ALL.len()
            && self
                .columns
                .iter()
                .zip(ParticleField::ALL)
                .all(|(a, b)| *a == b)
        {
            f.write_str("*")?;
        } else {
            for (i, c) in self.columns.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{c}")?;
            }
        }
        for (i, filter) in self.filters.iter().enumerate() {
            f.write_str(if i == 0 { " where " } else { " and " })?;
            write!(f, "{filter}")?;
        }
        if let Some((field, order)) = self.sort {
            let order = match order {
                SortOrder::Asc => "asc",
                SortOrder::Desc => "desc",
            };
            write!(f, " order by {field} {order}")?;
        }
        if self.limit != Self::DEFAULT_LIMIT {
            write!(f, " limit {}", self.limit)?;
        }
        if self.offset != 0 {
            write!(f, " offset {}", self.offset)?;
        }
        Ok(())
    }
}

/// A parse error at byte offset `at` of the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryError {
    pub message: String,
    pub at: usize,
}

impl QueryError {
    fn new(at: usize, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            at,
        }
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "col {}: {}", self.at + 1, self.message)
    }
}

impl std::error::Error for QueryError {}

/// Keywords understood by the parser, for completion and highlighting.
pub const KEYWORDS: [&str; 11] = [
    "select", "where", "and", "between", "order", "by", "asc", "desc", "limit", "offset", "*",
];

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Word(String),
    Num(f64),
    Op(CmpOp),
    Comma,
    Star,
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    at: usize,
}

fn tokenize(text: &str) -> Result<Vec<Token>, QueryError> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let start = i;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let tok = if c.is_ascii_alphabetic() || c == b'_' {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            Tok::Word(text[start..i].to_ascii_lowercase())
        } else if c.is_ascii_digit() || c == b'.' || c == b'-' || c == b'+' {
            i += 1;
            while i < bytes.len() {
                let b = bytes[i];
                let exp_sign = (b == b'-' || b == b'+') && matches!(bytes[i - 1], b'e' | b'E');
                if b.is_ascii_digit() || b == b'.' || b == b'e' || b == b'E' || exp_sign {
                    i += 1;
                } else {
                    break;
                }
            }
            let s = &text[start..i];
            Tok::Num(
                s.parse()
                    .map_err(|_| QueryError::new(start, format!("invalid number `{s}`")))?,
            )
        } else {
            let two = text.get(i..i + 2).unwrap_or("");
            let (op, len) = match two {
                "<=" => (Some(CmpOp::Le), 2),
                ">=" => (Some(CmpOp::Ge), 2),
                "==" => (Some(CmpOp::Eq), 2),
                "!=" | "<>" => (Some(CmpOp::Ne), 2),
                _ => match c {
                    b'<' => (Some(CmpOp::Lt), 1),
                    b'>' => (Some(CmpOp::Gt), 1),
                    b'=' => (Some(CmpOp::Eq), 1),
                    _ => (None, 1),
                },
            };
            i += len;
            match (op, c) {
                (Some(op), _) => Tok::Op(op),
                (None, b',') => Tok::Comma,
                (None, b'*') => Tok::Star,
                _ => {
                    let ch = text[start..].chars().next().unwrap_or('?');
                    return Err(QueryError::new(start, format!("unexpected `{ch}`")));
                }
            }
        };
        out.push(Token { tok, at: start });
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    end: usize,
}

impl Parser {
    fn new(text: &str) -> Result<Self, QueryError> {
        Ok(Self {
            tokens: tokenize(text)?,
            pos: 0,
            end: text.len(),
        })
    }

    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos).map(|t| &t.tok)
    }

    fn at(&self) -> usize {
        self.tokens.get(self.pos).map_or(self.end, |t| t.at)
    }

    fn next(&mut self) -> Option<Tok> {
        let tok = self.tokens.get(self.pos).map(|t| t.tok.clone());
        self.pos += 1;
        tok
    }

    fn peek_word(&self, word: &str) -> bool {
        matches!(self.peek(), Some(Tok::Word(w)) if w == word)
    }

    fn eat_word(&mut self, word: &str) -> bool {
        let found = self.peek_word(word);
        if found {
            self.pos += 1;
        }
        found
    }

    fn expect_word(&mut self, word: &str) -> Result<(), QueryError> {
        if self.eat_word(word) {
            Ok(())
        } else {
            Err(QueryError::new(self.at(), format!("expected `{word}`")))
        }
    }

    fn field(&mut self) -> Result<ParticleField, QueryError> {
        let at = self.at();
        match self.next() {
            Some(Tok::Word(w)) => ParticleField::parse(&w)
                .ok_or_else(|| QueryError::new(at, format!("unknown field `{w}`"))),
            _ => Err(QueryError::new(at, "expected a field name")),
        }
    }

    fn number(&mut self) -> Result<f64, QueryError> {
        let at = self.at();
        match self.next() {
            Some(Tok::Num(n)) => Ok(n),
            _ => Err(QueryError::new(at, "expected a number")),
        }
    }

    fn integer(&mut self, what: &str) -> Result<u64, QueryError> {
        let at = self.at();
        let n = self.number()?;
        if n.fract() != 0.0 || n < 0.0 || n > u64::MAX as f64 {
            return Err(QueryError::new(
                at,
                format!("{what} must be a non-negative integer"),
            ));
        }
        Ok(n as u64)
    }

    fn query(mut self) -> Result<ParticleQuery, QueryError> {
        let mut q = ParticleQuery::default();
        if self.eat_word("select") {
            q.columns = self.columns()?;
        }
        if self.eat_word("where") {
            loop {
                q.filters.push(self.filter()?);
                if !self.eat_word("and") {
                    break;
                }
            }
        }
        if self.eat_word("order") {
            self.expect_word("by")?;
            let field = self.field()?;
            let order = if self.eat_word("desc") {
                SortOrder::Desc
            } else {
                self.eat_word("asc");
                SortOrder::Asc
            };
            q.sort = Some((field, order));
        }
        // `limit` and `offset` may appear in either order.
        for _ in 0..2 {
            if self.eat_word("limit") {
                let at = self.at();
                let n = self.integer("limit")?;
                if n > u64::from(ParticleQuery::MAX_LIMIT) {
                    return Err(QueryError::new(
                        at,
                        format!("limit is capped at {}", ParticleQuery::MAX_LIMIT),
                    ));
                }
                q.limit = n as u32;
            } else if self.eat_word("offset") {
                q.offset = self.integer("offset")?;
            }
        }
        if self.peek().is_some() {
            let at = self.at();
            let hint = match self.peek() {
                Some(Tok::Word(w)) if w == "or" => "only `and` is supported",
                _ => "expected `where`, `order by`, `limit`, `offset`, or end of query",
            };
            return Err(QueryError::new(at, hint));
        }
        Ok(q)
    }

    fn columns(&mut self) -> Result<Vec<ParticleField>, QueryError> {
        if matches!(self.peek(), Some(Tok::Star)) {
            self.pos += 1;
            return Ok(ParticleField::ALL.to_vec());
        }
        let mut columns = Vec::new();
        loop {
            let at = self.at();
            let field = self.field()?;
            if columns.contains(&field) {
                return Err(QueryError::new(at, format!("duplicate column `{field}`")));
            }
            columns.push(field);
            if !matches!(self.peek(), Some(Tok::Comma)) {
                break;
            }
            self.pos += 1;
        }
        Ok(columns)
    }

    fn filter(&mut self) -> Result<Filter, QueryError> {
        let field = self.field()?;
        if self.eat_word("between") {
            let lo = self.number()?;
            self.expect_word("and")?;
            let hi = self.number()?;
            return Ok(Filter::Between { field, lo, hi });
        }
        let at = self.at();
        let op = match self.next() {
            Some(Tok::Op(op)) => op,
            _ => return Err(QueryError::new(at, "expected a comparison or `between`")),
        };
        let value = self.number()?;
        Ok(Filter::Compare { field, op, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ParticleField as F;

    #[test]
    fn empty_is_default() {
        assert_eq!(
            ParticleQuery::parse("  ").unwrap(),
            ParticleQuery::default()
        );
    }

    #[test]
    fn parses_every_clause() {
        let q = ParticleQuery::parse(
            "SELECT id, x, act WHERE speed >= 1.5e-1 and r between 0.5 and 2 \
             and lane != 3 ORDER BY energy DESC offset 10 limit 50",
        )
        .unwrap();
        assert_eq!(q.columns, vec![F::Id, F::X, F::Activity]);
        assert_eq!(
            q.filters,
            vec![
                Filter::Compare {
                    field: F::Speed,
                    op: CmpOp::Ge,
                    value: 0.15
                },
                Filter::Between {
                    field: F::Radius,
                    lo: 0.5,
                    hi: 2.0
                },
                Filter::Compare {
                    field: F::Lane,
                    op: CmpOp::Ne,
                    value: 3.0
                },
            ]
        );
        assert_eq!(q.sort, Some((F::Energy, SortOrder::Desc)));
        assert_eq!((q.offset, q.limit), (10, 50));
    }

    #[test]
    fn display_round_trips() {
        for text in [
            "",
            "select * where x < -0.25",
            "select id, energy where activity > 0.8 and z between -1 and 1 order by lz asc limit 7 offset 3",
        ] {
            let q = ParticleQuery::parse(text).unwrap();
            assert_eq!(ParticleQuery::parse(&q.to_string()).unwrap(), q, "{q}");
        }
    }

    #[test]
    fn errors_point_at_the_problem() {
        let err = |t: &str| ParticleQuery::parse(t).unwrap_err();
        assert_eq!(err("select foo").at, 7);
        assert_eq!(err("where x >").at, 9);
        assert_eq!(
            err("where x < 1 or y > 2").message,
            "only `and` is supported"
        );
        assert_eq!(err("select x, x").at, 10);
        assert_eq!(err("limit 1.5").at, 6);
        assert_eq!(err("where x # 1").at, 8);
        assert!(err("limit 999999").message.contains("capped"));
    }

    #[test]
    fn filters_evaluate() {
        let mut row = [0.0; crate::particles::FIELD_COUNT];
        row[F::X.index()] = 1.0;
        let q = ParticleQuery::parse("where x between 1 and 2 and x != 1.5").unwrap();
        assert!(q.matches(&row));
        row[F::X.index()] = 2.5;
        assert!(!q.matches(&row));
    }
}
