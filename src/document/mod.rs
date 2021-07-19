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

use std::ops::Index;

use crate::core::directory::CheckableDirectory;
use crate::rendered::DocumentRendered;

pub(crate) mod bibliography;
pub(crate) mod language;
pub(crate) mod structure;
pub(crate) mod system;
pub(crate) mod text;

use bibliography::Bibliography;
use structure::{Block, BlockLocation, Book};

// TODO: Remove.
#[derive(Default)]
pub struct Counter {
    systems: usize,
    types: usize,
    symbols: usize,
    definitions: usize,
    axioms: usize,
    theorems: usize,
    proofs: usize,
}

pub struct Document<'a> {
    books: Vec<Book<'a>>,

    bibliography: Bibliography,
}

impl<'a> Document<'a> {
    pub(crate) fn new(books: Vec<Book<'a>>, bibliography: Bibliography) -> Self {
        Document {
            books,
            bibliography,
        }
    }

    pub fn crosslink(&'a self) {
        for book in &self.books {
            book.crosslink(self);
        }
    }

    // TODO: Remove.
    pub fn checkable(&'a self) -> CheckableDirectory {
        let mut counter = Counter::default();
        for book in &self.books {
            book.count(&mut counter);
        }

        // FIXME: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
        let mut systems = Vec::new();
        let mut types = Vec::new();
        let mut symbols = Vec::new();
        let mut definitions = Vec::new();
        let mut axioms = Vec::new();
        let mut theorems = Vec::new();
        let mut proofs = Vec::new();
        for book in &self.books {
            book.populate_checkable(
                &mut systems,
                &mut types,
                &mut symbols,
                &mut definitions,
                &mut axioms,
                &mut theorems,
                &mut proofs,
            );
        }

        CheckableDirectory::new(
            systems,
            types,
            symbols,
            definitions,
            axioms,
            theorems,
            proofs,
        )
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
