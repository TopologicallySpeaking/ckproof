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

use pest::error::Error as PestError;
use std::io::Error as IoError;
use url::ParseError as UrlError;

use super::directory::{
    AxiomBuilderRef, ProofBuilderRef, ProofBuilderStepRef, Readable, SymbolBuilderRef,
    SystemBuilderChild, SystemBuilderRef, TheoremBuilderRef, TypeBuilderRef, VariableBuilderRef,
};
use super::Rule;

#[derive(Debug)]
pub enum BuilderCreationError {
    IoError(IoError),
    PestError(PestError<Rule>),
}

impl From<IoError> for BuilderCreationError {
    fn from(e: IoError) -> BuilderCreationError {
        BuilderCreationError::IoError(e)
    }
}

impl From<PestError<Rule>> for BuilderCreationError {
    fn from(e: PestError<Rule>) -> BuilderCreationError {
        BuilderCreationError::PestError(e)
    }
}

#[derive(Debug)]
pub enum SystemParsingError {
    IdAlreadyTaken(SystemBuilderRef),

    MissingName,
    MissingTagline,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,
}

#[derive(Debug)]
pub enum TypeParsingError {
    ParentNotFound,
    MissingName,
    MissingTagline,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,
}

#[derive(Debug)]
pub enum SymbolParsingError {
    ParentNotFound,
    MissingName,
    MissingTagline,
    MissingTypeSignature,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,
    DuplicateTypeSignature,
    DuplicateReads,
    DuplicateDisplays,
}

#[derive(Debug)]
pub enum VariableParsingError {
    IdAlreadyTaken(VariableBuilderRef),
}

#[derive(Debug)]
pub enum AxiomParsingError {
    ParentNotFound,
    MissingName,
    MissingTagline,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,

    VariableError(VariableBuilderRef, VariableParsingError),
}

#[derive(Debug)]
pub enum TheoremParsingError {
    ParentNotFound,
    MissingName,
    MissingTagline,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,

    VariableError(VariableBuilderRef, VariableParsingError),
}

#[derive(Debug)]
pub enum ProofStepParsingError {
    TagAlreadyTaken(ProofBuilderStepRef),
    MissingJustification,
    DuplicateJustification,
    DuplicateTags,

    SystemChildJustificationNotFound,
    SystemChildJustificationWrongKind,
}

#[derive(Debug)]
pub enum ProofParsingError {
    ParentNotFound,
    ParentNotTheorem,

    VariableError(VariableBuilderRef, VariableParsingError),
    StepError(ProofBuilderStepRef, ProofStepParsingError),
}

#[derive(Debug)]
pub enum ParsingError {
    IoError(IoError),
    PestError(PestError<Rule>),
    UrlError(UrlError),

    SystemError(SystemBuilderRef, SystemParsingError),
    TypeError(TypeBuilderRef, TypeParsingError),
    SymbolError(SymbolBuilderRef, SymbolParsingError),
    AxiomError(AxiomBuilderRef, AxiomParsingError),
    TheoremError(TheoremBuilderRef, TheoremParsingError),
    ProofError(ProofBuilderRef, ProofParsingError),

    SystemChildIdAlreadyTaken(SystemBuilderChild, SystemBuilderChild),
    SystemChildParentIdNotFound(SystemBuilderChild),
    SystemReadSignatureAlreadyTaken(Readable, Readable),
}

impl From<IoError> for ParsingError {
    fn from(e: IoError) -> ParsingError {
        ParsingError::IoError(e)
    }
}

impl From<PestError<Rule>> for ParsingError {
    fn from(e: PestError<Rule>) -> ParsingError {
        ParsingError::PestError(e)
    }
}

impl From<UrlError> for ParsingError {
    fn from(e: UrlError) -> ParsingError {
        ParsingError::UrlError(e)
    }
}

#[derive(Debug)]
pub enum ParsingWarning {}

#[derive(Debug)]
pub struct ParsingErrorContext {
    warnings: Vec<ParsingWarning>,
    errors: Vec<ParsingError>,
}

impl ParsingErrorContext {
    pub fn new() -> ParsingErrorContext {
        ParsingErrorContext {
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn err<E: Into<ParsingError>>(&mut self, e: E) {
        self.errors.push(e.into());
    }

    pub fn error_found(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl Default for ParsingErrorContext {
    fn default() -> Self {
        Self::new()
    }
}
