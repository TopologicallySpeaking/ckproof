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
use std::lazy::OnceCell;
use std::path::{Path, PathBuf};

use pest::iterators::Pair;
use pest::Parser;

use crate::document::structure::{Block, BlockLocation, Book, Chapter, Page};

use super::bibliography::{BibliographyBuilderEntry, LocalBibliographyBuilder};
use super::errors::{BookParsingError, ChapterParsingError, ParsingError, ParsingErrorContext};
use super::index::BuilderIndex;
use super::language::{DefinitionBuilder, ReadableBuilder, SymbolBuilder, TypeBuilder};
use super::system::{
    AxiomBuilder, ProofBuilder, SystemBuilder, SystemBuilderChild, TheoremBuilder,
};
use super::text::{
    HeadingBuilder, ListBuilder, ParagraphBuilder, QuoteBuilder, TableBuilder, TextBlockBuilder,
    TodoBuilder,
};
use super::{DocumentParser, Rule};

pub enum BlockBuilder<'a> {
    System(SystemBuilder<'a>),
    Type(TypeBuilder<'a>),
    Symbol(SymbolBuilder<'a>),
    Definition(DefinitionBuilder<'a>),
    Axiom(AxiomBuilder<'a>),
    Theorem(TheoremBuilder<'a>),
    Proof(ProofBuilder<'a>),

    List(ListBuilder<'a>),
    Table(TableBuilder<'a>),
    Quote(QuoteBuilder<'a>),
    Todo(TodoBuilder<'a>),
    Heading(HeadingBuilder),
    Text(TextBlockBuilder<'a>),
}

impl<'a> BlockBuilder<'a> {
    fn from_pest(path: &Path, pair: Pair<Rule>, location: &mut BlockLocation) -> Self {
        let location = location.next_block();

        match pair.as_rule() {
            Rule::system_block => Self::System(SystemBuilder::from_pest(path, pair, location)),
            Rule::type_block => Self::Type(TypeBuilder::from_pest(path, pair, location)),
            Rule::symbol_block => Self::Symbol(SymbolBuilder::from_pest(path, pair, location)),
            Rule::definition_block => {
                Self::Definition(DefinitionBuilder::from_pest(path, pair, location))
            }
            Rule::axiom_block => Self::Axiom(AxiomBuilder::from_pest(path, pair, location)),
            Rule::theorem_block => Self::Theorem(TheoremBuilder::from_pest(path, pair, location)),
            Rule::proof_block => Self::Proof(ProofBuilder::from_pest(path, pair, location)),

            Rule::ul_block => Self::List(ListBuilder::from_pest(path, pair, false, location)),
            Rule::ol_block => Self::List(ListBuilder::from_pest(path, pair, true, location)),

            Rule::table_block => Self::Table(TableBuilder::from_pest(path, pair, location)),
            Rule::quote_block => Self::Quote(QuoteBuilder::from_pest(pair, location)),
            Rule::todo_block => Self::Todo(TodoBuilder::from_pest(path, pair)),
            Rule::heading_block => Self::Heading(HeadingBuilder::from_pest(pair)),
            Rule::text_block => Self::Text(TextBlockBuilder::from_pest(path, pair, location)),

            _ => unreachable!(),
        }
    }

    fn build_index(&'a self, index: &mut BuilderIndex<'a>, errors: &mut ParsingErrorContext<'a>) {
        match self {
            Self::System(system_ref) => index.add_system(system_ref, errors),
            Self::Type(type_ref) => {
                index.add_system_child(SystemBuilderChild::Type(type_ref), errors)
            }
            Self::Symbol(symbol_ref) => {
                index.add_system_child(SystemBuilderChild::Symbol(symbol_ref), errors)
            }
            Self::Definition(definition_ref) => {
                index.add_system_child(SystemBuilderChild::Definition(definition_ref), errors)
            }
            Self::Axiom(axiom_ref) => {
                index.add_system_child(SystemBuilderChild::Axiom(axiom_ref), errors)
            }
            Self::Theorem(theorem_ref) => {
                index.add_system_child(SystemBuilderChild::Theorem(theorem_ref), errors)
            }

            _ => {}
        }
    }

    fn verify_structure(&'a self, index: &BuilderIndex<'a>, errors: &mut ParsingErrorContext<'a>) {
        match self {
            Self::System(system_ref) => system_ref.verify_structure(index, errors),
            Self::Type(type_ref) => type_ref.verify_structure(index, errors),
            Self::Symbol(symbol_ref) => symbol_ref.verify_structure(index, errors),
            Self::Definition(definition_ref) => definition_ref.verify_structure(index, errors),
            Self::Axiom(axiom_ref) => axiom_ref.verify_structure(index, errors),
            Self::Theorem(theorem_ref) => theorem_ref.verify_structure(index, errors),
            Self::Proof(proof_ref) => proof_ref.verify_structure(index, errors),

            Self::List(list_ref) => list_ref.verify_structure(index, errors),
            Self::Table(table_ref) => table_ref.verify_structure(index, errors),
            Self::Quote(quote_ref) => quote_ref.verify_structure(index, errors),
            Self::Todo(todo_ref) => todo_ref.verify_structure(index, errors),
            Self::Heading(heading_ref) => heading_ref.verify_structure(errors),
            Self::Text(text_ref) => {
                text_ref.verify_structure(index, errors, |e| {
                    ParsingError::TextError(text_ref.text(), e)
                });
            }
        }
    }

    fn build_operators(
        &'a self,
        index: &mut BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self {
            Self::Symbol(symbol_ref) => {
                if let Some(read_signature) = symbol_ref.read_signature() {
                    index.add_operator(read_signature, ReadableBuilder::Symbol(symbol_ref), errors);
                }
            }
            Self::Definition(definition_ref) => {
                if let Some(read_signature) = definition_ref.read_signature() {
                    index.add_operator(
                        read_signature,
                        ReadableBuilder::Definition(definition_ref),
                        errors,
                    );
                }
            }

            _ => {}
        }
    }

    pub fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        match self {
            Self::System(system_ref) => system_ref.bib_refs(),
            Self::Type(type_ref) => type_ref.bib_refs(),
            Self::Symbol(symbol_ref) => symbol_ref.bib_refs(),
            Self::Definition(definition_ref) => definition_ref.bib_refs(),
            Self::Axiom(axiom_ref) => axiom_ref.bib_refs(),
            Self::Theorem(theorem_ref) => theorem_ref.bib_refs(),
            Self::Proof(proof_ref) => proof_ref.bib_refs(),

            Self::List(list_ref) => list_ref.bib_refs(),
            Self::Table(table_ref) => table_ref.bib_refs(),
            Self::Quote(quote_ref) => quote_ref.bib_refs(),
            Self::Todo(todo_ref) => todo_ref.bib_refs(),
            Self::Heading(heading_ref) => heading_ref.bib_refs(),
            Self::Text(text_ref) => text_ref.bib_refs(),
        }
    }

    pub fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        match self {
            Self::System(system_ref) => system_ref.set_local_bib_refs(index),
            Self::Type(type_ref) => type_ref.set_local_bib_refs(index),
            Self::Symbol(symbol_ref) => symbol_ref.set_local_bib_refs(index),
            Self::Definition(definition_ref) => definition_ref.set_local_bib_refs(index),
            Self::Axiom(axiom_ref) => axiom_ref.set_local_bib_refs(index),
            Self::Theorem(theorem_ref) => theorem_ref.set_local_bib_refs(index),
            Self::Proof(proof_ref) => proof_ref.set_local_bib_refs(index),

            Self::List(list_ref) => list_ref.set_local_bib_refs(index),
            Self::Table(table_ref) => table_ref.set_local_bib_refs(index),
            Self::Quote(quote_ref) => quote_ref.set_local_bib_refs(index),
            Self::Todo(todo_ref) => todo_ref.set_local_bib_refs(index),
            Self::Text(text_ref) => text_ref.set_local_bib_refs(index),

            _ => {}
        }
    }

    fn build_formulas(&'a self, index: &BuilderIndex<'a>, errors: &mut ParsingErrorContext<'a>) {
        match self {
            Self::Definition(definition_ref) => definition_ref.build_formulas(index, errors),
            Self::Axiom(axiom_ref) => axiom_ref.build_formulas(index, errors),
            Self::Theorem(theorem_ref) => theorem_ref.build_formulas(index, errors),
            Self::Proof(proof_ref) => proof_ref.build_formulas(index, errors),

            _ => {}
        }
    }

    // TODO: Remove.
    fn set_href(&self, book_id: &str, chapter_id: &str, page_id: &str) {
        match self {
            Self::System(system_ref) => system_ref.set_href(book_id, chapter_id, page_id),
            Self::Symbol(symbol_ref) => symbol_ref.set_href(book_id, chapter_id, page_id),
            Self::Definition(definition_ref) => {
                definition_ref.set_href(book_id, chapter_id, page_id)
            }
            Self::Axiom(axiom_ref) => axiom_ref.set_href(book_id, chapter_id, page_id),
            Self::Theorem(theorem_ref) => theorem_ref.set_href(book_id, chapter_id, page_id),
            Self::Proof(proof_ref) => proof_ref.set_href(book_id, chapter_id, page_id),

            _ => {}
        }
    }

    fn finish<'b>(&self) -> Block<'b> {
        match self {
            Self::System(system_ref) => Block::System(system_ref.finish()),
            Self::Type(type_ref) => Block::Type(type_ref.finish()),
            Self::Symbol(symbol_ref) => Block::Symbol(symbol_ref.finish()),
            Self::Definition(definition_ref) => Block::Definition(definition_ref.finish()),
            Self::Axiom(axiom_ref) => Block::Axiom(axiom_ref.finish()),
            Self::Theorem(theorem_ref) => Block::Theorem(theorem_ref.finish()),
            Self::Proof(proof_ref) => Block::Proof(proof_ref.finish()),

            Self::List(list_ref) => Block::List(list_ref.finish()),
            Self::Table(table_ref) => Block::Table(table_ref.finish()),
            Self::Quote(quote_ref) => Block::Quote(quote_ref.finish()),
            Self::Todo(todo_ref) => Block::Todo(todo_ref.finish()),
            Self::Heading(heading_ref) => Block::Heading(heading_ref.finish()),
            Self::Text(text_ref) => Block::Text(text_ref.finish()),
        }
    }
}

pub struct PageBuilder<'a> {
    id: String,
    name: String,

    blocks: Vec<BlockBuilder<'a>>,
    local_bibliography: OnceCell<LocalBibliographyBuilder<'a>>,

    // TODO: Remove.
    href: OnceCell<String>,
}

impl<'a> PageBuilder<'a> {
    fn from_pest(
        pair: Pair<Rule>,
        library_path: &Path,
        book_id: &str,
        chapter_id: &str,
        location: &mut BlockLocation,
        errors: &mut ParsingErrorContext,
    ) -> Self {
        assert_eq!(pair.as_rule(), Rule::manifest_page);
        let mut inner = pair.into_inner();

        let id = inner.next().unwrap().as_str().to_owned();

        let string = inner.next().unwrap();
        let string_contents = string.into_inner().next().unwrap();
        let name = string_contents.as_str().to_owned();

        let page_path: PathBuf = [
            library_path,
            Path::new(book_id),
            Path::new(chapter_id),
            Path::new(&format!("{}.math", id)),
        ]
        .iter()
        .collect();

        let contents = match std::fs::read_to_string(&page_path) {
            Ok(contents) => contents,
            Err(e) => {
                errors.err(e);
                return PageBuilder {
                    id,
                    name,

                    blocks: Vec::new(),
                    local_bibliography: OnceCell::new(),

                    href: OnceCell::new(),
                };
            }
        };
        let pair = match DocumentParser::parse(Rule::document, &contents) {
            Ok(mut parsed) => parsed.next().unwrap(),
            Err(e) => {
                errors.err(e);
                return PageBuilder {
                    id,
                    name,

                    blocks: Vec::new(),
                    local_bibliography: OnceCell::new(),

                    href: OnceCell::new(),
                };
            }
        };

        let blocks = pair
            .into_inner()
            .filter_map(|pair| match pair.as_rule() {
                Rule::EOI => None,

                _ => Some(BlockBuilder::from_pest(&page_path, pair, location)),
            })
            .collect();

        location.next_page();

        PageBuilder {
            id,
            name,

            blocks,
            local_bibliography: OnceCell::new(),

            href: OnceCell::new(),
        }
    }

    fn build_index(&'a self, index: &mut BuilderIndex<'a>, errors: &mut ParsingErrorContext<'a>) {
        for block in &self.blocks {
            block.build_index(index, errors);
        }
    }

    fn verify_structure(&'a self, index: &BuilderIndex<'a>, errors: &mut ParsingErrorContext<'a>) {
        for block in &self.blocks {
            block.verify_structure(index, errors);
        }
    }

    fn build_local_bib(&'a self) {
        self.local_bibliography
            .set(LocalBibliographyBuilder::new(&self.blocks))
            .unwrap();
    }

    fn build_operators(
        &'a self,
        index: &mut BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        for block in &self.blocks {
            block.build_operators(index, errors);
        }
    }

    fn build_formulas(&'a self, index: &BuilderIndex<'a>, errors: &mut ParsingErrorContext<'a>) {
        for block in &self.blocks {
            block.build_formulas(index, errors);
        }
    }

    // TODO: Remove.
    fn set_href(&self, book_id: &str, chapter_id: &str) {
        let id = &self.id;
        let href = format!("/{}/{}/{}", book_id, chapter_id, id);
        self.href.set(href).unwrap();

        for block in &self.blocks {
            block.set_href(book_id, chapter_id, id);
        }
    }

    fn finish<'b>(&self) -> Page<'b> {
        let id = self.id.clone();
        let name = self.name.clone();

        let blocks = self.blocks.iter().map(BlockBuilder::finish).collect();

        let local_bibliography = self.local_bibliography.get().unwrap().finish();

        let href = self.href.get().unwrap().clone();

        Page::new(id, name, blocks, local_bibliography, href)
    }
}

pub struct ChapterBuilder<'a> {
    id: String,
    name: String,
    tagline: ParagraphBuilder<'a>,

    pages: Vec<PageBuilder<'a>>,

    // TODO: Remove.
    href: OnceCell<String>,
}

impl<'a> ChapterBuilder<'a> {
    fn from_pest(
        pair: Pair<Rule>,
        library_path: &Path,
        book_id: &str,
        location: &mut BlockLocation,
        errors: &mut ParsingErrorContext,
    ) -> Self {
        assert_eq!(pair.as_rule(), Rule::manifest_chapter);
        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();

        let string = inner.next().unwrap();
        let string_contents = string.into_inner().next().unwrap();
        let name = string_contents.as_str().to_owned();

        let path: PathBuf = [library_path, Path::new("manifest.math")].iter().collect();
        let tagline = ParagraphBuilder::from_pest(&path, inner.next().unwrap());

        let pages = inner
            .map(|pair| PageBuilder::from_pest(pair, library_path, book_id, &id, location, errors))
            .collect();

        location.next_chapter();

        ChapterBuilder {
            id,
            name,
            tagline,

            pages,

            href: OnceCell::new(),
        }
    }

    fn build_index(&'a self, index: &mut BuilderIndex<'a>, errors: &mut ParsingErrorContext<'a>) {
        for page in &self.pages {
            page.build_index(index, errors);
        }
    }

    fn verify_structure(&'a self, index: &BuilderIndex<'a>, errors: &mut ParsingErrorContext<'a>) {
        self.tagline.verify_structure(index, errors, |e| {
            ParsingError::ChapterError(self, ChapterParsingError::TaglineError(e))
        });

        for page in &self.pages {
            page.verify_structure(index, errors);
        }
    }

    fn build_local_bib(&'a self) {
        for page in &self.pages {
            page.build_local_bib();
        }
    }

    fn build_operators(
        &'a self,
        index: &mut BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        for page in &self.pages {
            page.build_operators(index, errors);
        }
    }

    fn build_formulas(&'a self, index: &BuilderIndex<'a>, errors: &mut ParsingErrorContext<'a>) {
        for page in &self.pages {
            page.build_formulas(index, errors);
        }
    }

    // TODO: Remove.
    fn set_href(&self, book_id: &str) {
        let id = &self.id;
        let href = format!("/{}/{}", book_id, id);
        self.href.set(href).unwrap();

        for page in &self.pages {
            page.set_href(book_id, id);
        }
    }

    fn finish<'b>(&self) -> Chapter<'b> {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.finish();

        let pages = self.pages.iter().map(PageBuilder::finish).collect();

        let href = self.href.get().unwrap().clone();

        Chapter::new(id, name, tagline, pages, href)
    }
}

impl<'a> std::fmt::Debug for ChapterBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Chapter").field(&self.id).finish()
    }
}

pub struct BookBuilder<'a> {
    id: String,
    name: String,
    tagline: ParagraphBuilder<'a>,

    chapters: Vec<ChapterBuilder<'a>>,

    // TODO: Remove.
    href: OnceCell<String>,
}

impl<'a> BookBuilder<'a> {
    pub fn from_pest(
        pair: Pair<Rule>,
        library_path: &Path,
        location: &mut BlockLocation,
        errors: &mut ParsingErrorContext,
    ) -> Self {
        assert_eq!(pair.as_rule(), Rule::manifest_book);
        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();

        let string = inner.next().unwrap();
        let string_contents = string.into_inner().next().unwrap();
        let name = string_contents.as_str().to_owned();

        let path: PathBuf = [library_path, Path::new("manifest.math")].iter().collect();
        let tagline = ParagraphBuilder::from_pest(&path, inner.next().unwrap());

        let chapters = inner
            .map(|pair| ChapterBuilder::from_pest(pair, library_path, &id, location, errors))
            .collect();

        location.next_book();

        BookBuilder {
            id,
            name,
            tagline,

            chapters,

            href: OnceCell::new(),
        }
    }

    pub fn build_index(
        &'a self,
        index: &mut BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        for chapter in &self.chapters {
            chapter.build_index(index, errors);
        }
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.tagline.verify_structure(index, errors, |e| {
            ParsingError::BookError(self, BookParsingError::TaglineError(e))
        });

        for chapter in &self.chapters {
            chapter.verify_structure(index, errors);
        }
    }

    pub fn build_local_bib(&'a self) {
        // TODO: Bibliography references in the book's tagline.

        for chapter in &self.chapters {
            chapter.build_local_bib();
        }
    }

    pub fn build_operators(
        &'a self,
        index: &mut BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        for chapter in &self.chapters {
            chapter.build_operators(index, errors);
        }
    }

    pub fn build_formulas(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        for chapter in &self.chapters {
            chapter.build_formulas(index, errors);
        }
    }

    // TODO: Remove.
    pub fn set_href(&self) {
        let id = &self.id;
        let href = format!("/{}", id);
        self.href.set(href).unwrap();

        for chapter in &self.chapters {
            chapter.set_href(id)
        }
    }

    pub fn finish<'b>(&self) -> Book<'b> {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.finish();

        let chapters = self.chapters.iter().map(ChapterBuilder::finish).collect();

        let href = self.href.get().unwrap().clone();

        Book::new(id, name, tagline, chapters, href)
    }
}

impl<'a> std::fmt::Debug for BookBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Book").field(&self.id).finish()
    }
}
