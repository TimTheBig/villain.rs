use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum TokenType {
    Colon,
    Attribute,
    AttributeValue,
    Interpolation,
    TagOpen,
    TagClose,
    TextNode,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Token {
    pub(crate) token_type: TokenType,
    pub(crate) position: usize,
    pub(crate) value: Option<String>,
}

impl Token {
    pub(crate) fn new_with_value(token_type: TokenType, position: usize, value: &str) -> Self {
        Self {
            token_type,
            position,
            value: Some(value.to_string()),
        }
    }

    pub(crate) fn new(token_type: TokenType, position: usize) -> Self {
        Self {
            token_type,
            position,
            value: None,
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let position = format!("@ {}", self.position);
        let value = self.value.clone().unwrap_or(String::new());

        match self.token_type {
            TokenType::Colon => write!(f, ":"),
            TokenType::Attribute => write!(f, "{value}{position}"),
            TokenType::AttributeValue => {
                write!(f, r#""{value}"{position}"#)
            }
            TokenType::Interpolation => {
                write!(f, r#"{{ {value} }}{position}"#)
            }
            TokenType::TagOpen => write!(f, "<{value}>{position}"),
            TokenType::TagClose => write!(f, "</{value}>{position}"),
            TokenType::TextNode => write!(f, "{value}{position}"),
        }
    }
}
