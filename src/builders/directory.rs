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

use std::collections::HashMap;
use std::ops::Index;

use pest::iterators::Pair;

use crate::document::directory::{
    AxiomBlockRef, Block, BlockDirectory, HeadingBlockRef, ProofBlockRef, ProofBlockStepRef,
    SymbolBlockRef, SystemBlockRef, TableBlockRef, TextBlockRef, TheoremBlockRef, TodoBlockRef,
    TypeBlockRef, VariableBlockRef,
};

use super::deduction::{AxiomBuilder, ProofBuilder, TheoremBuilder};
use super::errors::{ParsingError, ParsingErrorContext};
use super::language::{
    ReadBuilder, SymbolBuilder, SystemBuilder, TypeBuilder, TypeSignatureBuilder, VariableBuilder,
};
use super::text::{HeadingBuilder, TableBuilder, TextBlockBuilder, TodoBuilder};
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
pub enum VariableBuilderParentRef {
    Axiom(AxiomBuilderRef),
    Theorem(TheoremBuilderRef),
    Proof(ProofBuilderRef),
}

impl From<AxiomBuilderRef> for VariableBuilderParentRef {
    fn from(axiom_ref: AxiomBuilderRef) -> VariableBuilderParentRef {
        VariableBuilderParentRef::Axiom(axiom_ref)
    }
}

impl From<TheoremBuilderRef> for VariableBuilderParentRef {
    fn from(theorem_ref: TheoremBuilderRef) -> VariableBuilderParentRef {
        VariableBuilderParentRef::Theorem(theorem_ref)
    }
}

impl From<ProofBuilderRef> for VariableBuilderParentRef {
    fn from(proof_ref: ProofBuilderRef) -> VariableBuilderParentRef {
        VariableBuilderParentRef::Proof(proof_ref)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct VariableBuilderRef(VariableBuilderParentRef, usize);

impl VariableBuilderRef {
    pub fn get(&self) -> usize {
        self.1
    }

    pub fn finish(&self) -> VariableBlockRef {
        VariableBlockRef::new(self.1)
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
    pub fn step(self, step: usize) -> ProofBuilderStepRef {
        ProofBuilderStepRef(self, step)
    }

    pub fn finish(&self) -> ProofBlockRef {
        ProofBlockRef::new(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ProofBuilderStepRef(ProofBuilderRef, usize);

impl ProofBuilderStepRef {
    pub fn parent_proof(&self) -> ProofBuilderRef {
        self.0
    }

    pub fn finish(&self) -> ProofBlockStepRef {
        ProofBlockStepRef::new(self.0.finish(), self.1)
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

    pub fn finish(&self) -> Block {
        match self {
            Self::Type(type_ref) => type_ref.finish().into(),
            Self::Symbol(symbol_ref) => symbol_ref.finish().into(),
            Self::Axiom(axiom_ref) => axiom_ref.finish().into(),
            Self::Theorem(theorem_ref) => theorem_ref.finish().into(),
        }
    }
}

pub enum BlockBuilder {
    System(SystemBuilderRef),
    Type(TypeBuilderRef),
    Symbol(SymbolBuilderRef),
    Axiom(AxiomBuilderRef),
    Theorem(TheoremBuilderRef),
    Proof(ProofBuilderRef),

    Table(TableBuilderRef),
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
                Self::Proof(directory.add_proof(ProofBuilder::from_pest(pair, href)))
            }

            Rule::table_block => Self::Table(directory.add_table(TableBuilder::from_pest(pair))),
            Rule::todo_block => Self::Todo(directory.add_todo(TodoBuilder::from_pest(pair))),
            Rule::heading_block => {
                Self::Heading(directory.add_heading(HeadingBuilder::from_pest(pair)))
            }
            Rule::text_block => Self::Text(directory.add_text(TextBlockBuilder::from_pest(pair))),

            _ => unreachable!(),
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
    pub fn add_vars(
        &mut self,
        parent: VariableBuilderParentRef,
        vars: &[VariableBuilder],
        errors: &mut ParsingErrorContext,
    ) {
        for (i, var) in vars.iter().enumerate() {
            if let Some(old_var) = self.vars.get(var.id()) {
                errors.err(ParsingError::VariableDuplicateId(
                    *old_var,
                    VariableBuilderRef(parent, i),
                ));
            } else {
                self.vars
                    .insert(var.id().to_owned(), VariableBuilderRef(parent, i));
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
            errors.err(ParsingError::SystemChildIdAlreadyTaken(
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
            errors.err(ParsingError::SystemReadSignatureAlreadyTaken(
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
            errors.err(ParsingError::SystemIdAlreadyTaken(
                system_ref,
                old_index.system_ref,
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
            None => errors.err(ParsingError::SystemChildParentIdNotFound(child_ref)),
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

    pub fn add_tag(
        &mut self,
        tag: &str,
        proof_step_ref: ProofBuilderStepRef,
        errors: &mut ParsingErrorContext,
    ) {
        if let Some(old_step) = self.tags.get(tag) {
            errors.err(ParsingError::ProofStepTagAlreadyTaken(
                proof_step_ref,
                *old_step,
            ));
        }

        self.tags.insert(tag.to_owned(), proof_step_ref);
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
    headings: Vec<HeadingBuilder>,
    todos: Vec<TodoBuilder>,
    texts: Vec<TextBlockBuilder>,

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
            headings: Vec::new(),
            todos: Vec::new(),
            texts: Vec::new(),

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

    pub fn add_table(&mut self, table: TableBuilder) -> TableBuilderRef {
        assert!(self.index.is_none());
        self.tables.push(table);
        TableBuilderRef(self.tables.len() - 1)
    }

    pub fn add_heading(&mut self, heading: HeadingBuilder) -> HeadingBuilderRef {
        assert!(self.index.is_none());
        self.headings.push(heading);
        HeadingBuilderRef(self.headings.len() - 1)
    }

    pub fn add_todo(&mut self, todo: TodoBuilder) -> TodoBuilderRef {
        assert!(self.index.is_none());
        self.todos.push(todo);
        TodoBuilderRef(self.todos.len() - 1)
    }

    pub fn add_text(&mut self, text: TextBlockBuilder) -> TextBlockBuilderRef {
        assert!(self.index.is_none());
        self.texts.push(text);
        TextBlockBuilderRef(self.texts.len() - 1)
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

        for todo in &self.todos {
            todo.verify_structure(&self, errors);
        }

        for text in &self.texts {
            text.verify_structure(&self, errors);
        }
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
        let headings = self.headings.iter().map(HeadingBuilder::finish).collect();
        let todos = self.todos.iter().map(TodoBuilder::finish).collect();
        let texts = self.texts.iter().map(TextBlockBuilder::finish).collect();

        BlockDirectory::new(
            systems, types, symbols, axioms, theorems, proofs, tables, headings, todos, texts,
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

impl Index<TheoremBuilderRef> for BuilderDirectory {
    type Output = TheoremBuilder;

    fn index(&self, theorem_ref: TheoremBuilderRef) -> &Self::Output {
        &self.theorems[theorem_ref.0]
    }
}
