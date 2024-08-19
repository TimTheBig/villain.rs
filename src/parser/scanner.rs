use super::token::{Token, TokenType};
use thiserror::Error;

#[derive(PartialEq)]
pub(crate) enum ScannerContext {
    InTag,
    BetweenTags,
}

#[derive(Error, Debug, PartialEq)]
pub(crate) enum ScannerError {
    #[error("Unexpected character: {0} at position {1}")]
    UnexpectedCharacter(char, usize),

    #[error("Unexpected end of file at position {0}")]
    UnexpectedEof(usize),
}

pub(crate) struct Scanner {
    chars: Vec<char>,
    position: usize,
    context: ScannerContext,
    tokens: Vec<Token>,
}

impl Scanner {
    pub fn new(input: String) -> Self {
        Self {
            chars: input.chars().rev().collect(),
            position: 0,
            context: ScannerContext::BetweenTags,
            tokens: Vec::new(),
        }
    }

    fn next(&mut self) -> Option<char> {
        self.position += 1;
        self.chars.pop()
    }

    fn peek(&self) -> Option<char> {
        self.chars.last().copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.next();
                continue;
            }

            break;
        }
    }

    fn collect_name(&mut self) -> String {
        let mut name = String::new();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '-' {
                name.push(c);
                self.next();
                continue;
            }

            break;
        }

        name
    }

    fn collect_until(&mut self, stop: char) -> String {
        let mut value = String::new();

        while let Some(c) = self.peek() {
            if c == stop {
                break;
            }

            value.push(c);
            self.next();
        }

        value
    }

    fn scan_attribute(&mut self, position: usize) -> Result<(), ScannerError> {
        self.skip_whitespace();
        let attribute_name = self.collect_name();
        self.tokens.push(Token::new_with_value(
            TokenType::Attribute,
            position,
            &attribute_name,
        ));

        if let Some('=') = self.peek() {
            self.next();
            self.skip_whitespace();

            if let Some('"') = self.peek() {
                self.next();
                let value = &self.collect_until('"');
                self.next();

                self.tokens.push(Token::new_with_value(
                    TokenType::AttributeValue,
                    self.position,
                    value,
                ));

                return Ok(());
            }
        }
        Ok(())
    }

    fn scan_text_node(&mut self) -> Result<(), ScannerError> {
        let position = self.position;
        let value = self.collect_until('<');
        self.scan_text_node_from_string(position, &value)?;

        Ok(())
    }

    fn scan_text_node_from_string(
        &mut self,
        begin: usize,
        text_to_search: &str,
    ) -> Result<(), ScannerError> {
        let mut position = begin;
        let value = text_to_search;
        // check if interpolation is inside value
        if value.contains("{{") && value.contains("}}") {
            // find how many times interpolation is inside value
            let matches_open = value.matches("{{").count();
            let matches_close = value.matches("}}").count();
            // get index of first opening brace
            let begin_index: usize;
            let end_index: usize;
            if matches_open == matches_close {
                begin_index = value.find("{{").unwrap();
                end_index = value.find("}}").unwrap();
            } else if matches_close > matches_open && matches_open > 0 {
                begin_index = value.rfind("{{").unwrap();
                end_index = value.rfind("}}").unwrap();
            } else if matches_open > matches_close && matches_close > 0 {
                begin_index = value.rfind("{{").unwrap();
                end_index = value.find("}}").unwrap();
            } else {
                return Err(ScannerError::UnexpectedCharacter('}', position));
            }

            // push text before interpolation
            let text_before = &value[..begin_index].trim_start();

            if text_before.contains("{{") && text_before.contains("}}") {
                self.scan_text_node_from_string(position, text_before)?;
            } else if !text_before.is_empty() {
                self.tokens.push(Token::new_with_value(
                    TokenType::TextNode,
                    position,
                    text_before,
                ));
                position += text_before.len();
            }

            // push interpolation
            let interpolation = &value[begin_index + 2..end_index].trim();

            position += interpolation.len();
            if interpolation.is_empty() {
                return Err(ScannerError::UnexpectedCharacter('}', position));
            } else if interpolation.contains("{{") && interpolation.contains("}}") {
                self.scan_text_node_from_string(position, interpolation)?;
            } else {
                self.tokens.push(Token::new_with_value(
                    TokenType::Interpolation,
                    position,
                    interpolation,
                ));
            }

            // push text after interpolation
            let text_after = &value[end_index + 2..].trim_end();
            if text_after.contains("{{") && text_after.contains("}}") {
                self.scan_text_node_from_string(position, text_after)?;
            } else if !text_after.is_empty() {
                self.tokens.push(Token::new_with_value(
                    TokenType::TextNode,
                    position,
                    &value[end_index + 2..],
                ));
            }

            Ok(())
        } else {
            // if there is no closing brace, add text to tokens
            self.tokens
                .push(Token::new_with_value(TokenType::TextNode, position, value));
            Ok(())
        }
    }

    fn scan(&mut self) -> Result<&[Token], ScannerError> {
        let mut tag = String::new();
        let mut append_closing = false;
        while let Some(c) = self.peek() {
            let position = self.position;

            match c {
                '<' => {
                    if self.context == ScannerContext::InTag {
                        return Err(ScannerError::UnexpectedCharacter(c, position));
                    }
                    self.next();

                    // Check if this is a closing tag
                    if let Some('/') = self.peek() {
                        self.next();
                        self.skip_whitespace();

                        let tag = self.collect_name();
                        self.tokens.push(Token::new_with_value(
                            TokenType::TagClose,
                            position,
                            &tag,
                        ));

                        continue;
                    }

                    // Its an open tag
                    self.skip_whitespace();

                    tag = self.collect_name();

                    self.tokens
                        .push(Token::new_with_value(TokenType::TagOpen, position, &tag));

                    self.skip_whitespace();

                    // Continue attribute collection. We are now in a tag
                    self.context = ScannerContext::InTag;
                }
                '/' => {
                    self.next();

                    // If we are in a tag and the next char is a > we are probably looking at a selfclosing tag
                    // So we instruct our tokenizer to fake-add a closing tag
                    if self.context == ScannerContext::InTag {
                        if let Some('>') = self.peek() {
                            append_closing = true;
                        }
                    }
                }
                '>' => {
                    self.next();
                    if self.context == ScannerContext::InTag {
                        if append_closing {
                            self.tokens.push(Token::new_with_value(
                                TokenType::TagClose,
                                position,
                                &tag,
                            ));
                            append_closing = false;
                            tag.clear();
                        }

                        self.context = ScannerContext::BetweenTags;
                    }
                }
                'a'..='z' | 'A'..='Z' | '0'..='9' | ':' | '{' => {
                    if self.context == ScannerContext::InTag {
                        if c == ':' {
                            self.next();
                            self.tokens.push(Token::new(TokenType::Colon, position));
                            continue;
                        }

                        self.scan_attribute(position)?;
                    } else {
                        //
                        self.scan_text_node()?;
                    }
                }
                _ => {
                    self.next();
                }
            }
        }

        if self.context != ScannerContext::BetweenTags {
            return Err(ScannerError::UnexpectedEof(self.position));
        }

        Ok(&self.tokens)
    }
}

impl TryInto<Vec<Token>> for Scanner {
    type Error = ScannerError;

    fn try_into(mut self) -> Result<Vec<Token>, Self::Error> {
        match self.scan() {
            Ok(tokens) => Ok(tokens.to_vec()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::parser::token::TokenType;

    use super::*;

    #[test]
    fn test_gracefully_handles_unclosed_tag() {
        let input = "<template".to_string();
        let scanner = Scanner::new(input);

        let scan: Result<Vec<Token>, ScannerError> = scanner.try_into();

        assert!(scan.is_err());
        assert_eq!(scan.unwrap_err(), ScannerError::UnexpectedEof(9));
    }

    #[test]
    fn test_gracefully_handles_tag_in_unclosed_tag() {
        let input = "<template<".to_string();
        let scanner = Scanner::new(input);

        let scan: Result<Vec<Token>, ScannerError> = scanner.try_into();

        assert!(scan.is_err());
        assert_eq!(scan.unwrap_err(), ScannerError::UnexpectedCharacter('<', 9));
    }

    #[test]
    fn test_scans_short_tag() {
        let input = "<template />".to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "template");
        assert_eq!(tokens[1].token_type, TokenType::TagClose);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "template");
    }

    #[test]
    fn test_scans_short_tag_with_attribute() {
        let input = r#"<template v-model="test" />"#.to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "template");
        assert_eq!(tokens[1].token_type, TokenType::Attribute);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "v-model");
        assert_eq!(tokens[2].token_type, TokenType::AttributeValue);
        assert_eq!(tokens[2].value.as_ref().unwrap(), "test");
        assert_eq!(tokens[3].token_type, TokenType::TagClose);
        assert_eq!(tokens[3].value.as_ref().unwrap(), "template");
    }

    #[test]
    fn test_scans_empty_tag() {
        let input = "<template></template>".to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "template");
        assert_eq!(tokens[1].token_type, TokenType::TagClose);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "template");
    }

    #[test]
    fn test_scans_tag_with_text() {
        let input = "<template>Hello</template>".to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "template");
        assert_eq!(tokens[1].token_type, TokenType::TextNode);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "Hello");
        assert_eq!(tokens[2].token_type, TokenType::TagClose);
        assert_eq!(tokens[2].value.as_ref().unwrap(), "template");
    }

    #[test]
    fn test_scans_empty_tag_with_spaces() {
        let input = "<template  ></  template  >".to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "template");
        assert_eq!(tokens[1].token_type, TokenType::TagClose);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "template");
    }

    #[test]
    fn test_scans_nested_tag() {
        let input = "<span><div></div></span>".to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "span");
        assert_eq!(tokens[1].token_type, TokenType::TagOpen);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "div");
        assert_eq!(tokens[2].token_type, TokenType::TagClose);
        assert_eq!(tokens[2].value.as_ref().unwrap(), "div");
        assert_eq!(tokens[3].token_type, TokenType::TagClose);
        assert_eq!(tokens[3].value.as_ref().unwrap(), "span");
    }

    #[test]
    fn test_scans_tag_with_attributes() {
        let input = r#"<template attr attr2="100"></template>"#.to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "template");
        assert_eq!(tokens[1].token_type, TokenType::Attribute);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "attr");
        assert_eq!(tokens[2].token_type, TokenType::Attribute);
        assert_eq!(tokens[2].value.as_ref().unwrap(), "attr2");
        assert_eq!(tokens[3].token_type, TokenType::AttributeValue);
        assert_eq!(tokens[3].value.as_ref().unwrap(), "100");
        assert_eq!(tokens[4].token_type, TokenType::TagClose);
        assert_eq!(tokens[4].value.as_ref().unwrap(), "template");
    }

    #[test]
    fn test_scans_tag_with_vattributes() {
        let input = r#"<template attr :attr2="100"></template>"#.to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "template");
        assert_eq!(tokens[1].token_type, TokenType::Attribute);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "attr");
        assert_eq!(tokens[2].token_type, TokenType::Colon);
        assert_eq!(tokens[3].token_type, TokenType::Attribute);
        assert_eq!(tokens[3].value.as_ref().unwrap(), "attr2");
        assert_eq!(tokens[4].token_type, TokenType::AttributeValue);
        assert_eq!(tokens[4].value.as_ref().unwrap(), "100");
        assert_eq!(tokens[5].token_type, TokenType::TagClose);
        assert_eq!(tokens[5].value.as_ref().unwrap(), "template");
    }

    #[test]
    fn test_scans_tag_with_text_with_interpolation() {
        let input = r#"<div>Hello {{ username }} </div>"#.to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "div");
        assert_eq!(tokens[1].token_type, TokenType::TextNode);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "Hello ");
        assert_eq!(tokens[2].token_type, TokenType::Interpolation);
        assert_eq!(tokens[2].value.as_ref().unwrap(), "username");
        assert_eq!(tokens[3].token_type, TokenType::TagClose);
        assert_eq!(tokens[3].value.as_ref().unwrap(), "div");
    }

    #[test]
    fn test_scans_tag_with_interpolation() {
        let input = r#"<h1>{{ username }}</h1>"#.to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "h1");
        assert_eq!(tokens[1].token_type, TokenType::Interpolation);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "username");
        assert_eq!(tokens[2].token_type, TokenType::TagClose);
        assert_eq!(tokens[2].value.as_ref().unwrap(), "h1");
    }

    #[test]
    fn test_scans_tag_with_complex_interpolation() {
        let input =
            r#"<h1>Hi {{ username.first }}, {{ username.last }}! How are you?</h1>"#.to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();

        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "h1");
        assert_eq!(tokens[1].token_type, TokenType::TextNode);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "Hi ");
        assert_eq!(tokens[2].token_type, TokenType::Interpolation);
        assert_eq!(tokens[2].value.as_ref().unwrap(), "username.first");
        assert_eq!(tokens[3].token_type, TokenType::TextNode);
        assert_eq!(tokens[3].value.as_ref().unwrap(), ", ");
        assert_eq!(tokens[4].token_type, TokenType::Interpolation);
        assert_eq!(tokens[4].value.as_ref().unwrap(), "username.last");
        assert_eq!(tokens[5].token_type, TokenType::TextNode);
        assert_eq!(tokens[5].value.as_ref().unwrap(), "! How are you?");
        assert_eq!(tokens[6].token_type, TokenType::TagClose);
        assert_eq!(tokens[6].value.as_ref().unwrap(), "h1");
    }

    #[test]
    fn test_scans_tag_with_super_complex_interpolation() {
        let input =
            r#"<h1>Hi {{ username.first }}, {{ username.last }}! How are you?{{ {"a": 1, b: {}} }}</h1>"#.to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();

        assert_eq!(tokens.len(), 8);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "h1");
        assert_eq!(tokens[1].token_type, TokenType::TextNode);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "Hi ");
        assert_eq!(tokens[2].token_type, TokenType::Interpolation);
        assert_eq!(tokens[2].value.as_ref().unwrap(), "username.first");
        assert_eq!(tokens[3].token_type, TokenType::TextNode);
        assert_eq!(tokens[3].value.as_ref().unwrap(), ", ");
        assert_eq!(tokens[4].token_type, TokenType::Interpolation);
        assert_eq!(tokens[4].value.as_ref().unwrap(), "username.last");
        assert_eq!(tokens[5].token_type, TokenType::TextNode);
        assert_eq!(tokens[5].value.as_ref().unwrap(), "! How are you?");
        assert_eq!(tokens[6].token_type, TokenType::Interpolation);
        assert_eq!(tokens[6].value.as_ref().unwrap(), r#"{"a": 1, b: {}}"#);
        assert_eq!(tokens[7].token_type, TokenType::TagClose);
        assert_eq!(tokens[7].value.as_ref().unwrap(), "h1");
    }

    #[test]
    fn test_scans_tag_with_only_super_complex_interpolation() {
        let input = r#"<h1>{{ {"a": 1, b: {}} }}</h1>"#.to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "h1");
        assert_eq!(tokens[1].token_type, TokenType::Interpolation);
        assert_eq!(tokens[1].value.as_ref().unwrap(), r#"{"a": 1, b: {}}"#);
        assert_eq!(tokens[2].token_type, TokenType::TagClose);
        assert_eq!(tokens[2].value.as_ref().unwrap(), "h1");
    }

    #[test]
    fn test_scans_tag_template_and_interpolation_and_attrs() {
        let input =
            r#"<template><h1> {{ msg }} </h1><input v-model="msg" /></template>"#.to_string();
        let scanner = Scanner::new(input);

        let tokens: Vec<Token> = scanner.try_into().unwrap();
        assert_eq!(tokens.len(), 9);
        assert_eq!(tokens[0].token_type, TokenType::TagOpen);
        assert_eq!(tokens[0].value.as_ref().unwrap(), "template");
        assert_eq!(tokens[1].token_type, TokenType::TagOpen);
        assert_eq!(tokens[1].value.as_ref().unwrap(), "h1");
        assert_eq!(tokens[2].token_type, TokenType::Interpolation);
        assert_eq!(tokens[2].value.as_ref().unwrap(), "msg");
        assert_eq!(tokens[3].token_type, TokenType::TagClose);
        assert_eq!(tokens[3].value.as_ref().unwrap(), "h1");
        assert_eq!(tokens[4].token_type, TokenType::TagOpen);
        assert_eq!(tokens[4].value.as_ref().unwrap(), "input");
        assert_eq!(tokens[5].token_type, TokenType::Attribute);
        assert_eq!(tokens[5].value.as_ref().unwrap(), "v-model");
        assert_eq!(tokens[6].token_type, TokenType::AttributeValue);
        assert_eq!(tokens[6].value.as_ref().unwrap(), "msg");
        assert_eq!(tokens[7].token_type, TokenType::TagClose);
        assert_eq!(tokens[7].value.as_ref().unwrap(), "input");
        assert_eq!(tokens[8].token_type, TokenType::TagClose);
        assert_eq!(tokens[8].value.as_ref().unwrap(), "template");
    }
}
