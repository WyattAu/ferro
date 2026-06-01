use crate::error::SearchError;

#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Term(String),
    Phrase(Vec<String>),
    And(Box<Query>, Box<Query>),
    Or(Box<Query>, Box<Query>),
    Not(Box<Query>),
    Field(String, String),
    Boost(Box<Query>, f64),
}

pub struct QueryParser;

impl QueryParser {
    pub fn parse(input: &str) -> Result<Query, SearchError> {
        let tokens = tokenize_query(input);
        if tokens.is_empty() {
            return Err(SearchError::EmptyQuery);
        }
        let mut parser = ParserState::new(tokens);
        parser.parse_or()
    }
}

struct ParserState {
    tokens: Vec<Token>,
    pos: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    And,
    Or,
    Not,
    Quote,
    Colon,
    LParen,
    RParen,
}

impl ParserState {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    fn parse_or(&mut self) -> Result<Query, SearchError> {
        let mut left = self.parse_and()?;
        while let Some(Token::Or) = self.peek() {
            self.advance();
            let right = self.parse_and()?;
            left = Query::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Query, SearchError> {
        let mut left = self.parse_not()?;
        while let Some(Token::And) = self.peek() {
            self.advance();
            let right = self.parse_not()?;
            left = Query::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Query, SearchError> {
        if let Some(Token::Not) = self.peek() {
            self.advance();
            let inner = self.parse_primary()?;
            return Ok(Query::Not(Box::new(inner)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Query, SearchError> {
        match self.peek() {
            Some(Token::LParen) => {
                self.advance();
                let inner = self.parse_or()?;
                if let Some(Token::RParen) = self.peek() {
                    self.advance();
                }
                Ok(inner)
            }
            Some(Token::Quote) => {
                self.advance();
                let mut terms = Vec::new();
                while let Some(token) = self.peek() {
                    match token {
                        Token::Quote => {
                            self.advance();
                            break;
                        }
                        Token::Word(w) => {
                            terms.push(w.clone());
                            self.advance();
                        }
                        _ => {
                            self.advance();
                        }
                    }
                }
                if terms.is_empty() {
                    return Err(SearchError::EmptyQuery);
                }
                Ok(Query::Phrase(terms))
            }
            Some(Token::Word(word)) => {
                let word = word.clone();
                self.advance();

                if let Some(Token::Colon) = self.peek() {
                    self.advance();
                    if let Some(Token::Word(value)) = self.peek() {
                        let value = value.clone();
                        self.advance();
                        return Ok(Query::Field(word, value));
                    }
                }

                Ok(Query::Term(word))
            }
            Some(Token::Not) => {
                self.advance();
                let inner = self.parse_primary()?;
                Ok(Query::Not(Box::new(inner)))
            }
            _ => Err(SearchError::UnexpectedToken(format!("{:?}", self.peek()))),
        }
    }
}

fn tokenize_query(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '"' => {
                tokens.push(Token::Quote);
                chars.next();
            }
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            ':' => {
                tokens.push(Token::Colon);
                chars.next();
            }
            ' ' | '\t' | '\n' => {
                chars.next();
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == '"' || c == '(' || c == ')' || c == ':' {
                        break;
                    }
                    word.push(c);
                    chars.next();
                }
                match word.to_uppercase().as_str() {
                    "AND" => tokens.push(Token::And),
                    "OR" => tokens.push(Token::Or),
                    "NOT" => tokens.push(Token::Not),
                    _ => tokens.push(Token::Word(word)),
                }
            }
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_term() {
        let q = QueryParser::parse("hello").unwrap();
        assert_eq!(q, Query::Term("hello".to_string()));
    }

    #[test]
    fn test_parse_and_query() {
        let q = QueryParser::parse("foo AND bar").unwrap();
        assert_eq!(
            q,
            Query::And(
                Box::new(Query::Term("foo".to_string())),
                Box::new(Query::Term("bar".to_string())),
            )
        );
    }

    #[test]
    fn test_parse_or_query() {
        let q = QueryParser::parse("foo OR bar").unwrap();
        assert_eq!(
            q,
            Query::Or(
                Box::new(Query::Term("foo".to_string())),
                Box::new(Query::Term("bar".to_string())),
            )
        );
    }

    #[test]
    fn test_parse_not_query() {
        let q = QueryParser::parse("NOT foo").unwrap();
        assert_eq!(q, Query::Not(Box::new(Query::Term("foo".to_string()))));
    }

    #[test]
    fn test_parse_phrase() {
        let q = QueryParser::parse("\"hello world\"").unwrap();
        assert_eq!(
            q,
            Query::Phrase(vec!["hello".to_string(), "world".to_string()])
        );
    }

    #[test]
    fn test_parse_field_query() {
        let q = QueryParser::parse("name:value").unwrap();
        assert_eq!(q, Query::Field("name".to_string(), "value".to_string()));
    }

    #[test]
    fn test_parse_complex_query() {
        let q = QueryParser::parse("a AND b OR NOT c").unwrap();
        matches!(q, Query::Or(_, _));
    }

    #[test]
    fn test_parse_empty_query() {
        let result = QueryParser::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_parenthesized() {
        let q = QueryParser::parse("(a OR b) AND c").unwrap();
        assert_eq!(
            q,
            Query::And(
                Box::new(Query::Or(
                    Box::new(Query::Term("a".to_string())),
                    Box::new(Query::Term("b".to_string())),
                )),
                Box::new(Query::Term("c".to_string())),
            )
        );
    }

    #[test]
    fn test_precedence_and_over_or() {
        let q = QueryParser::parse("a OR b AND c").unwrap();
        match q {
            Query::Or(left, right) => {
                assert!(matches!(*left, Query::Term(_)));
                assert!(matches!(*right, Query::And(_, _)));
            }
            _ => panic!("Expected OR at top level"),
        }
    }
}
