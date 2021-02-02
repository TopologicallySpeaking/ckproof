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

use std::collections::{HashMap, HashSet};
use std::ops::Index;

use pest::iterators::Pair;

use crate::document::directory::{
    AxiomBlockRef, Bibliography, BibliographyRef, Block, BlockDirectory, BlockReference,
    HeadingBlockRef, LocalBibliography, LocalBibliographyRef, ProofBlockRef, ProofBlockStepRef,
    QuoteBlockRef, SymbolBlockRef, SystemBlockRef, TableBlockRef, TextBlockRef, TheoremBlockRef,
    TodoBlockRef, TypeBlockRef, VariableBlockRef,
};
use crate::document::text::Mla;

use super::deduction::{AxiomBuilder, ProofBuilder, TheoremBuilder};
use super::errors::{
    BibliographyParsingError, ParsingError, ParsingErrorContext, ProofStepParsingError,
    SystemParsingError, VariableParsingError,
};
use super::language::{
    ReadBuilder, SymbolBuilder, SystemBuilder, TypeBuilder, TypeSignatureBuilder, VariableBuilder,
};
use super::text::{
    HeadingBuilder, MlaBuilderEntries, QuoteBuilder, TableBuilder, TextBlockBuilder, TodoBuilder,
};
use super::{BlockLocation, Rule};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SystemBuilderRef(usize);

impl SystemBuilderRef {
    pub fn finish(&self) -> SystemBlockRef {
        SystemBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TypeBuilderRef(usize);

impl TypeBuilderRef {
    pub fn finish(&self) -> TypeBlockRef {
        TypeBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SymbolBuilderRef(usize);

impl SymbolBuilderRef {
    pub fn finish(&self) -> SymbolBlockRef {
        SymbolBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct VariableBuilderRef(usize);

impl VariableBuilderRef {
    pub fn get(&self) -> usize {
        self.0
    }

    pub fn finish(&self) -> VariableBlockRef {
        VariableBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AxiomBuilderRef(usize);

impl AxiomBuilderRef {
    pub fn finish(&self) -> AxiomBlockRef {
        AxiomBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TheoremBuilderRef(usize);

impl TheoremBuilderRef {
    pub fn finish(&self) -> TheoremBlockRef {
        TheoremBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ProofBuilderRef(usize);

impl ProofBuilderRef {
    pub fn finish(&self) -> ProofBlockRef {
        ProofBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ProofBuilderStepRef(usize);

impl ProofBuilderStepRef {
    pub fn new(i: usize) -> ProofBuilderStepRef {
        ProofBuilderStepRef(i)
    }

    pub fn finish(&self) -> ProofBlockStepRef {
        ProofBlockStepRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TableBuilderRef(usize);

impl TableBuilderRef {
    pub fn finish(&self) -> TableBlockRef {
        TableBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct QuoteBuilderRef(usize);

impl QuoteBuilderRef {
    pub fn finish(&self) -> QuoteBlockRef {
        QuoteBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct HeadingBuilderRef(usize);

impl HeadingBuilderRef {
    pub fn finish(&self) -> HeadingBlockRef {
        HeadingBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TodoBuilderRef(usize);

impl TodoBuilderRef {
    pub fn finish(&self) -> TodoBlockRef {
        TodoBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TextBlockBuilderRef(usize);

impl TextBlockBuilderRef {
    pub fn finish(&self) -> TextBlockRef {
        TextBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BibliographyBuilderRef(usize);

impl BibliographyBuilderRef {
    pub fn finish(&self) -> BibliographyRef {
        BibliographyRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LocalBibliographyBuilderRef(usize);

impl LocalBibliographyBuilderRef {
    pub fn finish(&self) -> LocalBibliographyRef {
        LocalBibliographyRef::new(self.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ReadableKind {
    Symbol(SymbolBuilderRef),
}

#[derive(Clone, Copy, Debug)]
pub struct Readable {
    kind: ReadableKind,
    arity: usize,
}

impl Readable {
    pub fn symbol(symbol_ref: SymbolBuilderRef, arity: usize) -> Readable {
        Readable {
            kind: ReadableKind::Symbol(symbol_ref),
            arity,
        }
    }

    pub fn kind(&self) -> ReadableKind {
        self.kind
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ReadSignature {
    read: ReadBuilder,
    inputs: Vec<TypeSignatureBuilder>,
}

impl ReadSignature {
    pub fn new(read: ReadBuilder, inputs: Vec<TypeSignatureBuilder>) -> ReadSignature {
        ReadSignature { read, inputs }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SystemBuilderChild {
    Type(TypeBuilderRef),
    Symbol(SymbolBuilderRef),
    Axiom(AxiomBuilderRef),
    Theorem(TheoremBuilderRef),
}

impl SystemBuilderChild {
    pub fn ty(self) -> Option<TypeBuilderRef> {
        match self {
            Self::Type(type_ref) => Some(type_ref),
            _ => None,
        }
    }

    pub fn symbol(self) -> Option<SymbolBuilderRef> {
        match self {
            Self::Symbol(symbol_ref) => Some(symbol_ref),
            _ => None,
        }
    }

    pub fn axiom(self) -> Option<AxiomBuilderRef> {
        match self {
            Self::Axiom(axiom_ref) => Some(axiom_ref),
            _ => None,
        }
    }

    pub fn theorem(self) -> Option<TheoremBuilderRef> {
        match self {
            Self::Theorem(theorem_ref) => Some(theorem_ref),
            _ => None,
        }
    }

    pub fn finish(&self) -> BlockReference {
        match self {
            Self::Type(type_ref) => type_ref.finish().into(),
            Self::Symbol(symbol_ref) => symbol_ref.finish().into(),
            Self::Axiom(axiom_ref) => axiom_ref.finish().into(),
            Self::Theorem(theorem_ref) => theorem_ref.finish().into(),
        }
    }
}

#[derive(Debug)]
pub enum BlockBuilder {
    System(SystemBuilderRef),
    Type(TypeBuilderRef),
    Symbol(SymbolBuilderRef),
    Axiom(AxiomBuilderRef),
    Theorem(TheoremBuilderRef),
    Proof(ProofBuilderRef),

    Table(TableBuilderRef),
    Quote(QuoteBuilderRef),
    Heading(HeadingBuilderRef),
    Todo(TodoBuilderRef),
    Text(TextBlockBuilderRef),
}

impl BlockBuilder {
    pub fn from_pest(
        pair: Pair<Rule>,
        directory: &mut BuilderDirectory,
        serial: BlockLocation,
        href: &str,
    ) -> BlockBuilder {
        match pair.as_rule() {
            Rule::system_block => {
                Self::System(directory.add_system(SystemBuilder::from_pest(pair, href)))
            }
            Rule::type_block => {
                Self::Type(directory.add_type(TypeBuilder::from_pest(pair, serial, href)))
            }
            Rule::symbol_block => {
                Self::Symbol(directory.add_symbol(SymbolBuilder::from_pest(pair, serial, href)))
            }
            Rule::axiom_block => {
                Self::Axiom(directory.add_axiom(AxiomBuilder::from_pest(pair, serial, href)))
            }
            Rule::theorem_block => {
                Self::Theorem(directory.add_theorem(TheoremBuilder::from_pest(pair, serial, href)))
            }
            Rule::proof_block => {
                Self::Proof(directory.add_proof(ProofBuilder::from_pest(pair, serial, href)))
            }

            Rule::table_block => Self::Table(directory.add_table(TableBuilder::from_pest(pair))),
            Rule::quote_block => Self::Quote(directory.add_quote(QuoteBuilder::from_pest(pair))),
            Rule::todo_block => Self::Todo(directory.add_todo(TodoBuilder::from_pest(pair))),
            Rule::heading_block => {
                Self::Heading(directory.add_heading(HeadingBuilder::from_pest(pair)))
            }
            Rule::text_block => Self::Text(directory.add_text(TextBlockBuilder::from_pest(pair))),

            _ => unreachable!(),
        }
    }

    fn bib_refs<'a>(
        &self,
        directory: &'a BuilderDirectory,
    ) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + 'a> {
        match self {
            Self::System(system_ref) => directory[*system_ref].bib_refs(),
            Self::Type(type_ref) => directory[*type_ref].bib_refs(),
            Self::Symbol(symbol_ref) => directory[*symbol_ref].bib_refs(),
            Self::Axiom(axiom_ref) => directory[*axiom_ref].bib_refs(),
            Self::Theorem(theorem_ref) => directory[*theorem_ref].bib_refs(),
            Self::Proof(proof_ref) => directory[*proof_ref].bib_refs(),

            Self::Table(table_ref) => directory[*table_ref].bib_refs(),
            Self::Heading(heading_ref) => directory[*heading_ref].bib_refs(),
            Self::Quote(quote_ref) => directory[*quote_ref].bib_refs(),
            Self::Todo(todo_ref) => directory[*todo_ref].bib_refs(),
            Self::Text(text_ref) => directory[*text_ref].bib_refs(),
        }
    }

    fn set_local_bib_refs(
        &self,
        directory: &BuilderDirectory,
        index: &LocalBibliographyBuilderIndex,
    ) {
        match self {
            Self::System(system_ref) => directory[*system_ref].set_local_bib_refs(index),
            Self::Type(type_ref) => directory[*type_ref].set_local_bib_refs(index),
            Self::Symbol(symbol_ref) => directory[*symbol_ref].set_local_bib_refs(index),
            Self::Axiom(axiom_ref) => directory[*axiom_ref].set_local_bib_refs(index),
            Self::Theorem(theorem_ref) => directory[*theorem_ref].set_local_bib_refs(index),
            Self::Proof(proof_ref) => directory[*proof_ref].set_local_bib_refs(index),

            Self::Table(table_ref) => directory[*table_ref].set_local_bib_refs(index),
            Self::Heading(_) => {}
            Self::Quote(quote_ref) => directory[*quote_ref].set_local_bib_refs(index),
            Self::Todo(todo_ref) => directory[*todo_ref].set_local_bib_refs(index),
            Self::Text(text_ref) => directory[*text_ref].set_local_bib_refs(index),
        }
    }

    pub fn finish(&self) -> Block {
        match self {
            Self::System(system_ref) => system_ref.finish().into(),
            Self::Type(type_ref) => type_ref.finish().into(),
            Self::Symbol(symbol_ref) => symbol_ref.finish().into(),
            Self::Axiom(axiom_ref) => axiom_ref.finish().into(),
            Self::Theorem(theorem_ref) => theorem_ref.finish().into(),
            Self::Proof(proof_ref) => proof_ref.finish().into(),

            Self::Table(table_ref) => table_ref.finish().into(),
            Self::Quote(quote_ref) => quote_ref.finish().into(),
            Self::Heading(heading_ref) => heading_ref.finish().into(),
            Self::Todo(todo_ref) => todo_ref.finish().into(),
            Self::Text(text_ref) => text_ref.finish().into(),
        }
    }
}

pub struct LocalIndex<'a> {
    parent_system: &'a SystemIndex,

    vars: HashMap<String, VariableBuilderRef>,
}

impl<'a> LocalIndex<'a> {
    pub fn add_vars<F>(
        &mut self,
        vars: &[VariableBuilder],
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(VariableBuilderRef, VariableParsingError) -> ParsingError,
    {
        for (i, var) in vars.iter().enumerate() {
            if let Some(old_var) = self.vars.get(var.id()) {
                errors.err(generate_error(
                    VariableBuilderRef(i),
                    VariableParsingError::IdAlreadyTaken(*old_var),
                ));
            } else {
                self.vars.insert(var.id().to_owned(), VariableBuilderRef(i));
            }
        }
    }

    pub fn search_operator(&self, read_signature: &ReadSignature) -> Option<Readable> {
        self.parent_system.search_operator(read_signature)
    }

    pub fn search_variable(&self, var_id: &str) -> Option<VariableBuilderRef> {
        self.vars.get(var_id).copied()
    }
}

struct SystemIndex {
    system_ref: SystemBuilderRef,

    children: HashMap<String, SystemBuilderChild>,
    operators: HashMap<ReadSignature, Readable>,
}

impl SystemIndex {
    fn new(system_ref: SystemBuilderRef) -> SystemIndex {
        SystemIndex {
            system_ref,

            children: HashMap::new(),
            operators: HashMap::new(),
        }
    }

    fn add_child(
        &mut self,
        id: &str,
        child_ref: SystemBuilderChild,
        errors: &mut ParsingErrorContext,
    ) {
        if let Some(old_child_ref) = self.children.get(id).copied() {
            errors.err(ParsingError::system_child_id_already_taken(
                child_ref,
                old_child_ref,
            ));
        }

        self.children.insert(id.to_owned(), child_ref);
    }

    fn search_child(&self, id: &str) -> Option<SystemBuilderChild> {
        self.children.get(id).copied()
    }

    fn add_operator(
        &mut self,
        read_signature: ReadSignature,
        readable: Readable,
        errors: &mut ParsingErrorContext,
    ) {
        if let Some(old_readable) = self.operators.get(&read_signature).copied() {
            errors.err(ParsingError::read_signature_already_taken(
                readable,
                old_readable,
            ));
        }

        self.operators.insert(read_signature, readable);
    }

    fn search_operator(&self, read_signature: &ReadSignature) -> Option<Readable> {
        self.operators.get(read_signature).copied()
    }
}

struct BuilderIndex {
    systems: HashMap<String, SystemIndex>,
}

impl BuilderIndex {
    fn new() -> BuilderIndex {
        BuilderIndex {
            systems: HashMap::new(),
        }
    }

    fn add_system(
        &mut self,
        id: &str,
        system_ref: SystemBuilderRef,
        errors: &mut ParsingErrorContext,
    ) {
        if let Some(old_index) = self.systems.get(id) {
            errors.err(ParsingError::SystemError(
                system_ref,
                SystemParsingError::IdAlreadyTaken(old_index.system_ref),
            ));
        }

        let new_index = SystemIndex::new(system_ref);
        self.systems.insert(id.to_owned(), new_index);
    }

    fn search_system(&self, id: &str) -> Option<SystemBuilderRef> {
        self.systems.get(id).map(|index| index.system_ref)
    }

    fn add_system_child(
        &mut self,
        system_id: &str,
        child_id: &str,
        child_ref: SystemBuilderChild,
        errors: &mut ParsingErrorContext,
    ) {
        match self.systems.get_mut(system_id) {
            Some(index) => index.add_child(child_id, child_ref, errors),
            None => errors.err(ParsingError::system_child_parent_not_found(child_ref)),
        }
    }

    fn search_system_child(&self, system_id: &str, child_id: &str) -> Option<SystemBuilderChild> {
        self.systems
            .get(system_id)
            .and_then(|index| index.search_child(child_id))
    }

    fn add_operator(
        &mut self,
        system_id: &str,
        read_signature: ReadSignature,
        readable: Readable,
        errors: &mut ParsingErrorContext,
    ) {
        match self.systems.get_mut(system_id) {
            Some(index) => index.add_operator(read_signature, readable, errors),

            // This possibility has already been ruled out in `Self::add_system_child`.
            None => unreachable!(),
        }
    }
}

pub struct TagIndex {
    tags: HashMap<String, ProofBuilderStepRef>,
}

impl TagIndex {
    pub fn new() -> TagIndex {
        TagIndex {
            tags: HashMap::new(),
        }
    }

    pub fn add_tag<F>(
        &mut self,
        tag: &str,
        step_ref: ProofBuilderStepRef,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ProofBuilderStepRef, ProofStepParsingError) -> ParsingError,
    {
        if let Some(old_step) = self.tags.get(tag) {
            errors.err(generate_error(
                step_ref,
                ProofStepParsingError::TagAlreadyTaken(*old_step),
            ));
        }

        self.tags.insert(tag.to_owned(), step_ref);
    }

    pub fn search(&self, tag: &str) -> Option<ProofBuilderStepRef> {
        self.tags.get(tag).copied()
    }
}

pub struct BuilderDirectory {
    systems: Vec<SystemBuilder>,
    types: Vec<TypeBuilder>,
    symbols: Vec<SymbolBuilder>,
    axioms: Vec<AxiomBuilder>,
    theorems: Vec<TheoremBuilder>,
    proofs: Vec<ProofBuilder>,

    tables: Vec<TableBuilder>,
    quotes: Vec<QuoteBuilder>,
    headings: Vec<HeadingBuilder>,
    todos: Vec<TodoBuilder>,
    texts: Vec<TextBlockBuilder>,

    bibliography: Option<BibliographyBuilder>,
    index: Option<BuilderIndex>,
}

impl BuilderDirectory {
    pub fn new() -> BuilderDirectory {
        BuilderDirectory {
            systems: Vec::new(),
            types: Vec::new(),
            symbols: Vec::new(),
            axioms: Vec::new(),
            theorems: Vec::new(),
            proofs: Vec::new(),

            tables: Vec::new(),
            quotes: Vec::new(),
            headings: Vec::new(),
            todos: Vec::new(),
            texts: Vec::new(),

            bibliography: None,
            index: None,
        }
    }

    pub fn add_system(&mut self, mut system: SystemBuilder) -> SystemBuilderRef {
        assert!(self.index.is_none());

        let system_ref = SystemBuilderRef(self.systems.len());
        system.set_self_ref(system_ref);

        self.systems.push(system);
        system_ref
    }

    pub fn add_type(&mut self, mut ty: TypeBuilder) -> TypeBuilderRef {
        assert!(self.index.is_none());

        let ty_ref = TypeBuilderRef(self.types.len());
        ty.set_self_ref(ty_ref);

        self.types.push(ty);
        ty_ref
    }

    pub fn add_symbol(&mut self, mut symbol: SymbolBuilder) -> SymbolBuilderRef {
        assert!(self.index.is_none());

        let symbol_ref = SymbolBuilderRef(self.symbols.len());
        symbol.set_self_ref(symbol_ref);

        self.symbols.push(symbol);
        symbol_ref
    }

    pub fn add_axiom(&mut self, mut axiom: AxiomBuilder) -> AxiomBuilderRef {
        assert!(self.index.is_none());

        let axiom_ref = AxiomBuilderRef(self.axioms.len());
        axiom.set_self_ref(axiom_ref);

        self.axioms.push(axiom);
        axiom_ref
    }

    pub fn add_theorem(&mut self, mut theorem: TheoremBuilder) -> TheoremBuilderRef {
        assert!(self.index.is_none());

        let theorem_ref = TheoremBuilderRef(self.theorems.len());
        theorem.set_self_ref(theorem_ref);

        self.theorems.push(theorem);
        theorem_ref
    }

    pub fn add_proof(&mut self, mut proof: ProofBuilder) -> ProofBuilderRef {
        assert!(self.index.is_none());

        let proof_ref = ProofBuilderRef(self.proofs.len());
        proof.set_self_ref(proof_ref);

        self.proofs.push(proof);
        proof_ref
    }

    pub fn add_table(&mut self, mut table: TableBuilder) -> TableBuilderRef {
        assert!(self.index.is_none());

        let table_ref = TableBuilderRef(self.tables.len());
        table.set_self_ref(table_ref);

        self.tables.push(table);
        table_ref
    }

    pub fn add_quote(&mut self, mut quote: QuoteBuilder) -> QuoteBuilderRef {
        assert!(self.index.is_none());

        let quote_ref = QuoteBuilderRef(self.quotes.len());
        quote.set_self_ref(quote_ref);

        self.quotes.push(quote);
        quote_ref
    }

    pub fn add_heading(&mut self, heading: HeadingBuilder) -> HeadingBuilderRef {
        assert!(self.index.is_none());
        self.headings.push(heading);
        HeadingBuilderRef(self.headings.len() - 1)
    }

    pub fn add_todo(&mut self, mut todo: TodoBuilder) -> TodoBuilderRef {
        assert!(self.index.is_none());

        let todo_ref = TodoBuilderRef(self.todos.len());
        todo.set_self_ref(todo_ref);

        self.todos.push(todo);
        todo_ref
    }

    pub fn add_text(&mut self, mut text: TextBlockBuilder) -> TextBlockBuilderRef {
        assert!(self.index.is_none());

        let text_ref = TextBlockBuilderRef(self.texts.len());
        text.set_self_ref(text_ref);

        self.texts.push(text);
        text_ref
    }

    pub fn set_bib(&mut self, bib: BibliographyBuilder) {
        assert!(self.bibliography.is_none());
        self.bibliography = Some(bib);
    }

    pub fn get_local(&self, system_id: &str) -> LocalIndex {
        let index = self.index.as_ref().unwrap();
        let parent_system = index.systems.get(system_id).unwrap();

        LocalIndex {
            parent_system,

            vars: HashMap::new(),
        }
    }

    pub fn build_index(&mut self, errors: &mut ParsingErrorContext) {
        assert!(self.index.is_none());
        let mut index = BuilderIndex::new();

        for (i, system) in self.systems.iter().enumerate() {
            let id = system.id();

            index.add_system(id, SystemBuilderRef(i), errors);
        }

        for (i, ty) in self.types.iter().enumerate() {
            let id = ty.id();
            let system_id = ty.system_id();

            index.add_system_child(
                system_id,
                id,
                SystemBuilderChild::Type(TypeBuilderRef(i)),
                errors,
            );
        }

        for (i, symbol) in self.symbols.iter().enumerate() {
            let id = symbol.id();
            let system_id = symbol.system_id();

            index.add_system_child(
                system_id,
                id,
                SystemBuilderChild::Symbol(SymbolBuilderRef(i)),
                errors,
            );
        }

        for (i, axiom) in self.axioms.iter().enumerate() {
            let id = axiom.id();
            let system_id = axiom.system_id();

            index.add_system_child(
                system_id,
                id,
                SystemBuilderChild::Axiom(AxiomBuilderRef(i)),
                errors,
            );
        }

        for (i, theorem) in self.theorems.iter().enumerate() {
            let id = theorem.id();
            let system_id = theorem.system_id();

            index.add_system_child(
                system_id,
                id,
                SystemBuilderChild::Theorem(TheoremBuilderRef(i)),
                errors,
            );
        }

        self.index = Some(index);

        self.bibliography
            .as_mut()
            .map(|bib| bib.build_index(errors));
    }

    pub fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        for system in &self.systems {
            system.verify_structure(&self, errors);
        }

        for ty in &self.types {
            ty.verify_structure(&self, errors);
        }

        for symbol in &self.symbols {
            symbol.verify_structure(&self, errors);
        }

        for axiom in &self.axioms {
            axiom.verify_structure(&self, errors);
        }

        for theorem in &self.theorems {
            theorem.verify_structure(&self, errors);
        }

        for proof in &self.proofs {
            proof.verify_structure(&self, errors);
        }

        for table in &self.tables {
            table.verify_structure(&self, errors);
        }

        for quote in &self.quotes {
            quote.verify_structure(&self, errors);
        }

        for todo in &self.todos {
            todo.verify_structure(&self, errors);
        }

        for text in &self.texts {
            text.verify_structure(&self, errors);
        }

        self.bibliography
            .as_ref()
            .map(|bib| bib.verify_structure(errors));
    }

    pub fn build_operators(&mut self, errors: &mut ParsingErrorContext) {
        for symbol in self.symbols.iter() {
            if let Some(read_signature) = symbol.read_signature() {
                let system_id = &symbol.system_id();
                let readable = symbol.as_readable();

                self.index.as_mut().unwrap().add_operator(
                    system_id,
                    read_signature,
                    readable,
                    errors,
                );
            }
        }
    }

    pub fn build_formulas(&self, errors: &mut ParsingErrorContext) {
        for axiom in &self.axioms {
            axiom.build_formulas(self, errors);
        }

        for theorem in &self.theorems {
            theorem.build_formulas(self, errors);
        }

        for proof in &self.proofs {
            proof.build_formulas(self, errors);
        }
    }

    pub fn finish(&self) -> BlockDirectory {
        let systems = self.systems.iter().map(SystemBuilder::finish).collect();
        let types = self.types.iter().map(TypeBuilder::finish).collect();
        let symbols = self.symbols.iter().map(SymbolBuilder::finish).collect();
        let axioms = self.axioms.iter().map(AxiomBuilder::finish).collect();
        let theorems = self.theorems.iter().map(TheoremBuilder::finish).collect();
        let proofs = self.proofs.iter().map(ProofBuilder::finish).collect();

        let tables = self.tables.iter().map(TableBuilder::finish).collect();
        let quotes = self.quotes.iter().map(QuoteBuilder::finish).collect();
        let headings = self.headings.iter().map(HeadingBuilder::finish).collect();
        let todos = self.todos.iter().map(TodoBuilder::finish).collect();
        let texts = self.texts.iter().map(TextBlockBuilder::finish).collect();

        let bibliography = self.bibliography.as_ref().map(BibliographyBuilder::finish);

        BlockDirectory::new(
            systems,
            types,
            symbols,
            axioms,
            theorems,
            proofs,
            tables,
            quotes,
            headings,
            todos,
            texts,
            bibliography,
        )
    }

    pub fn search_system(&self, id: &str) -> Option<SystemBuilderRef> {
        self.index.as_ref().unwrap().search_system(id)
    }

    pub fn search_system_child(
        &self,
        system_id: &str,
        child_id: &str,
    ) -> Option<SystemBuilderChild> {
        self.index
            .as_ref()
            .unwrap()
            .search_system_child(system_id, child_id)
    }

    pub fn search_bib_key(&self, bib_key: &str) -> Option<BibliographyBuilderRef> {
        self.bibliography
            .as_ref()
            .and_then(|bib| bib.search_key(bib_key))
    }
}

impl Index<SystemBuilderRef> for BuilderDirectory {
    type Output = SystemBuilder;

    fn index(&self, system_ref: SystemBuilderRef) -> &Self::Output {
        &self.systems[system_ref.0]
    }
}

impl Index<TypeBuilderRef> for BuilderDirectory {
    type Output = TypeBuilder;

    fn index(&self, type_ref: TypeBuilderRef) -> &Self::Output {
        &self.types[type_ref.0]
    }
}

impl Index<SymbolBuilderRef> for BuilderDirectory {
    type Output = SymbolBuilder;

    fn index(&self, symbol_ref: SymbolBuilderRef) -> &Self::Output {
        &self.symbols[symbol_ref.0]
    }
}

impl Index<AxiomBuilderRef> for BuilderDirectory {
    type Output = AxiomBuilder;

    fn index(&self, axiom_ref: AxiomBuilderRef) -> &Self::Output {
        &self.axioms[axiom_ref.0]
    }
}

impl Index<TheoremBuilderRef> for BuilderDirectory {
    type Output = TheoremBuilder;

    fn index(&self, theorem_ref: TheoremBuilderRef) -> &Self::Output {
        &self.theorems[theorem_ref.0]
    }
}

impl Index<ProofBuilderRef> for BuilderDirectory {
    type Output = ProofBuilder;

    fn index(&self, proof_ref: ProofBuilderRef) -> &Self::Output {
        &self.proofs[proof_ref.0]
    }
}

impl Index<TableBuilderRef> for BuilderDirectory {
    type Output = TableBuilder;

    fn index(&self, table_ref: TableBuilderRef) -> &Self::Output {
        &self.tables[table_ref.0]
    }
}

impl Index<QuoteBuilderRef> for BuilderDirectory {
    type Output = QuoteBuilder;

    fn index(&self, quote_ref: QuoteBuilderRef) -> &Self::Output {
        &self.quotes[quote_ref.0]
    }
}

impl Index<HeadingBuilderRef> for BuilderDirectory {
    type Output = HeadingBuilder;

    fn index(&self, heading_ref: HeadingBuilderRef) -> &Self::Output {
        &self.headings[heading_ref.0]
    }
}

impl Index<TodoBuilderRef> for BuilderDirectory {
    type Output = TodoBuilder;

    fn index(&self, todo_ref: TodoBuilderRef) -> &Self::Output {
        &self.todos[todo_ref.0]
    }
}

impl Index<TextBlockBuilderRef> for BuilderDirectory {
    type Output = TextBlockBuilder;

    fn index(&self, text_ref: TextBlockBuilderRef) -> &Self::Output {
        &self.texts[text_ref.0]
    }
}

pub struct BibliographyBuilderIndex {
    entries: HashMap<String, BibliographyBuilderRef>,
}

impl BibliographyBuilderIndex {
    fn new() -> BibliographyBuilderIndex {
        BibliographyBuilderIndex {
            entries: HashMap::new(),
        }
    }

    fn add_entry(
        &mut self,
        key: &str,
        bib_ref: BibliographyBuilderRef,
        errors: &mut ParsingErrorContext,
    ) {
        if let Some(old_ref) = self.entries.get(key) {
            errors.err(ParsingError::BibliographyError(
                BibliographyParsingError::KeyAlreadyTaken(bib_ref, *old_ref),
            ));
        } else {
            self.entries.insert(key.to_owned(), bib_ref);
        }
    }

    fn search_key(&self, key: &str) -> Option<BibliographyBuilderRef> {
        self.entries.get(key).copied()
    }
}

struct BibliographyBuilderEntry {
    key: String,
    mla: MlaBuilderEntries,

    self_ref: Option<BibliographyBuilderRef>,
}

impl BibliographyBuilderEntry {
    fn from_pest(pair: Pair<Rule>) -> BibliographyBuilderEntry {
        assert_eq!(pair.as_rule(), Rule::bib_entry);

        let mut inner = pair.into_inner();
        let key = inner.next().unwrap().as_str().to_owned();
        let mla = MlaBuilderEntries::from_pest(inner);

        BibliographyBuilderEntry {
            key,
            mla,

            self_ref: None,
        }
    }

    fn set_self_ref(&mut self, self_ref: BibliographyBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(self_ref);
    }

    fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        self.mla.verify_structure(errors, |e| {
            ParsingError::BibliographyError(BibliographyParsingError::MlaError(
                self.self_ref.unwrap(),
                e,
            ))
        });
    }

    fn finish(&self) -> Mla {
        self.mla.finish()
    }
}

pub struct BibliographyBuilder {
    entries: Vec<BibliographyBuilderEntry>,

    index: Option<BibliographyBuilderIndex>,
}

impl BibliographyBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> BibliographyBuilder {
        assert_eq!(pair.as_rule(), Rule::bib);

        let entries = pair
            .into_inner()
            .filter_map(|pair| match pair.as_rule() {
                Rule::bib_entry => Some(BibliographyBuilderEntry::from_pest(pair)),
                Rule::EOI => None,

                _ => unreachable!(),
            })
            .collect();

        BibliographyBuilder {
            entries,

            index: None,
        }
    }

    fn build_index(&mut self, errors: &mut ParsingErrorContext) {
        assert!(self.index.is_none());
        let mut index = BibliographyBuilderIndex::new();

        for (i, entry) in self.entries.iter_mut().enumerate() {
            let self_ref = BibliographyBuilderRef(i);
            entry.set_self_ref(self_ref);

            let key = &entry.key;
            index.add_entry(key, self_ref, errors);
        }

        self.index = Some(index)
    }

    fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        for entry in &self.entries {
            entry.verify_structure(errors);
        }
    }

    pub fn search_key(&self, key: &str) -> Option<BibliographyBuilderRef> {
        self.index.as_ref().unwrap().search_key(key)
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

pub struct LocalBibliographyBuilderIndex {
    map: HashMap<BibliographyBuilderRef, LocalBibliographyBuilderRef>,
}

impl LocalBibliographyBuilderIndex {
    fn new(entries: &[BibliographyBuilderRef]) -> LocalBibliographyBuilderIndex {
        let map = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| (*entry, LocalBibliographyBuilderRef(i)))
            .collect();

        LocalBibliographyBuilderIndex { map }
    }
}

impl Index<BibliographyBuilderRef> for LocalBibliographyBuilderIndex {
    type Output = LocalBibliographyBuilderRef;

    fn index(&self, bib_ref: BibliographyBuilderRef) -> &Self::Output {
        &self.map[&bib_ref]
    }
}

pub struct LocalBibliographyBuilder {
    entries: Vec<BibliographyBuilderRef>,
}

impl LocalBibliographyBuilder {
    pub fn new(
        page_blocks: &[BlockBuilder],
        directory: &BuilderDirectory,
    ) -> LocalBibliographyBuilder {
        let mut seen = HashSet::new();
        let entries: Vec<_> = page_blocks
            .iter()
            .flat_map(|block| block.bib_refs(directory))
            .filter_map(|bib_ref| {
                if seen.contains(&bib_ref) {
                    None
                } else {
                    seen.insert(bib_ref);
                    Some(bib_ref)
                }
            })
            .collect();

        let index = LocalBibliographyBuilderIndex::new(&entries);
        for block in page_blocks {
            block.set_local_bib_refs(directory, &index);
        }

        LocalBibliographyBuilder { entries }
    }

    pub fn finish(&self) -> LocalBibliography {
        let entries = self
            .entries
            .iter()
            .map(BibliographyBuilderRef::finish)
            .collect();

        LocalBibliography::new(entries)
    }
}
