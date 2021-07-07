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
use std::path::{Path, PathBuf};

use pest::Parser;

use crate::document::bibliography::Bibliography;
use crate::document::structure::BlockLocation;
use crate::document::Document;

pub mod errors;

mod bibliography;
mod index;
mod language;
mod structure;
mod system;
mod text;

mod hidden {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "builders/syntax.pest"]
    pub struct DocumentParser;
}
use hidden::{DocumentParser, Rule};

use bibliography::BibliographyBuilder;
use errors::ParsingErrorContext;
use index::BuilderIndex;
use structure::BookBuilder;

pub struct ManifestBuilder<'a> {
    library_path: PathBuf,

    // TODO: This should not be optional. If there is no bibliography, just make it empty.
    bibliography: OnceCell<Option<BibliographyBuilder>>,
    books: OnceCell<Vec<BookBuilder<'a>>>,

    errors: OnceCell<ParsingErrorContext<'a>>,
}

impl<'a> ManifestBuilder<'a> {
    fn process_manifest(&self, errors: &mut ParsingErrorContext) -> Vec<BookBuilder<'a>> {
        let manifest_path: PathBuf = [&self.library_path, Path::new("manifest.math")]
            .iter()
            .collect();
        let contents = match std::fs::read_to_string(manifest_path) {
            Ok(contents) => contents,
            Err(e) => {
                errors.err(e);
                return Vec::new();
            }
        };
        let manifest_pair = match DocumentParser::parse(Rule::manifest, &contents) {
            Ok(mut parsed) => parsed.next().unwrap(),
            Err(e) => {
                errors.err(e);
                return Vec::new();
            }
        };

        let mut location = BlockLocation::new();
        manifest_pair
            .into_inner()
            .filter_map(|pair| match pair.as_rule() {
                Rule::EOI => None,
                _ => Some(BookBuilder::from_pest(
                    pair,
                    &self.library_path,
                    &mut location,
                    errors,
                )),
            })
            .collect()
    }

    pub fn from_lib<P: AsRef<Path>>(library_path: P) -> Self {
        ManifestBuilder {
            library_path: library_path.as_ref().to_owned(),

            bibliography: OnceCell::new(),
            books: OnceCell::new(),

            errors: OnceCell::new(),
        }
    }

    fn finish<'b>(&self) -> Document<'b> {
        let books = self.books.get().unwrap();

        // TODO: Remove.
        for book in books {
            book.set_href();
        }

        let books = books.iter().map(BookBuilder::finish).collect();
        let bibliography = self
            .bibliography
            .get()
            .unwrap()
            .as_ref()
            .map(BibliographyBuilder::finish)
            .unwrap_or(Bibliography::empty());

        Document::new(books, bibliography)
    }

    pub fn build<'b>(&'a self) -> Result<Document<'b>, &ParsingErrorContext<'a>> {
        let errors = self.errors.get_or_init(|| {
            let mut errors = ParsingErrorContext::new();
            let bibliography = self
                .bibliography
                .get_or_init(|| BibliographyBuilder::from_lib(&self.library_path, &mut errors));
            let books = self
                .books
                .get_or_init(|| self.process_manifest(&mut errors));

            if errors.error_found() {
                return errors;
            }

            let mut index = BuilderIndex::new();

            for book in books {
                book.build_index(&mut index, &mut errors);
            }
            if let Some(bibliography) = bibliography {
                bibliography.build_index(&mut index, &mut errors);
            }

            if errors.error_found() {
                return errors;
            }

            for book in books {
                book.verify_structure(&index, &mut errors);
            }
            if let Some(bibliography) = bibliography {
                bibliography.verify_structure(&mut errors);
            }

            if errors.error_found() {
                return errors;
            }

            for book in books {
                book.build_local_bib();
            }

            for book in books {
                book.build_operators(&mut index, &mut errors);
            }
            if errors.error_found() {
                return errors;
            }

            for book in books {
                book.build_formulas(&index, &mut errors);
            }
            errors
        });

        if errors.error_found() {
            Err(errors)
        } else {
            Ok(self.finish())
        }
    }
}
