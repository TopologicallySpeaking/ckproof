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

use crate::rendered::MlaRendered;

use super::text::RawCitation;
use super::Document;

pub struct Bibliography {
    entries: Vec<RawCitation>,
}

impl Bibliography {
    pub fn empty() -> Self {
        Bibliography {
            entries: Vec::new(),
        }
    }

    pub fn new(entries: Vec<RawCitation>) -> Self {
        Bibliography { entries }
    }
}

impl Index<usize> for Bibliography {
    type Output = RawCitation;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

pub struct LocalBibliographyEntry<'a> {
    index: usize,
    entry: OnceCell<&'a RawCitation>,
}

impl<'a> LocalBibliographyEntry<'a> {
    pub fn new(index: usize) -> Self {
        LocalBibliographyEntry {
            index,
            entry: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.entry.set(&document.bibliography[self.index]).unwrap();
    }

    fn render(&self) -> MlaRendered {
        let entry = self.entry.get().unwrap();
        entry.render()
    }
}

pub struct LocalBibliography<'a> {
    entries: Vec<LocalBibliographyEntry<'a>>,
}

impl<'a> LocalBibliography<'a> {
    pub fn new(entries: Vec<LocalBibliographyEntry<'a>>) -> Self {
        LocalBibliography { entries }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        for entry in &self.entries {
            entry.crosslink(document);
        }
    }

    pub fn render(&self) -> Vec<MlaRendered> {
        self.entries
            .iter()
            .map(LocalBibliographyEntry::render)
            .collect()
    }
}
