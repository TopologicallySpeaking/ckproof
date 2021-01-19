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

use std::path::{Path, PathBuf};

use pest::iterators::Pair;
use pest::Parser;

use crate::document::structure::{Book, Chapter, Document, Page};

use super::directory::{
    BibliographyBuilder, BlockBuilder, BuilderDirectory, LocalBibliographyBuilder,
};
use super::errors::{BuilderCreationError, ParsingError, ParsingErrorContext};
use super::text::ParagraphBuilder;
use super::{BlockCounter, DocumentParser, Rule};

struct PageBuilder {
    id: String,
    name: String,
    href: String,

    page_path: PathBuf,
    blocks: Option<Vec<BlockBuilder>>,

    local_bibliography: Option<LocalBibliographyBuilder>,
}

impl PageBuilder {
    fn from_pest(pair: Pair<Rule>, chapter_path: &Path, href: &str) -> PageBuilder {
        assert_eq!(pair.as_rule(), Rule::manifest_page);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let href = format!("{}/{}", href, id);

        let string = inner.next().unwrap();
        let string_contents = string.into_inner().next().unwrap();
        let name = string_contents.as_str().to_owned();

        let mut page_path: PathBuf = [chapter_path, Path::new(&id)].iter().collect();
        page_path.set_extension("math");

        PageBuilder {
            id,
            name,
            href,

            page_path,
            blocks: None,

            local_bibliography: None,
        }
    }

    fn load_documents(
        &mut self,
        directory: &mut BuilderDirectory,
        errors: &mut ParsingErrorContext,
        counter: &mut BlockCounter,
    ) {
        assert!(self.blocks.is_none());

        let contents = match std::fs::read_to_string(&self.page_path) {
            Ok(contents) => contents,
            Err(e) => {
                errors.err(e);
                return;
            }
        };
        let pair = match DocumentParser::parse(Rule::document, &contents) {
            Ok(mut parsed) => parsed.next().unwrap(),
            Err(e) => {
                errors.err(e);
                return;
            }
        };

        let blocks = pair
            .into_inner()
            .filter_map(|pair| match pair.as_rule() {
                Rule::EOI => None,

                _ => Some(BlockBuilder::from_pest(
                    pair,
                    directory,
                    counter.next_block(),
                    &self.href,
                )),
            })
            .collect();

        self.blocks = Some(blocks);
    }

    fn build_local_bib(&mut self, directory: &BuilderDirectory) {
        assert!(self.local_bibliography.is_none());

        self.local_bibliography = Some(LocalBibliographyBuilder::new(
            self.blocks.as_ref().unwrap(),
            directory,
        ));
    }

    fn finish(&self) -> Page {
        let id = self.id.clone();
        let name = self.name.clone();
        let href = self.href.clone();
        let blocks = self
            .blocks
            .as_ref()
            .unwrap()
            .iter()
            .map(BlockBuilder::finish)
            .collect();
        let local_bibliography = self
            .local_bibliography
            .as_ref()
            .map(LocalBibliographyBuilder::finish);

        Page::new(id, name, href, blocks, local_bibliography)
    }
}

struct ChapterBuilder {
    id: String,
    name: String,
    href: String,
    tagline: ParagraphBuilder,
    pages: Vec<PageBuilder>,
}

impl ChapterBuilder {
    fn from_pest(pair: Pair<Rule>, book_path: &Path, href: &str) -> ChapterBuilder {
        assert_eq!(pair.as_rule(), Rule::manifest_chapter);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let href = format!("{}/{}", href, id);

        let string = inner.next().unwrap();
        let string_contents = string.into_inner().next().unwrap();
        let name = string_contents.as_str().to_owned();
        let tagline = ParagraphBuilder::from_pest(inner.next().unwrap());

        let chapter_path: PathBuf = [book_path, Path::new(&id)].iter().collect();
        let pages = inner
            .map(|pair| PageBuilder::from_pest(pair, &chapter_path, &href))
            .collect();

        ChapterBuilder {
            id,
            name,
            href,
            tagline,
            pages,
        }
    }

    fn load_documents(
        &mut self,
        directory: &mut BuilderDirectory,
        errors: &mut ParsingErrorContext,
        counter: &mut BlockCounter,
    ) {
        for page in &mut self.pages {
            page.load_documents(directory, errors, counter);
            counter.next_page();
        }
    }

    fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        self.tagline.verify_structure(directory, errors);
    }

    fn build_local_bib(&mut self, directory: &BuilderDirectory) {
        for page in &mut self.pages {
            page.build_local_bib(directory);
        }
    }

    fn finish(&self) -> Chapter {
        let id = self.id.clone();
        let name = self.name.clone();
        let href = self.href.clone();
        let tagline = self.tagline.finish();
        let pages = self.pages.iter().map(PageBuilder::finish).collect();

        Chapter::new(id, name, href, tagline, pages)
    }
}

struct BookBuilder {
    id: String,
    name: String,
    href: String,
    tagline: ParagraphBuilder,
    chapters: Vec<ChapterBuilder>,
}

impl BookBuilder {
    fn from_pest(pair: Pair<Rule>, library_path: &Path) -> BookBuilder {
        assert_eq!(pair.as_rule(), Rule::manifest_book);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let href = format!("/{}", id);

        let string = inner.next().unwrap();
        let string_contents = string.into_inner().next().unwrap();
        let name = string_contents.as_str().to_owned();
        let tagline = ParagraphBuilder::from_pest(inner.next().unwrap());

        let book_path: PathBuf = [library_path, Path::new(&id)].iter().collect();
        let chapters = inner
            .map(|pair| ChapterBuilder::from_pest(pair, &book_path, &href))
            .collect();

        BookBuilder {
            id,
            name,
            href,
            tagline,
            chapters,
        }
    }

    fn load_documents(
        &mut self,
        directory: &mut BuilderDirectory,
        errors: &mut ParsingErrorContext,
        counter: &mut BlockCounter,
    ) {
        for chapter in &mut self.chapters {
            chapter.load_documents(directory, errors, counter);
            counter.next_chapter();
        }
    }

    fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        self.tagline.verify_structure(directory, errors);

        for chapter in &self.chapters {
            chapter.verify_structure(directory, errors);
        }
    }

    fn build_local_bib(&mut self, directory: &BuilderDirectory) {
        // TODO: Figure out what to do about the tagline.

        for chapter in &mut self.chapters {
            chapter.build_local_bib(directory);
        }
    }

    fn finish(&self) -> Book {
        let id = self.id.clone();
        let name = self.name.clone();
        let href = self.href.clone();
        let tagline = self.tagline.finish();
        let chapters = self.chapters.iter().map(ChapterBuilder::finish).collect();

        Book::new(id, name, href, tagline, chapters)
    }
}

pub struct DocumentBuilder {
    library_path: PathBuf,

    books: Vec<BookBuilder>,

    directory: Option<BuilderDirectory>,
}

impl DocumentBuilder {
    pub fn from_lib<P: AsRef<Path>>(
        library_path: P,
    ) -> Result<DocumentBuilder, BuilderCreationError> {
        let manifest_path: PathBuf = [library_path.as_ref(), &Path::new("manifest.math")]
            .iter()
            .collect();
        let contents = std::fs::read_to_string(manifest_path)?;
        let pair = DocumentParser::parse(Rule::manifest, &contents)?
            .next()
            .unwrap();

        let books = pair
            .into_inner()
            .filter(|pair| pair.as_rule() != Rule::EOI)
            .filter_map(|pair| match pair.as_rule() {
                Rule::EOI => None,
                _ => Some(BookBuilder::from_pest(pair, library_path.as_ref())),
            })
            .collect();

        Ok(DocumentBuilder {
            library_path: library_path.as_ref().to_owned(),

            books,

            directory: None,
        })
    }

    fn load_bib(&mut self, bib_path: &Path) -> Result<BibliographyBuilder, ParsingError> {
        let contents = std::fs::read_to_string(bib_path)?;
        let pair = DocumentParser::parse(Rule::bib, &contents)?.next().unwrap();

        Ok(BibliographyBuilder::from_pest(pair))
    }

    fn load_documents(
        &mut self,
        directory: &mut BuilderDirectory,
        errors: &mut ParsingErrorContext,
        counter: &mut BlockCounter,
    ) {
        let bib_path: PathBuf = [self.library_path.as_ref(), Path::new("bib.math")]
            .iter()
            .collect();
        if bib_path.exists() {
            match self.load_bib(&bib_path) {
                Ok(bib) => directory.set_bib(bib),
                Err(_) => todo!(),
            }
        }

        for book in &mut self.books {
            book.load_documents(directory, errors, counter);
            counter.next_book();
        }
    }

    fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        let directory = self.directory.as_ref().unwrap();

        for book in &self.books {
            book.verify_structure(directory, errors);
        }
    }

    fn build_local_bib(&mut self) {
        let directory = self.directory.as_ref().unwrap();

        for book in &mut self.books {
            book.build_local_bib(directory);
        }
    }

    fn finish(&self) -> Document {
        let books = self.books.iter().map(BookBuilder::finish).collect();
        let directory = self.directory.as_ref().unwrap().finish();

        Document::new(books, directory)
    }

    pub fn build(&mut self) -> Result<Document, ParsingErrorContext> {
        let mut errors = ParsingErrorContext::new();
        let mut directory = BuilderDirectory::new();

        self.load_documents(&mut directory, &mut errors, &mut BlockCounter::new());
        if errors.error_found() {
            return Err(errors);
        }

        self.directory = Some(directory);
        let directory = self.directory.as_mut().unwrap();

        directory.build_index(&mut errors);
        if errors.error_found() {
            return Err(errors);
        }

        // If we didn't explicitly drop the mutable reference, the borrow checker will complain.
        let directory = ();

        self.verify_structure(&mut errors);
        if errors.error_found() {
            return Err(errors);
        }

        let directory = self.directory.as_mut().unwrap();

        directory.verify_structure(&mut errors);
        if errors.error_found() {
            return Err(errors);
        }

        // If we didn't explicitly drop the mutable reference, the borrow checker will complain.
        let directory = ();
        self.build_local_bib();

        let directory = self.directory.as_mut().unwrap();

        directory.build_operators(&mut errors);
        if errors.error_found() {
            return Err(errors);
        }

        directory.build_formulas(&mut errors);
        if errors.error_found() {
            Err(errors)
        } else {
            Ok(self.finish())
        }
    }
}
