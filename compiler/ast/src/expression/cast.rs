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

use crate::Type;

use super::*;

/// A cast expression `e as U`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastExpression {
    /// The expression `e` of a type `T` that is being cast to `U`.
    pub inner: Box<Expression>,
    /// The type `U` to cast `e` to.
    pub target_type: Type,
    /// Span for the entire expression `e as U` to.
    pub span: Span,
}

impl fmt::Display for CastExpression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} as {}", self.inner, self.target_type)
    }
}

impl Node for CastExpression {
    fn span(&self) -> &Span {
        &self.span
    }

    fn set_span(&mut self, span: Span) {
        self.span = span;
    }
}
