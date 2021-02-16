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

use std::ops::Index;

use super::errors::CheckerError;
use super::{
    Axiom, AxiomRef, Definition, DefinitionRef, Proof, Symbol, SymbolRef, System, Theorem,
    TheoremRef, Type, Variable, VariableRef,
};

pub struct CheckableDirectory {
    systems: Vec<System>,
    types: Vec<Type>,
    symbols: Vec<Symbol>,
    definitions: Vec<Definition>,

    axioms: Vec<Axiom>,
    theorems: Vec<Theorem>,
    proofs: Vec<Proof>,
}

impl CheckableDirectory {
    pub fn new(
        systems: Vec<System>,
        types: Vec<Type>,
        symbols: Vec<Symbol>,
        definitions: Vec<Definition>,
        axioms: Vec<Axiom>,
        theorems: Vec<Theorem>,
        proofs: Vec<Proof>,
    ) -> CheckableDirectory {
        CheckableDirectory {
            systems,
            types,
            symbols,
            definitions,

            axioms,
            theorems,
            proofs,
        }
    }

    pub fn check(&self) -> Vec<CheckerError> {
        self.proofs
            .iter()
            .filter_map(|proof| proof.check(&self))
            .collect()
    }
}

impl Index<&SymbolRef> for CheckableDirectory {
    type Output = Symbol;

    fn index(&self, symbol_ref: &SymbolRef) -> &Self::Output {
        &self.symbols[symbol_ref.0]
    }
}

impl Index<&DefinitionRef> for CheckableDirectory {
    type Output = Definition;

    fn index(&self, definition_ref: &DefinitionRef) -> &Self::Output {
        &self.definitions[definition_ref.0]
    }
}

impl Index<&AxiomRef> for CheckableDirectory {
    type Output = Axiom;

    fn index(&self, axiom_ref: &AxiomRef) -> &Self::Output {
        &self.axioms[axiom_ref.0]
    }
}

impl Index<&TheoremRef> for CheckableDirectory {
    type Output = Theorem;

    fn index(&self, theorem_ref: &TheoremRef) -> &Self::Output {
        &self.theorems[theorem_ref.0]
    }
}

pub struct LocalCheckableDirectory {
    variables: Vec<Variable>,
}

impl LocalCheckableDirectory {
    pub fn new(variables: Vec<Variable>) -> LocalCheckableDirectory {
        LocalCheckableDirectory { variables }
    }

    pub fn vars(&self) -> &[Variable] {
        &self.variables
    }
}

// TODO: Make all the Index traits in this file use the reference, not a reference to the
// reference.
impl Index<&VariableRef> for LocalCheckableDirectory {
    type Output = Variable;

    fn index(&self, variable_ref: &VariableRef) -> &Self::Output {
        &self.variables[variable_ref.0]
    }
}
