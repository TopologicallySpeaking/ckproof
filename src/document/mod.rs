// Copyright 2020,2021 Alexander Isaacson
//
// This file is part of ckproof.
//
// Ckproof is free software: you can redistribute it and/or modify it under the terms of the GNU
// Affero General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// Ckproof is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License along with ckproof. If
// not, see <https://www.gnu.org/licenses/>.

use std::lazy::OnceCell;
use std::ops::Index;

use crate::rendered::DocumentRendered;

pub(crate) mod bibliography;
pub(crate) mod language;
pub(crate) mod structure;
pub(crate) mod system;
pub(crate) mod text;

pub mod errors;

use bibliography::Bibliography;
use errors::DocumentCheckingErrorContext;
use structure::{Block, BlockLocation, Book};

pub struct Document<'a> {
    books: Vec<Book<'a>>,
    bibliography: Bibliography,

    errors: OnceCell<DocumentCheckingErrorContext<'a>>,
}

impl<'a> Document<'a> {
    pub(crate) fn new(books: Vec<Book<'a>>, bibliography: Bibliography) -> Self {
        Document {
            books,
            bibliography,

            errors: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self) {
        for book in &self.books {
            book.crosslink(self);
        }
    }

    pub fn check(&'a self) -> Result<(), &DocumentCheckingErrorContext> {
        let errors = self.errors.get_or_init(|| {
            let mut errors = DocumentCheckingErrorContext::new();

            for book in &self.books {
                book.verify();
            }

            for book in &self.books {
                book.check(&mut errors);
            }

            errors
        });

        if errors.error_found() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    // TODO: Remove.
    pub fn render(&self) -> DocumentRendered {
        let books = self
            .books
            .iter()
            .enumerate()
            .map(|(i, book)| book.render(i))
            .collect();

        DocumentRendered::new(books)
    }
}

impl<'a> Index<BlockLocation> for Document<'a> {
    type Output = Block<'a>;

    fn index(&self, location: BlockLocation) -> &Self::Output {
        &self.books[location.book()][location]
    }
}
