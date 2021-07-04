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

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use pest::iterators::Pair;
use pest::Parser;

use crate::document::bibliography::{Bibliography, LocalBibliography, LocalBibliographyEntry};
use crate::document::text::RawCitation;

use super::errors::{BibliographyParsingError, ParsingError, ParsingErrorContext};
use super::index::BuilderIndex;
use super::structure::BlockBuilder;
use super::text::RawCitationBuilder;
use super::{DocumentParser, Rule};

#[derive(Debug)]
pub struct BibliographyBuilderEntry {
    id: String,

    raw_citation: RawCitationBuilder,

    index: usize,
}

impl BibliographyBuilderEntry {
    fn from_pest(pair: Pair<Rule>, index: usize) -> Self {
        assert_eq!(pair.as_rule(), Rule::bib_entry);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let raw_citation = RawCitationBuilder::from_pest_entries(inner);

        BibliographyBuilderEntry {
            id,
            raw_citation,

            index,
        }
    }

    fn verify_structure<'a>(&'a self, errors: &mut ParsingErrorContext<'a>) {
        self.raw_citation.verify_structure(errors, |e| {
            ParsingError::BibliographyError(self, BibliographyParsingError::RawCitationError(e))
        });
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    fn finish(&self) -> RawCitation {
        self.raw_citation.finish()
    }
}

impl PartialEq for BibliographyBuilderEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for BibliographyBuilderEntry {}

impl Hash for BibliographyBuilderEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct BibliographyBuilder {
    entries: Vec<BibliographyBuilderEntry>,
}

impl BibliographyBuilder {
    pub fn from_lib(library_path: &Path, errors: &mut ParsingErrorContext) -> Option<Self> {
        let bib_path: PathBuf = [library_path, Path::new("bib.math")].iter().collect();
        if !bib_path.exists() {
            return None;
        }

        let contents = match std::fs::read_to_string(bib_path) {
            Ok(contents) => contents,
            Err(e) => {
                errors.err(e);
                return None;
            }
        };

        let pair = match DocumentParser::parse(Rule::bib, &contents) {
            Ok(mut pair) => pair.next().unwrap(),
            Err(e) => {
                errors.err(e);
                return None;
            }
        };

        let entries = pair
            .into_inner()
            .enumerate()
            .filter_map(|(i, pair)| match pair.as_rule() {
                Rule::EOI => None,
                _ => Some(BibliographyBuilderEntry::from_pest(pair, i)),
            })
            .collect();

        Some(BibliographyBuilder { entries })
    }

    pub fn build_index<'a>(
        &'a self,
        index: &mut BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        for entry in &self.entries {
            index.add_bib_ref(entry, errors);
        }
    }

    pub fn verify_structure<'a>(&'a self, errors: &mut ParsingErrorContext<'a>) {
        for entry in &self.entries {
            entry.verify_structure(errors);
        }
    }

    pub fn finish(&self) -> Bibliography {
        let entries = self
            .entries
            .iter()
            .map(BibliographyBuilderEntry::finish)
            .collect();

        Bibliography::new(entries)
    }
}

#[derive(Debug)]
pub struct LocalBibliographyBuilder<'a> {
    entries: Vec<&'a BibliographyBuilderEntry>,
}

impl<'a> LocalBibliographyBuilder<'a> {
    pub fn new(blocks: &'a [BlockBuilder<'a>]) -> Self {
        let mut index = HashMap::new();
        let mut entries = Vec::new();

        let bib_refs = blocks.iter().flat_map(BlockBuilder::bib_refs);
        for bib_ref in bib_refs {
            index.entry(bib_ref).or_insert_with(|| {
                entries.push(bib_ref);
                entries.len() - 1
            });
        }

        for block in blocks {
            block.set_local_bib_refs(&index);
        }

        LocalBibliographyBuilder { entries }
    }

    pub fn finish<'b>(&self) -> LocalBibliography<'b> {
        let entries = self
            .entries
            .iter()
            .map(|entry| LocalBibliographyEntry::new(entry.index))
            .collect();

        LocalBibliography::new(entries)
    }
}
