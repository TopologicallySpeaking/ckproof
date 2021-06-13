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

use crate::document::deduction::{AxiomBlock, ProofBlock, TheoremBlock};
use crate::document::directory::Block;
use crate::document::language::{DefinitionBlock, SymbolBlock, SystemBlock, TypeBlock};
use crate::document::structure::{Book, Chapter, Page};
use crate::document::text::{
    HeadingBlock, ListBlock, QuoteBlock, TableBlock, TextBlock, TodoBlock,
};

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
use super::{BlockCounter, DocumentParser, Rule};

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
    fn from_pest(pair: Pair<Rule>, serial: &mut usize) -> Self {
        let curr_serial = *serial;
        *serial += 1;

        match pair.as_rule() {
            Rule::system_block => Self::System(SystemBuilder::from_pest(pair)),
            Rule::type_block => Self::Type(TypeBuilder::from_pest(pair, curr_serial)),
            Rule::symbol_block => Self::Symbol(SymbolBuilder::from_pest(pair, curr_serial)),
            Rule::definition_block => {
                Self::Definition(DefinitionBuilder::from_pest(pair, curr_serial))
            }
            Rule::axiom_block => Self::Axiom(AxiomBuilder::from_pest(pair, curr_serial)),
            Rule::theorem_block => Self::Theorem(TheoremBuilder::from_pest(pair, curr_serial)),
            Rule::proof_block => Self::Proof(ProofBuilder::from_pest(pair, curr_serial)),

            Rule::ul_block => Self::List(ListBuilder::from_pest(pair, false)),
            Rule::ol_block => Self::List(ListBuilder::from_pest(pair, true)),

            Rule::table_block => Self::Table(TableBuilder::from_pest(pair)),
            Rule::quote_block => Self::Quote(QuoteBuilder::from_pest(pair)),
            Rule::heading_block => Self::Heading(HeadingBuilder::from_pest(pair)),
            Rule::todo_block => Self::Todo(TodoBuilder::from_pest(pair)),
            Rule::text_block => Self::Text(TextBlockBuilder::from_pest(pair)),

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
    fn count(&'a self, counter: &mut BlockCounter, book_id: &str, chapter_id: &str, page_id: &str) {
        match self {
            Self::System(system_ref) => {
                system_ref.count(counter.system());
                system_ref.set_href(book_id, chapter_id, page_id);
            }
            Self::Type(type_ref) => {
                type_ref.count(counter.ty());
                type_ref.set_href(book_id, chapter_id, page_id);
            }
            Self::Symbol(symbol_ref) => {
                symbol_ref.count(counter.symbol());
                symbol_ref.set_href(book_id, chapter_id, page_id);
            }
            Self::Definition(definition_ref) => {
                definition_ref.count(counter.definition());
                definition_ref.set_href(book_id, chapter_id, page_id);
            }
            Self::Axiom(axiom_ref) => {
                axiom_ref.count(counter.axiom());
                axiom_ref.set_href(book_id, chapter_id, page_id);
            }
            Self::Theorem(theorem_ref) => {
                theorem_ref.count(counter.theorem());
                theorem_ref.set_href(book_id, chapter_id, page_id);
            }
            Self::Proof(proof_ref) => {
                proof_ref.count(counter.proof());
                proof_ref.set_href(book_id, chapter_id, page_id);
            }

            Self::List(list_ref) => list_ref.count(counter.list()),
            Self::Table(table_ref) => table_ref.count(counter.table()),
            Self::Quote(quote_ref) => quote_ref.count(counter.quote()),
            Self::Todo(todo_ref) => todo_ref.count(counter.todo()),
            Self::Heading(heading_ref) => heading_ref.count(counter.heading()),
            Self::Text(text_ref) => text_ref.count(counter.text()),
        }
    }

    // TODO: Remove.
    fn get_ref(&self) -> Block {
        match self {
            Self::System(system_ref) => Block::System(system_ref.get_ref()),
            Self::Type(type_ref) => Block::Type(type_ref.get_ref()),
            Self::Symbol(symbol_ref) => Block::Symbol(symbol_ref.get_ref()),
            Self::Definition(definition_ref) => Block::Definition(definition_ref.get_ref()),
            Self::Axiom(axiom_ref) => Block::Axiom(axiom_ref.get_ref()),
            Self::Theorem(theorem_ref) => Block::Theorem(theorem_ref.get_ref()),
            Self::Proof(proof_ref) => Block::Proof(proof_ref.get_ref()),

            Self::List(list_ref) => Block::List(list_ref.get_ref()),
            Self::Table(table_ref) => Block::Table(table_ref.get_ref()),
            Self::Quote(quote_ref) => Block::Quote(quote_ref.get_ref()),
            Self::Todo(todo_ref) => Block::Todo(todo_ref.get_ref()),
            Self::Heading(heading_ref) => Block::Heading(heading_ref.get_ref()),
            Self::Text(text_ref) => Block::Text(text_ref.get_ref()),
        }
    }

    // TODO: Remove.
    pub fn build_directory(
        &'a self,
        systems: &mut Vec<SystemBlock>,
        types: &mut Vec<TypeBlock>,
        symbols: &mut Vec<SymbolBlock>,
        definitions: &mut Vec<DefinitionBlock>,
        axioms: &mut Vec<AxiomBlock>,
        theorems: &mut Vec<TheoremBlock>,
        proofs: &mut Vec<ProofBlock>,
        lists: &mut Vec<ListBlock>,
        tables: &mut Vec<TableBlock>,
        quotes: &mut Vec<QuoteBlock>,
        todos: &mut Vec<TodoBlock>,
        headings: &mut Vec<HeadingBlock>,
        texts: &mut Vec<TextBlock>,
    ) {
        match self {
            Self::System(system_ref) => systems.push(system_ref.finish()),
            Self::Type(type_ref) => types.push(type_ref.finish()),
            Self::Symbol(symbol_ref) => symbols.push(symbol_ref.finish()),
            Self::Definition(definition_ref) => definitions.push(definition_ref.finish()),
            Self::Axiom(axiom_ref) => axioms.push(axiom_ref.finish()),
            Self::Theorem(theorem_ref) => theorems.push(theorem_ref.finish()),
            Self::Proof(proof_ref) => proofs.push(proof_ref.finish()),

            Self::List(list_ref) => lists.push(list_ref.finish()),
            Self::Table(table_ref) => tables.push(table_ref.finish()),
            Self::Quote(quote_ref) => quotes.push(quote_ref.finish()),
            Self::Todo(todo_ref) => todos.push(todo_ref.finish()),
            Self::Heading(heading_ref) => headings.push(heading_ref.finish()),
            Self::Text(text_ref) => texts.push(text_ref.finish()),
        }
    }
}

pub struct PageBuilder<'a> {
    id: String,
    name: String,

    blocks: Vec<BlockBuilder<'a>>,
    local_bibliography: OnceCell<LocalBibliographyBuilder<'a>>,
}

impl<'a> PageBuilder<'a> {
    fn from_pest(
        pair: Pair<Rule>,
        library_path: &Path,
        book_id: &str,
        chapter_id: &str,
        serial: &mut usize,
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
                };
            }
        };

        let blocks = pair
            .into_inner()
            .filter_map(|pair| match pair.as_rule() {
                Rule::EOI => None,

                _ => Some(BlockBuilder::from_pest(pair, serial)),
            })
            .collect();

        PageBuilder {
            id,
            name,

            blocks,
            local_bibliography: OnceCell::new(),
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
    fn count(&'a self, counter: &mut BlockCounter, book_id: &str, chapter_id: &str) {
        for block in &self.blocks {
            block.count(counter, book_id, chapter_id, &self.id);
        }
    }

    // TODO: Remove.
    pub fn build_directory(
        &'a self,
        systems: &mut Vec<SystemBlock>,
        types: &mut Vec<TypeBlock>,
        symbols: &mut Vec<SymbolBlock>,
        definitions: &mut Vec<DefinitionBlock>,
        axioms: &mut Vec<AxiomBlock>,
        theorems: &mut Vec<TheoremBlock>,
        proofs: &mut Vec<ProofBlock>,
        lists: &mut Vec<ListBlock>,
        tables: &mut Vec<TableBlock>,
        quotes: &mut Vec<QuoteBlock>,
        todos: &mut Vec<TodoBlock>,
        headings: &mut Vec<HeadingBlock>,
        texts: &mut Vec<TextBlock>,
    ) {
        for block in &self.blocks {
            block.build_directory(
                systems,
                types,
                symbols,
                definitions,
                axioms,
                theorems,
                proofs,
                lists,
                tables,
                quotes,
                todos,
                headings,
                texts,
            );
        }
    }

    // TODO: Remove.
    pub fn finish(&self, book_id: &str, chapter_id: &str) -> Page {
        let id = self.id.clone();
        let name = self.name.clone();
        let href = format!("/{}/{}/{}", book_id, chapter_id, &id);
        let blocks = self.blocks.iter().map(BlockBuilder::get_ref).collect();
        let local_bibliography = self
            .local_bibliography
            .get()
            .map(LocalBibliographyBuilder::finish);

        Page::new(id, name, href, blocks, local_bibliography)
    }
}

pub struct ChapterBuilder<'a> {
    id: String,
    name: String,
    tagline: ParagraphBuilder<'a>,

    pages: Vec<PageBuilder<'a>>,
}

impl<'a> ChapterBuilder<'a> {
    fn from_pest(
        pair: Pair<Rule>,
        library_path: &Path,
        book_id: &str,
        serial: &mut usize,
        errors: &mut ParsingErrorContext,
    ) -> Self {
        assert_eq!(pair.as_rule(), Rule::manifest_chapter);
        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();

        let string = inner.next().unwrap();
        let string_contents = string.into_inner().next().unwrap();
        let name = string_contents.as_str().to_owned();

        let tagline = ParagraphBuilder::from_pest(inner.next().unwrap());

        let pages = inner
            .map(|pair| PageBuilder::from_pest(pair, library_path, book_id, &id, serial, errors))
            .collect();

        ChapterBuilder {
            id,
            name,
            tagline,

            pages,
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
    fn count(&'a self, counter: &mut BlockCounter, book_id: &str) {
        for page in &self.pages {
            page.count(counter, book_id, &self.id);
        }
    }

    // TODO: Remove.
    pub fn build_directory(
        &'a self,
        systems: &mut Vec<SystemBlock>,
        types: &mut Vec<TypeBlock>,
        symbols: &mut Vec<SymbolBlock>,
        definitions: &mut Vec<DefinitionBlock>,
        axioms: &mut Vec<AxiomBlock>,
        theorems: &mut Vec<TheoremBlock>,
        proofs: &mut Vec<ProofBlock>,
        lists: &mut Vec<ListBlock>,
        tables: &mut Vec<TableBlock>,
        quotes: &mut Vec<QuoteBlock>,
        todos: &mut Vec<TodoBlock>,
        headings: &mut Vec<HeadingBlock>,
        texts: &mut Vec<TextBlock>,
    ) {
        for page in &self.pages {
            page.build_directory(
                systems,
                types,
                symbols,
                definitions,
                axioms,
                theorems,
                proofs,
                lists,
                tables,
                quotes,
                todos,
                headings,
                texts,
            );
        }
    }

    // TODO: Remove.
    pub fn finish(&self, book_id: &str) -> Chapter {
        let id = self.id.clone();
        let name = self.name.clone();
        let href = format!("/{}/{}", book_id, &id);
        let tagline = self.tagline.finish();
        let pages = self
            .pages
            .iter()
            .map(|page| page.finish(book_id, &id))
            .collect();

        Chapter::new(id, name, href, tagline, pages)
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
}

impl<'a> BookBuilder<'a> {
    pub fn from_pest(
        pair: Pair<Rule>,
        library_path: &Path,
        serial: &mut usize,
        errors: &mut ParsingErrorContext,
    ) -> Self {
        assert_eq!(pair.as_rule(), Rule::manifest_book);
        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();

        let string = inner.next().unwrap();
        let string_contents = string.into_inner().next().unwrap();
        let name = string_contents.as_str().to_owned();

        let tagline = ParagraphBuilder::from_pest(inner.next().unwrap());

        let chapters = inner
            .map(|pair| ChapterBuilder::from_pest(pair, library_path, &id, serial, errors))
            .collect();

        BookBuilder {
            id,
            name,
            tagline,

            chapters,
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
    pub fn count(&'a self, counters: &mut BlockCounter) {
        for chapter in &self.chapters {
            chapter.count(counters, &self.id);
        }
    }

    // TODO: Remove.
    pub fn build_directory(
        &'a self,
        systems: &mut Vec<SystemBlock>,
        types: &mut Vec<TypeBlock>,
        symbols: &mut Vec<SymbolBlock>,
        definitions: &mut Vec<DefinitionBlock>,
        axioms: &mut Vec<AxiomBlock>,
        theorems: &mut Vec<TheoremBlock>,
        proofs: &mut Vec<ProofBlock>,
        lists: &mut Vec<ListBlock>,
        tables: &mut Vec<TableBlock>,
        quotes: &mut Vec<QuoteBlock>,
        todos: &mut Vec<TodoBlock>,
        headings: &mut Vec<HeadingBlock>,
        texts: &mut Vec<TextBlock>,
    ) {
        for chapter in &self.chapters {
            chapter.build_directory(
                systems,
                types,
                symbols,
                definitions,
                axioms,
                theorems,
                proofs,
                lists,
                tables,
                quotes,
                todos,
                headings,
                texts,
            );
        }
    }

    // TODO: Remove.
    pub fn finish(&self) -> Book {
        let id = self.id.clone();
        let name = self.name.clone();
        let href = format!("/{}", &id);
        let tagline = self.tagline.finish();
        let chapters = self
            .chapters
            .iter()
            .map(|chapter| chapter.finish(&id))
            .collect();

        Book::new(id, name, href, tagline, chapters)
    }
}

impl<'a> std::fmt::Debug for BookBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Book").field(&self.id).finish()
    }
}
