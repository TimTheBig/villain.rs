use thiserror::Error;

use super::token::{Token, TokenType};

#[derive(Error, Debug, PartialEq)]
pub(crate) enum ParserError {
    #[error("Unexpected end of file at position {0}")]
    UnexpectedEof(usize),

    #[error("Unexpected token {0}")]
    UnexpectedToken(Token),

    #[error("Unmatching closing tag. Expected {0} but found {1}")]
    UnmatchingClosing(String, String),
}

#[derive(Debug, PartialEq)]
pub(crate) enum NodeType {
    Root,
    Tag(String),
    Code(String),
    Text(String),
    Attribute(String, Option<Token>, bool),
}

#[derive(Debug, PartialEq)]
pub(crate) struct Node {
    pub(crate) node_type: NodeType,
    pub(crate) children: Vec<Node>,
}

impl Node {
    pub(crate) fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            children: Vec::new(),
        }
    }

    pub(crate) fn add_child(&mut self, node: Node) {
        self.children.push(node);
    }
}

pub(crate) struct Parser {
    tokens: Vec<Token>,
}

impl Parser {
    pub(crate) fn new(mut tokens: Vec<Token>) -> Self {
        tokens.reverse();
        Self { tokens }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.last()
    }

    fn next(&mut self) -> Option<Token> {
        self.tokens.pop()
    }

    fn expect(&mut self, token_type: TokenType) -> Result<Token, ParserError> {
        if let Some(token) = self.next() {
            if token.token_type == token_type {
                Ok(token)
            } else {
                Err(ParserError::UnexpectedToken(token))
            }
        } else {
            Err(ParserError::UnexpectedEof(0))
        }
    }

    fn take_if_present(&mut self, token_type: TokenType) -> Result<Option<Token>, ParserError> {
        if let Some(token) = self.peek() {
            if token.token_type == token_type {
                return Ok(self.next());
            }
        } else {
            return Err(ParserError::UnexpectedEof(0));
        }

        Ok(None)
    }

    fn parse_text_node(&mut self) -> Result<Node, ParserError> {
        let text_node = self.expect(TokenType::TextNode)?.value.unwrap();

        Ok(Node::new(NodeType::Text(text_node)))
    }

    fn parse_interpolation_node(&mut self) -> Result<Node, ParserError> {
        let code = self.expect(TokenType::Interpolation)?.value.unwrap();

        Ok(Node::new(NodeType::Code(code)))
    }

    fn parse_attribute(&mut self, is_bound: bool) -> Result<Node, ParserError> {
        if is_bound {
            self.expect(TokenType::Colon)?;
        }
        let attribute = self.expect(TokenType::Attribute)?.value.unwrap();
        let value = self.take_if_present(TokenType::AttributeValue)?;

        Ok(Node::new(NodeType::Attribute(attribute, value, is_bound)))
    }

    fn parse_tag(&mut self) -> Result<Node, ParserError> {
        let open_tag = self.next().unwrap();
        let tag_name = open_tag.value.as_ref().unwrap();
        let mut node = Node::new(NodeType::Tag(tag_name.clone()));

        while let Some(token) = self.peek() {
            let attribute = match token.token_type {
                TokenType::Colon => self.parse_attribute(true)?,
                TokenType::Attribute => self.parse_attribute(false)?,
                TokenType::TagOpen => self.parse_tag()?,
                TokenType::TextNode => self.parse_text_node()?,
                TokenType::Interpolation => self.parse_interpolation_node()?,
                _ => break,
            };

            node.add_child(attribute);
        }

        let closing = self.expect(TokenType::TagClose)?;
        if closing.value.as_ref() != open_tag.value.as_ref() {
            return Err(ParserError::UnmatchingClosing(
                closing.value.as_ref().unwrap().to_string(),
                open_tag.value.as_ref().unwrap().to_string(),
            ));
        }

        Ok(node)
    }

    fn parse(&mut self) -> Result<Node, ParserError> {
        let mut root = Node::new(NodeType::Root);

        while let Some(token) = self.peek() {
            let next = match token.token_type {
                TokenType::TagOpen => self.parse_tag()?,
                TokenType::TextNode => self.parse_text_node()?,
                _ => return Err(ParserError::UnexpectedToken(token.clone())),
            };

            root.add_child(next);
        }

        Ok(root)
    }
}

impl TryInto<String> for Parser {
    type Error = ParserError;

    fn try_into(mut self) -> Result<String, Self::Error> {
        match self.parse() {
            Ok(root) => Ok(format!("{root:?}")),
            Err(e) => Err(e),
        }
    }
}

impl TryInto<Node> for Parser {
    type Error = ParserError;

    fn try_into(mut self) -> Result<Node, Self::Error> {
        match self.parse() {
            Ok(root) => Ok(root),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::scanner;

    use super::*;

    #[test]
    fn test_parse_text_node() {
        let tokens = vec![Token::new_with_value(
            TokenType::TextNode,
            0,
            "Hello, world!",
        )];
        let mut parser = Parser::new(tokens);

        let expected = Node::new(NodeType::Text("Hello, world!".to_string()));
        let actual = parser.parse_text_node().unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_parse_attribute() {
        let tokens = vec![
            Token::new_with_value(TokenType::Attribute, 0, "class"),
            Token::new_with_value(TokenType::AttributeValue, 0, "foo"),
        ];
        let mut parser = Parser::new(tokens);

        let expected = Node::new(NodeType::Attribute(
            "class".to_string(),
            Some(Token::new_with_value(TokenType::AttributeValue, 0, "foo")),
            false,
        ));
        let actual = parser.parse_attribute(false).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_parse_tag() {
        let tokens = vec![
            Token::new_with_value(TokenType::TagOpen, 0, "div"),
            Token::new_with_value(TokenType::Attribute, 0, "class"),
            Token::new_with_value(TokenType::AttributeValue, 0, "foo"),
            Token::new_with_value(TokenType::TagClose, 0, "div"),
        ];
        let mut parser = Parser::new(tokens);

        let mut expected = Node::new(NodeType::Tag("div".to_string()));
        expected.add_child(Node::new(NodeType::Attribute(
            "class".to_string(),
            Some(Token::new_with_value(TokenType::AttributeValue, 0, "foo")),
            false,
        )));
        let actual = parser.parse_tag().unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_parse_nested_tag_with_attributes() {
        let tokens = vec![
            Token::new_with_value(TokenType::TagOpen, 0, "div"),
            Token::new_with_value(TokenType::Attribute, 0, "class"),
            Token::new_with_value(TokenType::AttributeValue, 0, "foo"),
            Token::new_with_value(TokenType::TagOpen, 0, "span"),
            Token::new_with_value(TokenType::TagClose, 0, "span"),
            Token::new_with_value(TokenType::TagClose, 0, "div"),
        ];
        let mut parser = Parser::new(tokens);

        let mut expected = Node::new(NodeType::Tag("div".to_string()));
        expected.add_child(Node::new(NodeType::Attribute(
            "class".to_string(),
            Some(Token::new_with_value(TokenType::AttributeValue, 0, "foo")),
            false,
        )));
        expected.add_child(Node::new(NodeType::Tag("span".to_string())));

        let actual = parser.parse_tag().unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_parses_vue() {
        let input = r#"
        <script setup>
        import { ref } from "vue";

        const msg = ref("Hello World!");
        </script>

        <template>
          <h1>{{ msg }}</h1>
          <input v-model="msg" />

          <h2>{{ {"a": 1, b: {}} }}</h2>
        </template>
        "#;

        let scanner = scanner::Scanner::new(input.into());
        let parser = Parser::new(scanner.try_into().unwrap());

        let _s: String = parser.try_into().unwrap();
    }
}
