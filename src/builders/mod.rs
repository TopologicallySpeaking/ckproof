// Copyright 2020 Alexander Isaacson
//
// This file is part of ckproof.
//
// Ckproof is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// Ckproof is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with ckproof.  If not, see
// <https://www.gnu.org/licenses/>.

pub mod errors;

mod deduction;
mod directory;
mod language;
mod structure;
mod text;

pub use structure::DocumentBuilder;

mod hidden {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "builders/syntax.pest"]
    pub struct DocumentParser;
}
pub(super) use hidden::{DocumentParser, Rule};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockLocation {
    book: usize,
    chapter: usize,
    page: usize,
    block: usize,
}

struct BlockCounter {
    book: usize,
    chapter: usize,
    page: usize,
    block: usize,
}

impl BlockCounter {
    fn new() -> BlockCounter {
        BlockCounter {
            book: 0,
            chapter: 0,
            page: 0,
            block: 0,
        }
    }

    fn finish(&self) -> BlockLocation {
        BlockLocation {
            book: self.book,
            chapter: self.chapter,
            page: self.page,
            block: self.block,
        }
    }

    fn next_block(&mut self) -> BlockLocation {
        let ret = self.finish();
        self.block += 1;
        ret
    }

    fn next_page(&mut self) {
        self.page += 1;
        self.block = 0;
    }

    fn next_chapter(&mut self) {
        self.chapter += 1;
        self.page = 0;
        self.block = 0;
    }

    fn next_book(&mut self) {
        self.book += 1;
        self.chapter = 0;
        self.page = 0;
        self.block = 0;
    }
}
