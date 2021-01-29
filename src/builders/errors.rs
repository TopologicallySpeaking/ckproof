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
    AxiomBuilderRef, ProofBuilderRef, ProofBuilderStepRef, Readable, ReadableKind,
    SymbolBuilderRef, SystemBuilderChild, SystemBuilderRef, TheoremBuilderRef, TypeBuilderRef,
    VariableBuilderRef,
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
    IdAlreadyTaken(SystemBuilderChild),

    MissingName,
    MissingTagline,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,
}

#[derive(Debug)]
pub enum SymbolParsingError {
    ParentNotFound,
    IdAlreadyTaken(SystemBuilderChild),
    ReadSignatureAlreadyTaken(Readable),

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
    IdAlreadyTaken(SystemBuilderChild),

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
    IdAlreadyTaken(SystemBuilderChild),

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
}

impl ParsingError {
    pub fn system_child_parent_not_found(child_ref: SystemBuilderChild) -> ParsingError {
        match child_ref {
            SystemBuilderChild::Type(type_ref) => {
                ParsingError::TypeError(type_ref, TypeParsingError::ParentNotFound)
            }
            SystemBuilderChild::Symbol(symbol_ref) => {
                ParsingError::SymbolError(symbol_ref, SymbolParsingError::ParentNotFound)
            }
            SystemBuilderChild::Axiom(axiom_ref) => {
                ParsingError::AxiomError(axiom_ref, AxiomParsingError::ParentNotFound)
            }
            SystemBuilderChild::Theorem(axiom_ref) => {
                ParsingError::TheoremError(axiom_ref, TheoremParsingError::ParentNotFound)
            }
        }
    }

    pub fn system_child_id_already_taken(
        child_ref: SystemBuilderChild,
        old_ref: SystemBuilderChild,
    ) -> ParsingError {
        match child_ref {
            SystemBuilderChild::Type(type_ref) => {
                ParsingError::TypeError(type_ref, TypeParsingError::IdAlreadyTaken(old_ref))
            }
            SystemBuilderChild::Symbol(symbol_ref) => {
                ParsingError::SymbolError(symbol_ref, SymbolParsingError::IdAlreadyTaken(old_ref))
            }
            SystemBuilderChild::Axiom(axiom_ref) => {
                ParsingError::AxiomError(axiom_ref, AxiomParsingError::IdAlreadyTaken(old_ref))
            }
            SystemBuilderChild::Theorem(axiom_ref) => {
                ParsingError::TheoremError(axiom_ref, TheoremParsingError::IdAlreadyTaken(old_ref))
            }
        }
    }

    pub fn read_signature_already_taken(read: Readable, old_read: Readable) -> ParsingError {
        match read.kind() {
            ReadableKind::Symbol(symbol_ref) => ParsingError::SymbolError(
                symbol_ref,
                SymbolParsingError::ReadSignatureAlreadyTaken(old_read),
            ),
        }
    }
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
