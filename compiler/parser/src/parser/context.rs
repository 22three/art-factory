// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the Leo library.

// The Leo library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Leo library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the Leo library. If not, see <https://www.gnu.org/licenses/>.

use crate::{assert_no_whitespace, tokenizer::*, Token, KEYWORD_TOKENS};

use leo_ast::*;
use leo_errors::emitter::Handler;
use leo_errors::{LeoError, ParserError, Result};
use leo_span::{Span, Symbol};

use std::{borrow::Cow, unreachable};

/// Stores a program in tokenized format plus additional context.
/// May be converted into a [`Program`] AST by parsing all tokens.
pub struct ParserContext<'a> {
    #[allow(dead_code)]
    pub(crate) handler: &'a Handler,
    tokens: Vec<SpannedToken>,
    end_span: Span,
    // true if parsing an expression for if and loop statements -- means circuit inits are not legal
    pub(crate) disallow_circuit_construction: bool,
}

impl Iterator for ParserContext<'_> {
    type Item = SpannedToken;

    fn next(&mut self) -> Option<SpannedToken> {
        self.bump()
    }
}

impl<'a> ParserContext<'a> {
    ///
    /// Returns a new [`ParserContext`] type given a vector of tokens.
    ///
    pub fn new(handler: &'a Handler, mut tokens: Vec<SpannedToken>) -> Self {
        tokens.reverse();
        // todo: performance optimization here: drain filter
        tokens = tokens
            .into_iter()
            .filter(|x| !matches!(x.token, Token::CommentLine(_) | Token::CommentBlock(_)))
            .collect();
        Self {
            handler,
            end_span: tokens
                .iter()
                .find(|x| !x.span.content.trim().is_empty())
                .map(|x| x.span.clone())
                .unwrap_or_default(),
            tokens,
            disallow_circuit_construction: false,
        }
    }

    /// Returns the current token if there is one.
    pub fn peek_option(&self) -> Option<&SpannedToken> {
        self.tokens.last()
    }

    /// Emit the error `err`.
    pub(crate) fn emit_err(&self, err: ParserError) {
        self.handler.emit_err(err.into());
    }

    ///
    /// Returns an unexpected end of function [`SyntaxError`].
    ///
    pub fn eof(&self) -> LeoError {
        ParserError::unexpected_eof(&self.end_span).into()
    }

    ///
    /// Returns a reference to the next SpannedToken or error if it does not exist.
    ///
    pub fn peek_next(&self) -> Result<&SpannedToken> {
        self.tokens.get(self.tokens.len() - 2).ok_or_else(|| self.eof())
    }

    ///
    /// Returns a reference to the current SpannedToken or error if it does not exist.
    ///
    pub fn peek(&self) -> Result<&SpannedToken> {
        self.tokens.last().ok_or_else(|| self.eof())
    }

    ///
    /// Returns a reference to the next Token.
    ///
    pub fn peek_token(&self) -> Cow<'_, Token> {
        self.peek_option()
            .map(|x| &x.token)
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(Token::Eof))
    }

    ///
    /// Returns true if the next token exists.
    ///
    pub fn has_next(&self) -> bool {
        !self.tokens.is_empty()
    }

    /// Advances the current token.
    pub fn bump(&mut self) -> Option<SpannedToken> {
        self.tokens.pop()
    }

    ///
    /// Removes the next token if it exists and returns it, or [None] if
    /// the next token does not exist.
    ///
    pub fn eat(&mut self, token: Token) -> Option<SpannedToken> {
        if let Some(SpannedToken { token: inner, .. }) = self.peek_option() {
            if &token == inner {
                return self.bump();
            }
        }
        None
    }

    ///
    /// Appends a token to the back of the vector.
    ///
    pub fn backtrack(&mut self, token: SpannedToken) {
        self.tokens.push(token);
    }

    ///
    /// Removes the next token if it is a [`Token::Ident(_)`] and returns it, or [None] if
    /// the next token is not a [`Token::Ident(_)`] or if the next token does not exist.
    ///
    pub fn eat_identifier(&mut self) -> Option<Identifier> {
        if let Some(SpannedToken {
            token: Token::Ident(_), ..
        }) = self.peek_option()
        {
            if let SpannedToken {
                token: Token::Ident(name),
                span,
            } = self.bump().unwrap()
            {
                return Some(Identifier { name, span });
            } else {
                unreachable!("eat_identifier_ shouldn't produce this")
            }
        }
        None
    }

    ///
    /// Returns a reference to the next token if it is a [`GroupCoordinate`], or [None] if
    /// the next token is not a [`GroupCoordinate`].
    ///
    fn peek_group_coordinate(&self, i: &mut usize) -> Option<GroupCoordinate> {
        *i = i.checked_sub(1)?;
        let token = self.tokens.get(*i)?;
        Some(match &token.token {
            Token::Add => GroupCoordinate::SignHigh,
            Token::Minus => match self.tokens.get(i.checked_sub(1)?) {
                Some(SpannedToken {
                    token: Token::Int(value),
                    span,
                }) => {
                    *i -= 1;
                    GroupCoordinate::Number(format!("-{}", value), span.clone())
                }
                _ => GroupCoordinate::SignLow,
            },
            Token::Underscore => GroupCoordinate::Inferred,
            Token::Int(value) => GroupCoordinate::Number(value.clone(), token.span.clone()),
            _ => return None,
        })
    }

    /// Returns `true` if the next token is Function or if it is a Const followed by Function.
    /// Returns `false` otherwise.
    pub fn peek_is_function(&self) -> Result<bool> {
        let first = &self.peek()?.token;
        let next = if self.tokens.len() >= 2 {
            &self.peek_next()?.token
        } else {
            return Ok(false);
        };
        Ok(matches!(
            (first, next),
            (Token::Function | Token::At, _) | (Token::Const, Token::Function)
        ))
    }

    ///
    /// Removes the next two tokens if they are a pair of [`GroupCoordinate`] and returns them,
    /// or [None] if the next token is not a [`GroupCoordinate`].
    ///
    pub fn eat_group_partial(&mut self) -> Option<Result<(GroupCoordinate, GroupCoordinate, Span)>> {
        let mut i = self.tokens.len();
        let start_span = self.tokens.get(i.checked_sub(1)?)?.span.clone();
        let first = self.peek_group_coordinate(&mut i)?;
        i = i.checked_sub(1)?;
        if !matches!(
            self.tokens.get(i),
            Some(SpannedToken {
                token: Token::Comma,
                ..
            })
        ) {
            return None;
        }

        let second = self.peek_group_coordinate(&mut i)?;
        i = i.checked_sub(1)?;
        let right_paren_span = if let Some(SpannedToken {
            token: Token::RightParen,
            span,
        }) = self.tokens.get(i)
        {
            span.clone()
        } else {
            return None;
        };

        i = i.checked_sub(1)?;
        let end_span = if let Some(SpannedToken {
            token: Token::Group,
            span,
        }) = self.tokens.get(i)
        {
            span.clone()
        } else {
            return None;
        };

        self.tokens.drain(i..);
        if let Err(e) = assert_no_whitespace(
            &right_paren_span,
            &end_span,
            &format!("({},{})", first, second),
            "group",
        ) {
            return Some(Err(e));
        }
        Some(Ok((first, second, start_span + end_span)))
    }

    ///
    /// Removes the next token if it is a [`Token::Int(_)`] and returns it, or [None] if
    /// the next token is not a [`Token::Int(_)`] or if the next token does not exist.
    ///
    pub fn eat_int(&mut self) -> Option<(PositiveNumber, Span)> {
        if let Some(SpannedToken {
            token: Token::Int(_), ..
        }) = self.peek_option()
        {
            if let SpannedToken {
                token: Token::Int(value),
                span,
            } = self.bump().unwrap()
            {
                return Some((PositiveNumber { value }, span));
            } else {
                unreachable!("eat_int_ shouldn't produce this")
            }
        }
        None
    }

    ///
    /// Removes the next token if it exists and returns it, or [None] if
    /// the next token  does not exist.
    ///
    pub fn eat_any(&mut self, token: &[Token]) -> Option<SpannedToken> {
        if let Some(SpannedToken { token: inner, .. }) = self.peek_option() {
            if token.iter().any(|x| x == inner) {
                return self.bump();
            }
        }
        None
    }

    ///
    /// Returns the span of the next token if it is equal to the given [`Token`], or error.
    ///
    pub fn expect(&mut self, token: Token) -> Result<Span> {
        if let Some(SpannedToken { token: inner, span }) = self.peek_option() {
            if &token == inner {
                Ok(self.bump().unwrap().span)
            } else {
                Err(ParserError::unexpected(inner, token, span).into())
            }
        } else {
            Err(self.eof())
        }
    }

    ///
    /// Returns the span of the next token if it is equal to one of the given [`Token`]s, or error.
    ///
    pub fn expect_oneof(&mut self, token: &[Token]) -> Result<SpannedToken> {
        if let Some(SpannedToken { token: inner, span }) = self.peek_option() {
            if token.iter().any(|x| x == inner) {
                Ok(self.bump().unwrap())
            } else {
                return Err(ParserError::unexpected(
                    inner,
                    token.iter().map(|x| format!("'{}'", x)).collect::<Vec<_>>().join(", "),
                    span,
                )
                .into());
            }
        } else {
            Err(self.eof())
        }
    }

    ///
    /// Returns the [`Identifier`] of the next token if it is a keyword,
    /// [`Token::Int(_)`], or an [`Identifier`], or error.
    ///
    pub fn expect_loose_identifier(&mut self) -> Result<Identifier> {
        if let Some(token) = self.eat_any(KEYWORD_TOKENS) {
            return Ok(Identifier {
                name: token.token.keyword_to_symbol().unwrap(),
                span: token.span,
            });
        }
        if let Some((int, span)) = self.eat_int() {
            let name = Symbol::intern(&int.value);
            return Ok(Identifier { name, span });
        }
        self.expect_ident()
    }

    /// Returns the [`Identifier`] of the next token if it is an [`Identifier`], or error.
    pub fn expect_ident(&mut self) -> Result<Identifier> {
        if let Some(SpannedToken { token: inner, span }) = self.peek_option() {
            if let Token::Ident(_) = inner {
                if let SpannedToken {
                    token: Token::Ident(name),
                    span,
                } = self.bump().unwrap()
                {
                    Ok(Identifier { name, span })
                } else {
                    unreachable!("expect_ident_ shouldn't produce this")
                }
            } else {
                Err(ParserError::unexpected_str(inner, "ident", span).into())
            }
        } else {
            Err(self.eof())
        }
    }

    ///
    /// Returns the next token if it exists or return end of function.
    ///
    pub fn expect_any(&mut self) -> Result<SpannedToken> {
        if let Some(x) = self.tokens.pop() {
            Ok(x)
        } else {
            Err(self.eof())
        }
    }

    /// Parses a list of `T`s using `inner`
    /// The opening and closing delimiters are `bra` and `ket`,
    /// and elements in the list are separated by `sep`.
    /// When `(list, true)` is returned, `sep` was a terminator.
    pub(super) fn parse_list<T>(
        &mut self,
        open: Token,
        close: Token,
        sep: Token,
        mut inner: impl FnMut(&mut Self) -> Result<Option<T>>,
    ) -> Result<(Vec<T>, bool, Span)> {
        let mut list = Vec::new();
        let mut trailing = false;

        // Parse opening delimiter.
        let open_span = self.expect(open)?;

        while self.peek()?.token != close {
            // Parse the element. We allow inner parser recovery through the `Option`.
            if let Some(elem) = inner(self)? {
                list.push(elem);
            }

            // Parse the separator.
            if self.eat(sep.clone()).is_none() {
                trailing = false;
                break;
            }

            trailing = true;
        }

        // Parse closing delimiter.
        let close_span = self.expect(close)?;

        Ok((list, trailing, open_span + close_span))
    }

    /// Parse a list separated by `,` and delimited by parens.
    pub(super) fn parse_paren_comma_list<T>(
        &mut self,
        f: impl FnMut(&mut Self) -> Result<Option<T>>,
    ) -> Result<(Vec<T>, bool, Span)> {
        self.parse_list(Token::LeftParen, Token::RightParen, Token::Comma, f)
    }

    /// Returns true if the current token is `(`.
    pub(super) fn peek_is_left_par(&self) -> bool {
        matches!(self.peek_option().map(|t| &t.token), Some(Token::LeftParen))
    }
}
