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

use crate::deduction::language::{Definition, Symbol, Type};
use crate::deduction::system::{Axiom, Proof, System, Theorem};
use crate::rendered::{BlockRendered, BookRendered, ChapterRendered, PageRendered};

use super::bibliography::LocalBibliography;
use super::language::{DefinitionBlock, SymbolBlock, TypeBlock, VariableBlock};
use super::system::{AxiomBlock, ProofBlock, SystemBlock, TheoremBlock};
use super::text::{
    BareText, HeadingBlock, ListBlock, Paragraph, QuoteBlock, TableBlock, Text, TodoBlock,
};
use super::Document;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BlockLocation {
    block: usize,
    page: usize,
    chapter: usize,
    book: usize,

    serial: usize,
}

impl BlockLocation {
    pub fn new() -> Self {
        BlockLocation {
            block: 0,
            page: 0,
            chapter: 0,
            book: 0,

            serial: 0,
        }
    }

    pub fn next_block(&mut self) -> Self {
        let ret = *self;
        self.block += 1;
        self.serial += 1;

        ret
    }

    pub fn next_page(&mut self) {
        self.page += 1;
        self.block = 0;
    }

    pub fn next_chapter(&mut self) {
        self.chapter += 1;
        self.page = 0;
        self.block = 0;
    }

    pub fn next_book(&mut self) {
        self.book += 1;
        self.chapter = 0;
        self.page = 0;
        self.block = 0;
    }

    pub fn block(&self) -> usize {
        self.block
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn chapter(&self) -> usize {
        self.chapter
    }

    pub fn book(&self) -> usize {
        self.book
    }

    pub fn serial(&self) -> usize {
        self.serial
    }
}

pub enum Block<'a> {
    System(SystemBlock<'a>),
    Type(TypeBlock<'a>),
    Symbol(SymbolBlock<'a>),
    Definition(DefinitionBlock<'a>),
    Axiom(AxiomBlock<'a>),
    Theorem(TheoremBlock<'a>),
    Proof(ProofBlock<'a>),

    List(ListBlock<'a>),
    Table(TableBlock<'a>),
    Quote(QuoteBlock),
    Heading(HeadingBlock),
    Todo(TodoBlock<'a>),
    Text(Text<'a>),
}

impl<'a> Block<'a> {
    fn crosslink(&'a self, document: &'a Document<'a>) {
        match self {
            Self::System(system_ref) => system_ref.crosslink(document),
            Self::Type(type_ref) => type_ref.crosslink(document),
            Self::Symbol(symbol_ref) => symbol_ref.crosslink(document),
            Self::Definition(definition_ref) => definition_ref.crosslink(document),
            Self::Axiom(axiom_ref) => axiom_ref.crosslink(document),
            Self::Theorem(theorem_ref) => theorem_ref.crosslink(document),
            Self::Proof(proof_ref) => proof_ref.crosslink(document),

            Self::List(list_ref) => list_ref.crosslink(document),
            Self::Table(table_ref) => table_ref.crosslink(document),
            Self::Todo(todo_ref) => todo_ref.crosslink(document),
            Self::Text(text_ref) => text_ref.crosslink(document),

            _ => {}
        }
    }

    fn system(&self) -> Option<&SystemBlock<'a>> {
        match self {
            Self::System(system_ref) => Some(system_ref),

            _ => None,
        }
    }

    fn ty(&self) -> Option<&TypeBlock<'a>> {
        match self {
            Self::Type(type_ref) => Some(type_ref),

            _ => None,
        }
    }

    fn symbol(&self) -> Option<&SymbolBlock<'a>> {
        match self {
            Self::Symbol(symbol_ref) => Some(symbol_ref),

            _ => None,
        }
    }

    fn definition(&self) -> Option<&DefinitionBlock<'a>> {
        match self {
            Self::Definition(definition_ref) => Some(definition_ref),

            _ => None,
        }
    }

    fn axiom(&self) -> Option<&AxiomBlock<'a>> {
        match self {
            Self::Axiom(axiom_ref) => Some(axiom_ref),

            _ => None,
        }
    }

    fn theorem(&self) -> Option<&TheoremBlock<'a>> {
        match self {
            Self::Theorem(theorem_ref) => Some(theorem_ref),

            _ => None,
        }
    }

    // TODO: Remove.
    fn count(&self, counter: &mut super::Counter) {
        match self {
            Self::System(system_ref) => {
                system_ref.count(counter.systems);
                counter.systems += 1;
            }
            Self::Type(type_ref) => {
                type_ref.count(counter.types);
                counter.types += 1;
            }
            Self::Symbol(symbol_ref) => {
                symbol_ref.count(counter.symbols);
                counter.symbols += 1;
            }
            Self::Definition(definition_ref) => {
                definition_ref.count(counter.definitions);
                counter.definitions += 1;
            }
            Self::Axiom(axiom_ref) => {
                axiom_ref.count(counter.axioms);
                counter.axioms += 1;
            }
            Self::Theorem(theorem_ref) => {
                theorem_ref.count(counter.theorems);
                counter.theorems += 1;
            }
            Self::Proof(proof_ref) => {
                proof_ref.count(counter.proofs);
                counter.proofs += 1;
            }

            _ => {}
        }
    }

    // TODO: Remove.
    fn populate_checkable(
        &'a self,
        systems: &mut Vec<System>,
        types: &mut Vec<Type>,
        symbols: &mut Vec<Symbol>,
        definitions: &mut Vec<Definition>,
        axioms: &mut Vec<Axiom>,
        theorems: &mut Vec<Theorem>,
        proofs: &mut Vec<Proof>,
    ) {
        match self {
            Self::System(system_ref) => systems.push(system_ref.checkable()),
            Self::Type(type_ref) => types.push(type_ref.checkable()),
            Self::Symbol(symbol_ref) => symbols.push(symbol_ref.checkable()),
            Self::Definition(definition_ref) => definitions.push(definition_ref.checkable()),
            Self::Axiom(axiom_ref) => axioms.push(axiom_ref.checkable()),
            Self::Theorem(theorem_ref) => theorems.push(theorem_ref.checkable()),
            Self::Proof(proof_ref) => proofs.push(proof_ref.checkable()),

            _ => {}
        }
    }

    // TODO: Remove.
    fn render(&self) -> BlockRendered {
        match self {
            Self::System(system_ref) => BlockRendered::System(system_ref.render()),
            Self::Type(type_ref) => BlockRendered::Type(type_ref.render()),
            Self::Symbol(symbol_ref) => BlockRendered::Symbol(symbol_ref.render()),
            Self::Definition(definition_ref) => BlockRendered::Definition(definition_ref.render()),
            Self::Axiom(axiom_ref) => BlockRendered::Axiom(axiom_ref.render()),
            Self::Theorem(theorem_ref) => BlockRendered::Theorem(theorem_ref.render()),
            Self::Proof(proof_ref) => BlockRendered::Proof(proof_ref.render()),

            Self::List(list_ref) => BlockRendered::List(list_ref.render()),
            Self::Table(table_ref) => BlockRendered::Table(table_ref.render()),
            Self::Quote(quote_ref) => BlockRendered::Quote(quote_ref.render()),
            Self::Heading(heading_ref) => BlockRendered::Heading(heading_ref.render()),
            Self::Todo(todo_ref) => BlockRendered::Todo(todo_ref.render()),
            Self::Text(text_ref) => BlockRendered::Text(text_ref.render()),
        }
    }
}

impl<'a> std::fmt::Debug for Block<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub struct BlockRef<'a> {
    location: BlockLocation,
    block: OnceCell<&'a Block<'a>>,
}

impl<'a> BlockRef<'a> {
    pub fn new(location: BlockLocation) -> Self {
        BlockRef {
            location,
            block: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.block.set(&document[self.location]).unwrap();
    }

    // TODO: Remove.
    pub fn render(&self, text: Option<&BareText>) -> String {
        match self.block.get().unwrap() {
            Block::System(system_ref) => {
                let name = text
                    .map(BareText::render)
                    .unwrap_or(system_ref.name().to_owned());

                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    system_ref.href(),
                    name
                )
            }

            Block::Type(type_ref) => todo!(),

            Block::Symbol(symbol_ref) => {
                let name = text
                    .map(BareText::render)
                    .unwrap_or_else(|| symbol_ref.name().to_owned());

                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    symbol_ref.href(),
                    name
                )
            }

            Block::Definition(definition_ref) => {
                let name = text
                    .map(BareText::render)
                    .unwrap_or_else(|| definition_ref.name().to_owned());

                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    definition_ref.href(),
                    name
                )
            }

            Block::Axiom(axiom_ref) => {
                let name = text
                    .map(BareText::render)
                    .unwrap_or_else(|| axiom_ref.name().to_owned());

                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    axiom_ref.href(),
                    name
                )
            }

            Block::Theorem(theorem_ref) => {
                let name = text
                    .map(BareText::render)
                    .unwrap_or_else(|| theorem_ref.name().to_owned());

                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    theorem_ref.href(),
                    name
                )
            }

            _ => todo!(),
        }
    }
}

pub struct SystemBlockRef<'a> {
    location: BlockLocation,
    block: OnceCell<&'a SystemBlock<'a>>,
}

impl<'a> SystemBlockRef<'a> {
    pub fn new(location: BlockLocation) -> Self {
        SystemBlockRef {
            location,
            block: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        let block = &document[self.location].system().unwrap();
        self.block.set(block).unwrap();
    }

    pub fn id(&self) -> &str {
        self.block.get().unwrap().id()
    }

    pub fn name(&self) -> &str {
        self.block.get().unwrap().name()
    }

    // TODO: Remove.
    pub fn index(&self) -> usize {
        self.block.get().unwrap().index()
    }

    // TODO: Remove.
    pub fn href(&self) -> &str {
        self.block.get().unwrap().href()
    }
}

pub struct TypeBlockRef<'a> {
    location: BlockLocation,
    block: OnceCell<&'a TypeBlock<'a>>,
}

impl<'a> TypeBlockRef<'a> {
    pub fn new(location: BlockLocation) -> Self {
        TypeBlockRef {
            location,
            block: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        let block = &document[self.location].ty().unwrap();
        self.block.set(block).unwrap();
    }

    pub fn id(&self) -> &str {
        self.block.get().unwrap().id()
    }

    // TODO: Remove.
    pub fn index(&self) -> usize {
        self.block.get().unwrap().index()
    }
}

pub struct SymbolBlockRef<'a> {
    location: BlockLocation,
    block: OnceCell<&'a SymbolBlock<'a>>,
}

impl<'a> SymbolBlockRef<'a> {
    pub fn new(location: BlockLocation) -> Self {
        SymbolBlockRef {
            location,
            block: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        let block = &document[self.location].symbol().unwrap();
        self.block.set(block).unwrap();
    }

    // TODO: Remove.
    pub fn index(&self) -> usize {
        self.block.get().unwrap().index()
    }

    pub fn name(&self) -> &str {
        self.block.get().unwrap().name()
    }
}

pub struct DefinitionBlockRef<'a> {
    location: BlockLocation,
    block: OnceCell<&'a DefinitionBlock<'a>>,
}

impl<'a> DefinitionBlockRef<'a> {
    pub fn new(location: BlockLocation) -> Self {
        DefinitionBlockRef {
            location,
            block: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        let block = &document[self.location].definition().unwrap();
        self.block.set(block).unwrap();
    }

    // TODO: Remove.
    pub fn index(&self) -> usize {
        self.block.get().unwrap().index()
    }

    pub fn name(&self) -> &str {
        self.block.get().unwrap().name()
    }
}

pub struct AxiomBlockRef<'a> {
    location: BlockLocation,
    block: OnceCell<&'a AxiomBlock<'a>>,
}

impl<'a> AxiomBlockRef<'a> {
    pub fn new(location: BlockLocation) -> Self {
        AxiomBlockRef {
            location,
            block: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        let block = &document[self.location].axiom().unwrap();
        self.block.set(block).unwrap();
    }

    // TODO: Remove.
    pub fn index(&self) -> usize {
        self.block.get().unwrap().index()
    }

    pub fn name(&self) -> &str {
        self.block.get().unwrap().name()
    }

    pub fn href(&self) -> &str {
        self.block.get().unwrap().href()
    }
}

pub struct TheoremBlockRef<'a> {
    location: BlockLocation,
    block: OnceCell<&'a TheoremBlock<'a>>,
}

impl<'a> TheoremBlockRef<'a> {
    pub fn new(location: BlockLocation) -> Self {
        TheoremBlockRef {
            location,
            block: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        let block = &document[self.location].theorem().unwrap();
        self.block.set(block).unwrap();
    }

    pub fn vars(&self) -> &[VariableBlock<'a>] {
        self.block.get().unwrap().vars()
    }

    // TODO: Remove.
    pub fn index(&self) -> usize {
        self.block.get().unwrap().index()
    }

    pub fn name(&self) -> &str {
        self.block.get().unwrap().name()
    }

    pub fn href(&self) -> &str {
        self.block.get().unwrap().href()
    }
}

pub enum DeductableBlockRef<'a> {
    Axiom(AxiomBlockRef<'a>),
    Theorem(TheoremBlockRef<'a>),
}

impl<'a> DeductableBlockRef<'a> {
    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        match self {
            Self::Axiom(axiom_ref) => axiom_ref.crosslink(document),
            Self::Theorem(theorem_ref) => theorem_ref.crosslink(document),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Axiom(axiom_ref) => axiom_ref.name(),
            Self::Theorem(theorem_ref) => theorem_ref.name(),
        }
    }

    pub fn href(&self) -> &str {
        match self {
            Self::Axiom(axiom_ref) => axiom_ref.href(),
            Self::Theorem(theorem_ref) => theorem_ref.href(),
        }
    }
}

pub struct Page<'a> {
    id: String,
    name: String,

    blocks: Vec<Block<'a>>,

    local_bibliography: LocalBibliography<'a>,

    href: String,
}

impl<'a> Page<'a> {
    pub fn new(
        id: String,
        name: String,
        blocks: Vec<Block<'a>>,
        local_bibliography: LocalBibliography<'a>,
        href: String,
    ) -> Self {
        Page {
            id,
            name,

            blocks,

            local_bibliography,

            href,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.local_bibliography.crosslink(document);

        for block in &self.blocks {
            block.crosslink(document);
        }
    }

    // TODO: Remove.
    pub fn count(&self, counter: &mut super::Counter) {
        for block in &self.blocks {
            block.count(counter);
        }
    }

    // TODO: Remove.
    fn populate_checkable(
        &'a self,
        systems: &mut Vec<System>,
        types: &mut Vec<Type>,
        symbols: &mut Vec<Symbol>,
        definitions: &mut Vec<Definition>,
        axioms: &mut Vec<Axiom>,
        theorems: &mut Vec<Theorem>,
        proofs: &mut Vec<Proof>,
    ) {
        for block in &self.blocks {
            block.populate_checkable(
                systems,
                types,
                symbols,
                definitions,
                axioms,
                theorems,
                proofs,
            );
        }
    }

    // TODO: Remove.
    fn render(
        &self,
        chapter_num: usize,
        chapter_name: &str,
        page_num: usize,
        prev_href: &str,
        up_href: &str,
        next_href: Option<&str>,
    ) -> PageRendered {
        let id = self.id.clone();
        let page_name = self.name.clone();
        let href = self.href.clone();

        let blocks = self.blocks.iter().map(Block::render).collect();

        let local_bibliography = self.local_bibliography.render();

        PageRendered::new(
            id,
            href,
            page_num,
            chapter_num,
            page_name,
            chapter_name.to_owned(),
            prev_href.to_owned(),
            up_href.to_owned(),
            next_href.map(str::to_owned),
            blocks,
            Some(local_bibliography),
        )
    }
}

impl<'a> Index<BlockLocation> for Page<'a> {
    type Output = Block<'a>;

    fn index(&self, location: BlockLocation) -> &Self::Output {
        &self.blocks[location.block()]
    }
}

pub struct Chapter<'a> {
    id: String,
    name: String,
    tagline: Paragraph<'a>,

    pages: Vec<Page<'a>>,

    // TODO: Remove
    href: String,
}

impl<'a> Chapter<'a> {
    pub fn new(
        id: String,
        name: String,
        tagline: Paragraph<'a>,
        pages: Vec<Page<'a>>,
        href: String,
    ) -> Self {
        Chapter {
            id,
            name,
            tagline,

            pages,

            href,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.tagline.crosslink(document);

        for page in &self.pages {
            page.crosslink(document);
        }
    }

    // TODO: Remove.
    pub fn count(&self, counter: &mut super::Counter) {
        for page in &self.pages {
            page.count(counter);
        }
    }

    // TODO: Remove.
    fn populate_checkable(
        &'a self,
        systems: &mut Vec<System>,
        types: &mut Vec<Type>,
        symbols: &mut Vec<Symbol>,
        definitions: &mut Vec<Definition>,
        axioms: &mut Vec<Axiom>,
        theorems: &mut Vec<Theorem>,
        proofs: &mut Vec<Proof>,
    ) {
        for page in &self.pages {
            page.populate_checkable(
                systems,
                types,
                symbols,
                definitions,
                axioms,
                theorems,
                proofs,
            );
        }
    }

    // TODO: Remove.
    fn render(&self, chapter_num: usize, next_chapter_href: Option<&str>) -> ChapterRendered {
        let id = self.id.clone();
        let href = self.href.clone();
        let chapter_name = self.name.clone();
        let tagline = self.tagline.render();

        let mut prev_href = &href;
        let up_href = &href;
        let mut next_href = None;
        let pages = (0..self.pages.len())
            .map(|page_num| {
                let page = &self.pages[page_num];

                next_href = self
                    .pages
                    .get(page_num + 1)
                    .map(|page| page.href.as_ref())
                    .or(next_chapter_href);

                let ret = page.render(
                    chapter_num,
                    &chapter_name,
                    page_num,
                    prev_href,
                    up_href,
                    next_href,
                );

                prev_href = &page.href;

                ret
            })
            .collect();

        ChapterRendered::new(id, href, chapter_num, chapter_name, tagline, pages)
    }
}

impl<'a> Index<BlockLocation> for Chapter<'a> {
    type Output = Block<'a>;

    fn index(&self, location: BlockLocation) -> &Self::Output {
        &self.pages[location.page()][location]
    }
}

pub struct Book<'a> {
    id: String,
    name: String,
    tagline: Paragraph<'a>,

    chapters: Vec<Chapter<'a>>,

    // TODO: Remove.
    href: String,
}

impl<'a> Book<'a> {
    pub fn new(
        id: String,
        name: String,
        tagline: Paragraph<'a>,
        chapters: Vec<Chapter<'a>>,
        href: String,
    ) -> Self {
        Book {
            id,
            name,
            tagline,

            chapters,

            href,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.tagline.crosslink(document);

        for chapter in &self.chapters {
            chapter.crosslink(document);
        }
    }

    // TODO: Remove.
    pub fn count(&self, counter: &mut super::Counter) {
        for chapter in &self.chapters {
            chapter.count(counter);
        }
    }

    // TODO: Remove.
    pub fn populate_checkable(
        &'a self,
        systems: &mut Vec<System>,
        types: &mut Vec<Type>,
        symbols: &mut Vec<Symbol>,
        definitions: &mut Vec<Definition>,
        axioms: &mut Vec<Axiom>,
        theorems: &mut Vec<Theorem>,
        proofs: &mut Vec<Proof>,
    ) {
        for chapter in &self.chapters {
            chapter.populate_checkable(
                systems,
                types,
                symbols,
                definitions,
                axioms,
                theorems,
                proofs,
            );
        }
    }

    // TODO: Remove.
    pub fn render(&self, book_num: usize) -> BookRendered {
        let id = self.id.clone();
        let href = self.href.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render();

        let chapters = (0..self.chapters.len())
            .map(|chapter_num| {
                let chapter = &self.chapters[chapter_num];

                let next_chapter_href = self
                    .chapters
                    .get(chapter_num + 1)
                    .map(|chapter| chapter.href.as_ref());

                chapter.render(chapter_num, next_chapter_href)
            })
            .collect();

        BookRendered::new(id, href, book_num, name, tagline, chapters)
    }
}

impl<'a> Index<BlockLocation> for Book<'a> {
    type Output = Block<'a>;

    fn index(&self, location: BlockLocation) -> &Self::Output {
        &self.chapters[location.chapter()][location]
    }
}

#[cfg(test)]
mod tests {
    use super::BlockLocation;

    #[test]
    fn block_location() {
        let mut location = BlockLocation::new();

        assert_eq!(
            location.next_block(),
            BlockLocation {
                block: 0,
                page: 0,
                chapter: 0,
                book: 0,

                serial: 0,
            }
        );

        location.next_page();
        assert_eq!(
            location.next_block(),
            BlockLocation {
                block: 0,
                page: 1,
                chapter: 0,
                book: 0,

                serial: 1,
            }
        );

        location.next_chapter();
        location.next_page();
        assert_eq!(
            location.next_block(),
            BlockLocation {
                block: 0,
                page: 1,
                chapter: 1,
                book: 0,

                serial: 2,
            }
        );

        location.next_book();
        location.next_chapter();
        location.next_page();
        assert_eq!(
            location.next_block(),
            BlockLocation {
                block: 0,
                page: 1,
                chapter: 1,
                book: 1,

                serial: 3,
            }
        );
    }
}
