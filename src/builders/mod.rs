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

use crate::document::directory::BlockDirectory;
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

// TODO: Remove.
#[derive(Default)]
pub struct BlockCounter {
    systems: usize,
    types: usize,
    symbols: usize,
    definitions: usize,
    axioms: usize,
    theorems: usize,
    proofs: usize,

    tables: usize,
    quotes: usize,
    todos: usize,
    headings: usize,
    texts: usize,
}

impl BlockCounter {
    fn system(&mut self) -> usize {
        let ret = self.systems;
        self.systems += 1;
        ret
    }

    fn ty(&mut self) -> usize {
        let ret = self.types;
        self.types += 1;
        ret
    }

    fn symbol(&mut self) -> usize {
        let ret = self.symbols;
        self.symbols += 1;
        ret
    }

    fn definition(&mut self) -> usize {
        let ret = self.definitions;
        self.definitions += 1;
        ret
    }

    fn axiom(&mut self) -> usize {
        let ret = self.axioms;
        self.axioms += 1;
        ret
    }

    fn theorem(&mut self) -> usize {
        let ret = self.theorems;
        self.theorems += 1;
        ret
    }

    fn proof(&mut self) -> usize {
        let ret = self.proofs;
        self.proofs += 1;
        ret
    }

    fn table(&mut self) -> usize {
        let ret = self.tables;
        self.tables += 1;
        ret
    }

    fn quote(&mut self) -> usize {
        let ret = self.quotes;
        self.quotes += 1;
        ret
    }

    fn todo(&mut self) -> usize {
        let ret = self.todos;
        self.todos += 1;
        ret
    }

    fn heading(&mut self) -> usize {
        let ret = self.headings;
        self.headings += 1;
        ret
    }

    fn text(&mut self) -> usize {
        let ret = self.texts;
        self.texts += 1;
        ret
    }
}

pub struct ManifestBuilder<'a> {
    library_path: PathBuf,

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

        let mut serial = 0;
        manifest_pair
            .into_inner()
            .filter_map(|pair| match pair.as_rule() {
                Rule::EOI => None,
                _ => Some(BookBuilder::from_pest(
                    pair,
                    &self.library_path,
                    &mut serial,
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

    fn finish(&'a self) -> Document {
        assert!(!self.errors.get().unwrap().error_found());
        let mut counter = BlockCounter::default();

        let books = self.books.get().unwrap();
        for book in books {
            book.count(&mut counter);
        }

        // FIXME: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
        let mut systems = Vec::new();
        let mut types = Vec::new();
        let mut symbols = Vec::new();
        let mut definitions = Vec::new();
        let mut axioms = Vec::new();
        let mut theorems = Vec::new();
        let mut proofs = Vec::new();
        let mut tables = Vec::new();
        let mut quotes = Vec::new();
        let mut todos = Vec::new();
        let mut headings = Vec::new();
        let mut texts = Vec::new();

        for book in books {
            book.build_directory(
                &mut systems,
                &mut types,
                &mut symbols,
                &mut definitions,
                &mut axioms,
                &mut theorems,
                &mut proofs,
                &mut tables,
                &mut quotes,
                &mut todos,
                &mut headings,
                &mut texts,
            );
        }

        let bibliography = self
            .bibliography
            .get()
            .unwrap()
            .as_ref()
            .map(BibliographyBuilder::finish);

        let directory = BlockDirectory::new(
            systems,
            types,
            symbols,
            definitions,
            axioms,
            theorems,
            proofs,
            tables,
            quotes,
            headings,
            todos,
            texts,
            bibliography,
        );

        let books = books.iter().map(BookBuilder::finish).collect();

        Document::new(books, directory)
    }

    pub fn build(&'a self) -> Result<Document, &ParsingErrorContext<'a>> {
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
