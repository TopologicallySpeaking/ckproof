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

use super::errors::{CheckingError, CheckingErrorContext, LocalCheckingError};
use super::language::{Definition, Symbol, Type, Variable};
use super::system::{Axiom, Proof, System, Theorem};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SystemRef(pub(super) usize);

impl SystemRef {
    pub fn new(i: usize) -> SystemRef {
        SystemRef(i)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TypeRef(pub(super) usize);

impl TypeRef {
    pub fn new(i: usize) -> TypeRef {
        TypeRef(i)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SymbolRef(pub(super) usize);

impl SymbolRef {
    pub fn new(i: usize) -> SymbolRef {
        SymbolRef(i)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DefinitionRef(pub(super) usize);

impl DefinitionRef {
    pub fn new(i: usize) -> DefinitionRef {
        DefinitionRef(i)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct VariableRef(pub(super) usize);

impl VariableRef {
    pub fn new(i: usize) -> VariableRef {
        VariableRef(i)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AxiomRef(pub(super) usize);

impl AxiomRef {
    pub fn new(i: usize) -> AxiomRef {
        AxiomRef(i)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TheoremRef(pub(super) usize);

impl TheoremRef {
    pub fn new(i: usize) -> TheoremRef {
        TheoremRef(i)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HypothesisRef(pub(super) usize);

impl HypothesisRef {
    pub fn new(i: usize) -> HypothesisRef {
        HypothesisRef(i)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProofStepRef(pub(super) usize);

impl ProofStepRef {
    pub fn new(i: usize) -> ProofStepRef {
        ProofStepRef(i)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProofRef(pub(super) usize);

impl ProofRef {
    pub fn new(i: usize) -> ProofRef {
        ProofRef(i)
    }
}

pub struct LocalCheckableDirectory {
    vars: Vec<Variable>,
}

impl LocalCheckableDirectory {
    pub fn new(vars: Vec<Variable>) -> LocalCheckableDirectory {
        LocalCheckableDirectory { vars }
    }

    pub(super) fn verify<F>(
        &self,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(LocalCheckingError) -> CheckingError,
    {
        for (i, var) in self.vars.iter().enumerate() {
            var.verify(directory, errors, |e| {
                generate_error(LocalCheckingError::VariableError(VariableRef(i), e))
            });
        }
    }

    pub(super) fn search_variable(&self, variable_ref: VariableRef) -> Option<&Variable> {
        self.vars.get(variable_ref.0)
    }

    pub(super) fn contains_variable(&self, variable_ref: VariableRef) -> bool {
        self.search_variable(variable_ref).is_some()
    }
}

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

    fn verify(&self, errors: &mut CheckingErrorContext) {
        for (i, ty) in self.types.iter().enumerate() {
            ty.verify(self, errors, |e| CheckingError::TypeError(TypeRef(i), e));
        }

        for (i, symbol) in self.symbols.iter().enumerate() {
            symbol.verify(self, errors, |e| {
                CheckingError::SymbolError(SymbolRef(i), e)
            });
        }

        for (i, definition) in self.definitions.iter().enumerate() {
            definition.verify(self, errors, |e| {
                CheckingError::DefinitionError(DefinitionRef(i), e)
            });
        }

        for (i, axiom) in self.axioms.iter().enumerate() {
            axiom.verify(self, errors, |e| CheckingError::AxiomError(AxiomRef(i), e));
        }

        for (i, theorem) in self.theorems.iter().enumerate() {
            theorem.verify(self, errors, |e| {
                CheckingError::TheoremError(TheoremRef(i), e)
            });
        }

        for (i, proof) in self.proofs.iter().enumerate() {
            proof.verify(self, errors, |e| CheckingError::ProofError(ProofRef(i), e));
        }
    }

    pub fn check(&self) -> CheckingErrorContext {
        let mut errors = CheckingErrorContext::new();

        self.verify(&mut errors);
        if errors.error_found() {
            return errors;
        }

        for (i, proof) in self.proofs.iter().enumerate() {
            proof.check(self, &mut errors, |e| {
                CheckingError::ProofError(ProofRef(i), e)
            });
        }

        errors
    }

    pub(super) fn search_system(&self, system_ref: SystemRef) -> Option<&System> {
        self.systems.get(system_ref.0)
    }

    pub(super) fn contains_system(&self, system_ref: SystemRef) -> bool {
        self.search_system(system_ref).is_some()
    }

    pub(super) fn search_type(&self, type_ref: TypeRef) -> Option<&Type> {
        self.types.get(type_ref.0)
    }

    pub(super) fn contains_type(&self, type_ref: TypeRef) -> bool {
        self.search_type(type_ref).is_some()
    }

    pub(super) fn search_symbol(&self, symbol_ref: SymbolRef) -> Option<&Symbol> {
        self.symbols.get(symbol_ref.0)
    }

    pub(super) fn contains_symbol(&self, symbol_ref: SymbolRef) -> bool {
        self.search_symbol(symbol_ref).is_some()
    }

    pub(super) fn search_definition(&self, definition_ref: DefinitionRef) -> Option<&Definition> {
        self.definitions.get(definition_ref.0)
    }

    pub(super) fn contains_definition(&self, definition_ref: DefinitionRef) -> bool {
        self.search_definition(definition_ref).is_some()
    }

    pub(super) fn search_axiom(&self, axiom_ref: AxiomRef) -> Option<&Axiom> {
        self.axioms.get(axiom_ref.0)
    }

    pub(super) fn contains_axiom(&self, axiom_ref: AxiomRef) -> bool {
        self.search_axiom(axiom_ref).is_some()
    }

    pub(super) fn search_theorem(&self, theorem_ref: TheoremRef) -> Option<&Theorem> {
        self.theorems.get(theorem_ref.0)
    }

    pub(super) fn contains_theorem(&self, theorem_ref: TheoremRef) -> bool {
        self.search_theorem(theorem_ref).is_some()
    }
}

impl Index<DefinitionRef> for CheckableDirectory {
    type Output = Definition;

    fn index(&self, definition_ref: DefinitionRef) -> &Self::Output {
        &self.definitions[definition_ref.0]
    }
}

impl Index<AxiomRef> for CheckableDirectory {
    type Output = Axiom;

    fn index(&self, axiom_ref: AxiomRef) -> &Self::Output {
        &self.axioms[axiom_ref.0]
    }
}

impl Index<TheoremRef> for CheckableDirectory {
    type Output = Theorem;

    fn index(&self, theorem_ref: TheoremRef) -> &Self::Output {
        &self.theorems[theorem_ref.0]
    }
}
