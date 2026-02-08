use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("Invalid number: {0}")]
    InvalidNumber(String),
    #[error("Parse error at position {0}: {1}")]
    ParseError(usize, String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SExp {
    Atom(String),
    List(Vec<SExp>),
}

impl SExp {
    pub fn as_atom(&self) -> Option<&str> {
        match self {
            SExp::Atom(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[SExp]> {
        match self {
            SExp::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_list_mut(&mut self) -> Option<&mut Vec<SExp>> {
        match self {
            SExp::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&SExp> {
        if let SExp::List(items) = self {
            for item in items {
                // Check if this item is a list starting with the key
                if let SExp::List(sublist) = item {
                    if let Some(first) = sublist.first() {
                        if first.as_atom() == Some(key) {
                            // Return the content after the key (as a new list or the second element)
                            if sublist.len() == 2 {
                                return Some(&sublist[1]);
                            } else if sublist.len() > 2 {
                                // Return a reference to the sublist itself for further processing
                                // The caller can access sublist[1], sublist[2], etc.
                                return Some(item);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_all(&self, key: &str) -> Vec<&SExp> {
        let mut results = Vec::new();
        if let SExp::List(items) = self {
            for item in items {
                // Check if this item is a list starting with the key
                if let SExp::List(sublist) = item {
                    if let Some(first) = sublist.first() {
                        if first.as_atom() == Some(key) {
                            // Return the whole sublist for further processing
                            results.push(item);
                        }
                    }
                }
            }
        }
        results
    }
}

impl fmt::Display for SExp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SExp::Atom(s) => {
                // Quote strings that need quoting
                if s.contains(' ') || s.is_empty() || s.starts_with('(') || s.starts_with(')') {
                    write!(f, "\"{}\"", s.replace('"', "\\\""))
                } else {
                    write!(f, "{}", s)
                }
            }
            SExp::List(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
        }
    }
}

pub struct SExpParser {
    input: Vec<char>,
    pos: usize,
}

impl SExpParser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn parse(&mut self) -> Result<SExp, ParseError> {
        self.skip_whitespace();
        if self.is_eof() {
            return Err(ParseError::UnexpectedEof);
        }
        self.parse_sexp()
    }

    fn parse_sexp(&mut self) -> Result<SExp, ParseError> {
        self.skip_whitespace();
        
        if self.is_eof() {
            return Err(ParseError::UnexpectedEof);
        }

        if self.peek() == '(' {
            self.parse_list()
        } else {
            self.parse_atom()
        }
    }

    fn parse_list(&mut self) -> Result<SExp, ParseError> {
        self.expect_char('(')?;
        let mut items = Vec::new();

        loop {
            self.skip_whitespace();
            
            if self.is_eof() {
                return Err(ParseError::UnexpectedEof);
            }

            if self.peek() == ')' {
                self.advance();
                break;
            }

            items.push(self.parse_sexp()?);
        }

        Ok(SExp::List(items))
    }

    fn parse_atom(&mut self) -> Result<SExp, ParseError> {
        self.skip_whitespace();

        if self.peek() == '"' {
            self.parse_string()
        } else {
            self.parse_symbol()
        }
    }

    fn parse_string(&mut self) -> Result<SExp, ParseError> {
        self.expect_char('"')?;
        let mut s = String::new();
        let mut escaped = false;

        while !self.is_eof() {
            let ch = self.peek();
            
            if escaped {
                match ch {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    '\\' => s.push('\\'),
                    '"' => s.push('"'),
                    _ => s.push(ch),
                }
                escaped = false;
                self.advance();
            } else if ch == '\\' {
                escaped = true;
                self.advance();
            } else if ch == '"' {
                self.advance();
                break;
            } else {
                s.push(ch);
                self.advance();
            }
        }

        Ok(SExp::Atom(s))
    }

    fn parse_symbol(&mut self) -> Result<SExp, ParseError> {
        let mut s = String::new();

        while !self.is_eof() {
            let ch = self.peek();
            if ch.is_whitespace() || ch == '(' || ch == ')' {
                break;
            }
            s.push(ch);
            self.advance();
        }

        if s.is_empty() {
            Err(ParseError::UnexpectedToken("empty symbol".to_string()))
        } else {
            Ok(SExp::Atom(s))
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.is_eof() && self.peek().is_whitespace() {
            self.advance();
        }
    }

    fn peek(&self) -> char {
        if self.pos < self.input.len() {
            self.input[self.pos]
        } else {
            '\0'
        }
    }

    fn advance(&mut self) {
        if self.pos < self.input.len() {
            self.pos += 1;
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn expect_char(&mut self, expected: char) -> Result<(), ParseError> {
        if self.is_eof() {
            return Err(ParseError::UnexpectedEof);
        }

        let ch = self.peek();
        if ch == expected {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(format!(
                "Expected '{}', found '{}'",
                expected, ch
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_atom() {
        let mut parser = SExpParser::new("hello");
        let result = parser.parse().unwrap();
        assert_eq!(result, SExp::Atom("hello".to_string()));
    }

    #[test]
    fn test_parse_string() {
        let mut parser = SExpParser::new("\"hello world\"");
        let result = parser.parse().unwrap();
        assert_eq!(result, SExp::Atom("hello world".to_string()));
    }

    #[test]
    fn test_parse_list() {
        let mut parser = SExpParser::new("(a b c)");
        let result = parser.parse().unwrap();
        if let SExp::List(items) = result {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], SExp::Atom("a".to_string()));
            assert_eq!(items[1], SExp::Atom("b".to_string()));
            assert_eq!(items[2], SExp::Atom("c".to_string()));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_nested() {
        let mut parser = SExpParser::new("(a (b c) d)");
        let result = parser.parse().unwrap();
        if let SExp::List(items) = result {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], SExp::Atom("a".to_string()));
            if let SExp::List(nested) = &items[1] {
                assert_eq!(nested.len(), 2);
            } else {
                panic!("Expected nested list");
            }
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_get() {
        // get() finds a sublist whose first element is the key and returns the second element
        let mut parser = SExpParser::new("((key value) other stuff)");
        let sexp = parser.parse().unwrap();
        let value = sexp.get("key").unwrap();
        assert_eq!(value.as_atom(), Some("value"));
    }
}
