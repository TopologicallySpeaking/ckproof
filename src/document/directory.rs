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

use std::ops::Index;

use crate::deduction::directory::CheckableDirectory;

use crate::rendered::BlockRendered;

use super::deduction::{AxiomBlock, ProofBlock, ProofBlockStep, TheoremBlock};
use super::language::{SymbolBlock, SystemBlock, TypeBlock};
use super::text::{HeadingBlock, TableBlock, TextBlock, TodoBlock};

#[derive(Clone, Copy, Debug)]
pub struct SystemBlockRef(usize);

impl SystemBlockRef {
    pub fn new(i: usize) -> SystemBlockRef {
        SystemBlockRef(i)
    }

    pub fn get(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TypeBlockRef(usize);

impl TypeBlockRef {
    pub fn new(i: usize) -> TypeBlockRef {
        TypeBlockRef(i)
    }

    pub fn get(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SymbolBlockRef(usize);

impl SymbolBlockRef {
    pub fn new(i: usize) -> SymbolBlockRef {
        SymbolBlockRef(i)
    }

    pub fn get(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VariableBlockRef(usize);

impl VariableBlockRef {
    pub fn new(i: usize) -> VariableBlockRef {
        VariableBlockRef(i)
    }

    pub fn get(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AxiomBlockRef(usize);

impl AxiomBlockRef {
    pub fn new(i: usize) -> AxiomBlockRef {
        AxiomBlockRef(i)
    }

    pub fn get(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TheoremBlockRef(usize);

impl TheoremBlockRef {
    pub fn new(i: usize) -> TheoremBlockRef {
        TheoremBlockRef(i)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProofBlockRef(usize);

impl ProofBlockRef {
    pub fn new(i: usize) -> ProofBlockRef {
        ProofBlockRef(i)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProofBlockStepRef(ProofBlockRef, usize);

impl ProofBlockStepRef {
    pub fn new(proof_ref: ProofBlockRef, i: usize) -> ProofBlockStepRef {
        ProofBlockStepRef(proof_ref, i)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TableBlockRef(usize);

impl TableBlockRef {
    pub fn new(i: usize) -> TableBlockRef {
        TableBlockRef(i)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HeadingBlockRef(usize);

impl HeadingBlockRef {
    pub fn new(i: usize) -> HeadingBlockRef {
        HeadingBlockRef(i)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TodoBlockRef(usize);

impl TodoBlockRef {
    pub fn new(i: usize) -> TodoBlockRef {
        TodoBlockRef(i)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextBlockRef(usize);

impl TextBlockRef {
    pub fn new(i: usize) -> TextBlockRef {
        TextBlockRef(i)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block {
    System(SystemBlockRef),
    Type(TypeBlockRef),
    Symbol(SymbolBlockRef),
    Axiom(AxiomBlockRef),
    Theorem(TheoremBlockRef),
    Proof(ProofBlockRef),
    ProofStep(ProofBlockStepRef),

    Table(TableBlockRef),
    Heading(HeadingBlockRef),
    Todo(TodoBlockRef),
    Text(TextBlockRef),
}

impl Block {
    pub fn render(&self, directory: &BlockDirectory) -> BlockRendered {
        match self {
            Self::System(system_ref) => {
                let system = directory[*system_ref].render(directory);
                BlockRendered::System(system)
            }

            Self::Type(type_ref) => {
                let ty = directory[*type_ref].render(directory);
                BlockRendered::Type(ty)
            }

            Self::Symbol(symbol_ref) => {
                let symbol = directory[*symbol_ref].render(directory);
                BlockRendered::Symbol(symbol)
            }

            Self::Axiom(axiom_ref) => {
                let axiom = directory[*axiom_ref].render(directory);
                BlockRendered::Axiom(axiom)
            }

            Self::Theorem(theorem_ref) => {
                let theorem = directory[*theorem_ref].render(directory);
                BlockRendered::Theorem(theorem)
            }

            Self::Proof(proof_ref) => {
                let proof = directory[*proof_ref].render(directory);
                BlockRendered::Proof(proof)
            }

            Self::ProofStep(_) => unreachable!(),

            Self::Table(table_ref) => {
                let table = directory[*table_ref].render(directory);
                BlockRendered::Table(table)
            }

            Self::Heading(heading_ref) => {
                let heading = directory[*heading_ref].render();
                BlockRendered::Heading(heading)
            }

            Self::Todo(todo_ref) => {
                let todo = directory[*todo_ref].render(directory);
                BlockRendered::Todo(todo)
            }

            Self::Text(text_ref) => {
                let text = directory[*text_ref].render(directory);
                BlockRendered::Text(text)
            }
        }
    }

    pub fn render_ref(&self, directory: &BlockDirectory) -> String {
        match self {
            Self::System(system_ref) => {
                let system = &directory[*system_ref];

                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    system.href(),
                    system.name()
                )
            }

            Self::Type(type_ref) => {
                let ty = &directory[*type_ref];

                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    ty.href(),
                    ty.name()
                )
            }

            Self::Symbol(symbol_ref) => {
                let symbol = &directory[*symbol_ref];

                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    symbol.href(),
                    symbol.name()
                )
            }

            Self::Axiom(axiom_ref) => {
                let axiom = &directory[*axiom_ref];

                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    axiom.href(),
                    axiom.name()
                )
            }

            Self::Theorem(theorem_ref) => {
                let theorem = &directory[*theorem_ref];

                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    theorem.href(),
                    theorem.name()
                )
            }

            Self::Proof(_) => todo!(),

            Self::ProofStep(proof_step_ref) => {
                let proof_step = &directory[*proof_step_ref];

                format!(
                    "<a href=\"{}\">({})</a>",
                    proof_step.href(),
                    proof_step_ref.1 + 1
                )
            }

            Self::Table(_) => todo!(),
            Self::Heading(_) => todo!(),
            Self::Todo(_) => todo!(),
            Self::Text(_) => todo!(),
        }
    }
}

impl From<SystemBlockRef> for Block {
    fn from(system_ref: SystemBlockRef) -> Block {
        Block::System(system_ref)
    }
}

impl From<TypeBlockRef> for Block {
    fn from(type_ref: TypeBlockRef) -> Block {
        Block::Type(type_ref)
    }
}

impl From<SymbolBlockRef> for Block {
    fn from(symbol_ref: SymbolBlockRef) -> Block {
        Block::Symbol(symbol_ref)
    }
}

impl From<AxiomBlockRef> for Block {
    fn from(axiom_ref: AxiomBlockRef) -> Block {
        Block::Axiom(axiom_ref)
    }
}

impl From<TheoremBlockRef> for Block {
    fn from(theorem_ref: TheoremBlockRef) -> Block {
        Block::Theorem(theorem_ref)
    }
}

impl From<ProofBlockRef> for Block {
    fn from(proof_ref: ProofBlockRef) -> Block {
        Block::Proof(proof_ref)
    }
}

impl From<ProofBlockStepRef> for Block {
    fn from(proof_ref: ProofBlockStepRef) -> Block {
        Block::ProofStep(proof_ref)
    }
}

impl From<TableBlockRef> for Block {
    fn from(table_ref: TableBlockRef) -> Block {
        Block::Table(table_ref)
    }
}

impl From<HeadingBlockRef> for Block {
    fn from(heading_ref: HeadingBlockRef) -> Block {
        Block::Heading(heading_ref)
    }
}

impl From<TodoBlockRef> for Block {
    fn from(todo_ref: TodoBlockRef) -> Block {
        Block::Todo(todo_ref)
    }
}

impl From<TextBlockRef> for Block {
    fn from(text_ref: TextBlockRef) -> Block {
        Block::Text(text_ref)
    }
}

pub struct BlockDirectory {
    systems: Vec<SystemBlock>,
    types: Vec<TypeBlock>,
    symbols: Vec<SymbolBlock>,
    axioms: Vec<AxiomBlock>,
    theorems: Vec<TheoremBlock>,
    proofs: Vec<ProofBlock>,

    tables: Vec<TableBlock>,
    headings: Vec<HeadingBlock>,
    todos: Vec<TodoBlock>,
    texts: Vec<TextBlock>,
}

impl BlockDirectory {
    pub fn new(
        systems: Vec<SystemBlock>,
        types: Vec<TypeBlock>,
        symbols: Vec<SymbolBlock>,
        axioms: Vec<AxiomBlock>,
        theorems: Vec<TheoremBlock>,
        proofs: Vec<ProofBlock>,
        tables: Vec<TableBlock>,
        headings: Vec<HeadingBlock>,
        todos: Vec<TodoBlock>,
        texts: Vec<TextBlock>,
    ) -> BlockDirectory {
        BlockDirectory {
            systems,
            types,
            symbols,
            axioms,
            theorems,
            proofs,

            tables,
            headings,
            todos,
            texts,
        }
    }

    pub fn checkable(&self) -> CheckableDirectory {
        let systems = self.systems.iter().map(SystemBlock::checkable).collect();
        let types = self.types.iter().map(TypeBlock::checkable).collect();
        let symbols = self.symbols.iter().map(SymbolBlock::checkable).collect();

        let mut directory = CheckableDirectory::new(systems, types, symbols);

        let axioms = self
            .axioms
            .iter()
            .map(|axiom| axiom.checkable(&directory))
            .collect();
        directory.set_axioms(axioms);

        let theorems = self
            .theorems
            .iter()
            .map(|theorem| theorem.checkable(&directory))
            .collect();
        directory.set_theorems(theorems);

        let proofs = self
            .proofs
            .iter()
            .map(|proof| proof.checkable(&self, &directory))
            .collect();
        directory.set_proofs(proofs);

        directory
    }

    pub fn todos(&self) -> &[TodoBlock] {
        &self.todos
    }
}

impl Index<SystemBlockRef> for BlockDirectory {
    type Output = SystemBlock;

    fn index(&self, system_ref: SystemBlockRef) -> &Self::Output {
        &self.systems[system_ref.0]
    }
}

impl Index<TypeBlockRef> for BlockDirectory {
    type Output = TypeBlock;

    fn index(&self, type_ref: TypeBlockRef) -> &Self::Output {
        &self.types[type_ref.0]
    }
}

impl Index<SymbolBlockRef> for BlockDirectory {
    type Output = SymbolBlock;

    fn index(&self, symbol_ref: SymbolBlockRef) -> &Self::Output {
        &self.symbols[symbol_ref.0]
    }
}

impl Index<AxiomBlockRef> for BlockDirectory {
    type Output = AxiomBlock;

    fn index(&self, axiom_ref: AxiomBlockRef) -> &Self::Output {
        &self.axioms[axiom_ref.0]
    }
}

impl Index<TheoremBlockRef> for BlockDirectory {
    type Output = TheoremBlock;

    fn index(&self, theorem_ref: TheoremBlockRef) -> &Self::Output {
        &self.theorems[theorem_ref.0]
    }
}

impl Index<ProofBlockRef> for BlockDirectory {
    type Output = ProofBlock;

    fn index(&self, proof_ref: ProofBlockRef) -> &Self::Output {
        &self.proofs[proof_ref.0]
    }
}

impl Index<ProofBlockStepRef> for BlockDirectory {
    type Output = ProofBlockStep;

    fn index(&self, proof_step_ref: ProofBlockStepRef) -> &Self::Output {
        &self[proof_step_ref.0].step(proof_step_ref.1)
    }
}

impl Index<TableBlockRef> for BlockDirectory {
    type Output = TableBlock;

    fn index(&self, table_ref: TableBlockRef) -> &Self::Output {
        &self.tables[table_ref.0]
    }
}

impl Index<HeadingBlockRef> for BlockDirectory {
    type Output = HeadingBlock;

    fn index(&self, heading_ref: HeadingBlockRef) -> &Self::Output {
        &self.headings[heading_ref.0]
    }
}

impl Index<TodoBlockRef> for BlockDirectory {
    type Output = TodoBlock;

    fn index(&self, todo_ref: TodoBlockRef) -> &Self::Output {
        &self.todos[todo_ref.0]
    }
}

impl Index<TextBlockRef> for BlockDirectory {
    type Output = TextBlock;

    fn index(&self, text_ref: TextBlockRef) -> &Self::Output {
        &self.texts[text_ref.0]
    }
}
