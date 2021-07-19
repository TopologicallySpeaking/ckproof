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

use super::directory::{
    AxiomRef, DefinitionRef, HypothesisRef, ProofRef, ProofStepRef, SymbolRef, TheoremRef, TypeRef,
    VariableRef,
};

#[derive(Debug)]
pub enum TypeCheckingError {
    InvalidSystemRef,
}

#[derive(Debug)]
pub enum TypeSignatureCheckingError {
    InvalidTypeRef(TypeRef),
}

#[derive(Debug)]
pub enum SymbolCheckingError {
    InvalidSystemRef,
    TypeSignatureError(TypeSignatureCheckingError),
}

#[derive(Debug)]
pub enum DefinitionCheckingError {
    InvalidSystemRef,
    LocalError(LocalCheckingError),
    TypeSignatureError(TypeSignatureCheckingError),
    ExpandedError(FormulaCheckingError),
}

#[derive(Debug)]
pub enum VariableCheckingError {
    TypeSignatureError(TypeSignatureCheckingError),
}

#[derive(Debug)]
pub enum FormulaCheckingError {
    InvalidSymbolRef(SymbolRef),
    InvalidVariableRef(VariableRef),
    InvalidDefinitionRef(DefinitionRef),

    DefinitionWrongArity(DefinitionRef),
}

#[derive(Debug)]
pub enum AxiomCheckingError {
    InvalidSystemRef,
    LocalError(LocalCheckingError),
    PremiseError(HypothesisRef, FormulaCheckingError),
    AssertionError(FormulaCheckingError),
}

#[derive(Debug)]
pub enum TheoremCheckingError {
    InvalidSystemRef,
    LocalError(LocalCheckingError),
    PremiseError(HypothesisRef, FormulaCheckingError),
    AssertionError(FormulaCheckingError),
}

#[derive(Debug)]
pub enum ProofStepCheckingError {
    InvalidAxiomRef,
    InvalidTheoremRef,
    InvalidHypothesisRef,
    FormulaError(FormulaCheckingError),

    AxiomNotSubstitutable,
    AxiomAssertionNotSubstitutable,

    TheoremNotSubstitutable,
    TheoremAssertionNotSubstitutable,

    HypothesisMismatch,

    DefinitionMismatch,
}

#[derive(Debug)]
pub enum ProofCheckingError {
    InvalidTheoremRef,
    StepError(ProofStepRef, ProofStepCheckingError),

    Empty,
    AssertionMismatch,
}

#[derive(Debug)]
pub enum LocalCheckingError {
    VariableError(VariableRef, VariableCheckingError),
}

#[derive(Debug)]
pub enum CheckingError {
    TypeError(TypeRef, TypeCheckingError),
    SymbolError(SymbolRef, SymbolCheckingError),
    DefinitionError(DefinitionRef, DefinitionCheckingError),
    AxiomError(AxiomRef, AxiomCheckingError),
    TheoremError(TheoremRef, TheoremCheckingError),
    ProofError(ProofRef, ProofCheckingError),
}

#[derive(Debug)]
pub struct CheckingErrorContext {
    errors: Vec<CheckingError>,
}

impl CheckingErrorContext {
    pub fn new() -> CheckingErrorContext {
        CheckingErrorContext { errors: Vec::new() }
    }

    pub(super) fn err(&mut self, error: CheckingError) {
        self.errors.push(error);
    }

    pub fn error_found(&self) -> bool {
        !self.errors.is_empty()
    }
}
