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

use pest::error::Error as PestError;
use std::io::Error as IoError;
use url::ParseError as UrlError;

use crate::eprint;

use super::bibliography::BibliographyBuilderEntry;
use super::language::{
    DefinitionBuilder, DisplayFormulaBuilder, FormulaBuilder, ReadableBuilder, SymbolBuilder,
    TypeBuilder, TypeSignatureBuilderGround, VariableBuilder,
};
use super::structure::{BookBuilder, ChapterBuilder};
use super::system::{
    AxiomBuilder, DeductableBuilder, Flag, ProofBuilder, ProofBuilderStep, SystemBuilder,
    SystemBuilderChild, TheoremBuilder,
};
use super::text::{
    ListBuilder, MathBuilderElement, ParagraphBuilder, ParagraphBuilderElement, QuoteBuilder,
    RawCitationContainerBuilder, SystemChildReferenceBuilder, SystemReferenceBuilder, TableBuilder,
    TextBuilder,
};
use super::Rule;

#[derive(Debug)]
pub enum MathParsingError<'a> {
    SquareRootWrongInputArity(&'a MathBuilderElement),
    PowerWrongInputArity(&'a MathBuilderElement),
}

#[derive(Debug)]
pub enum ParagraphElementParsingError<'a> {
    MathError(MathParsingError<'a>),

    SystemReferenceIdNotFound,
    SystemChildReferenceIdNotFound,
    TagReferenceNotFound,
    CitationKeyNotFound,

    UnexpectedUnicornVomitBegin,
    UnexpectedUnicornVomitEnd,
    UnexpectedEmBegin,
    UnexpectedEmEnd,
}

impl<'a> ParagraphElementParsingError<'a> {
    fn eprint_system_reference_id_not_found(builder: &SystemReferenceBuilder) {
        let message = format!(
            "A System Reference contains the id `{}`, but this doesn't correspond to any known system.",
            builder.id(),
        );

        eprint(&message, builder.file_location());
    }

    fn eprint_system_child_reference_id_not_found(builder: &SystemChildReferenceBuilder) {
        // TODO: Indicate whether it's the system or the child which was not found.
        let message = format!(
            "A System Child Reference contains the id `{}.{}`, but this doesn't correspond to any known system child.",
            builder.system_id(),
            builder.child_id()
        );

        eprint(&message, builder.file_location());
    }

    fn eprint(&self, builder: &'a ParagraphBuilderElement<'a>) {
        match self {
            Self::SystemReferenceIdNotFound => {
                Self::eprint_system_reference_id_not_found(builder.system_reference().unwrap());
            }
            Self::SystemChildReferenceIdNotFound => {
                Self::eprint_system_child_reference_id_not_found(
                    builder.system_child_reference().unwrap(),
                );
            }

            _ => todo!("{:#?}", self),
        }
    }
}

#[derive(Debug)]
pub enum ParagraphParsingError<'a> {
    // TODO: Reference directory to the element instead of using its index.
    ElementError(usize, ParagraphElementParsingError<'a>),

    UnclosedUnicornVomit,
    UnclosedEm,
}

impl<'a> ParagraphParsingError<'a> {
    fn eprint(&self, builder: &'a ParagraphBuilder<'a>) {
        match self {
            Self::ElementError(i, error) => error.eprint(builder.get_element(*i)),

            _ => todo!("{:#?}", self),
        }
    }
}

#[derive(Debug)]
pub enum RawCitationContainerParsingError {
    DuplicateTitle,
    DuplicateOtherContributors,
    DuplicateVersion,
    DuplicateNumber,
    DuplicatePublisher,
    DuplicatePublicationDate,
    DuplicateLocation,
}

#[derive(Debug)]
pub enum RawCitationParsingError<'a> {
    MissingTitle,
    DuplicateAuthor,
    DuplicateTitle,

    ContainerError(
        &'a RawCitationContainerBuilder,
        RawCitationContainerParsingError,
    ),
}

#[derive(Debug)]
pub enum TextParsingError<'a> {
    RawCitationError(RawCitationParsingError<'a>),
    SublistError(MathParsingError<'a>),
    ParagraphError(ParagraphParsingError<'a>),
    DisplayMathError(MathParsingError<'a>),
}

impl<'a> TextParsingError<'a> {
    fn eprint(&self, builder: &'a TextBuilder<'a>) {
        match self {
            Self::ParagraphError(error) => error.eprint(builder.paragraph().unwrap()),

            _ => todo!("{:#?}", self),
        }
    }
}

#[derive(Debug)]
pub enum BookParsingError<'a> {
    TaglineError(ParagraphParsingError<'a>),
}

#[derive(Debug)]
pub enum ChapterParsingError<'a> {
    TaglineError(ParagraphParsingError<'a>),
}

#[derive(Debug)]
pub enum BibliographyParsingError<'a> {
    KeyAlreadyTaken(&'a BibliographyBuilderEntry),
    RawCitationError(RawCitationParsingError<'a>),
}

#[derive(Debug)]
pub enum SystemParsingError<'a> {
    IdAlreadyTaken(&'a SystemBuilder<'a>),

    MissingName,
    MissingTagline,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,

    TaglineParsingError(ParagraphParsingError<'a>),
    DescriptionParsingError(&'a TextBuilder<'a>, TextParsingError<'a>),
}

#[derive(Debug)]
pub enum SystemChildParsingError<'a> {
    ParentNotFound,
    IdAlreadyTaken(SystemBuilderChild<'a>),
}

#[derive(Debug)]
pub enum ReadableParsingError<'a> {
    IdAlreadyTaken(ReadableBuilder<'a>),
    DuplicateReflexive(DeductableBuilder<'a>),
    DuplicateSymmetric(DeductableBuilder<'a>),
    DuplicateTransitive(DeductableBuilder<'a>),
    DuplicateFunction(ReadableBuilder<'a>, DeductableBuilder<'a>),
}

#[derive(Debug)]
pub enum TypeParsingError<'a> {
    MissingName,
    MissingTagline,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,

    TaglineParsingError(ParagraphParsingError<'a>),
    DescriptionParsingError(&'a TextBuilder<'a>, TextParsingError<'a>),
}

#[derive(Debug)]
pub enum TypeSignatureParsingError<'a> {
    TypeIdNotFound(&'a TypeSignatureBuilderGround<'a>),
    SystemChildWrongKind(&'a TypeSignatureBuilderGround<'a>),
    ForwardReference(&'a TypeSignatureBuilderGround<'a>),
}

#[derive(Debug)]
pub enum SymbolParsingError<'a> {
    MissingName,
    MissingTagline,
    MissingTypeSignature,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,
    DuplicateTypeSignature,
    DuplicateReads,
    DuplicateDisplays,

    TaglineParsingError(ParagraphParsingError<'a>),
    DescriptionParsingError(&'a TextBuilder<'a>, TextParsingError<'a>),
    TypeSignatureError(TypeSignatureParsingError<'a>),
}

#[derive(Debug)]
pub enum DefinitionParsingError<'a> {
    MissingName,
    MissingTagline,
    MissingExpansion,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,
    DuplicateInputs,
    DuplicateReads,
    DuplicateDisplays,
    DuplicateExpansion,

    TaglineParsingError(ParagraphParsingError<'a>),
    DescriptionParsingError(&'a TextBuilder<'a>, TextParsingError<'a>),

    VariableError(&'a VariableBuilder<'a>, VariableParsingError<'a>),
    FormulaError(&'a FormulaBuilder<'a>, FormulaParsingError),
}

#[derive(Debug)]
pub enum VariableParsingError<'a> {
    TypeSignatureError(TypeSignatureParsingError<'a>),
}

#[derive(Debug)]
pub enum FormulaParsingError {
    VariableIdNotFound,
    OperatorNotFound,
}

#[derive(Debug)]
pub enum FlagListParsingError<'a> {
    DuplicateFlag(Flag),

    ReflexivityPremiseNotEmpty,
    ReflexivityAssertionNotBinary,
    ReflexivityArgumentMismatch,

    SymmetryPremiseWrongLength,
    SymmetryPremiseNotBinary,
    SymmetryAssertionNotBinary,
    SymmetrySymbolMismatch,
    SymmetryArgumentMismatch,

    TransitivityWrongPremiseLength,
    TransitivityFirstPremiseNotBinary,
    TransitivitySecondPremiseNotBinary,
    TransitivityPremiseSymbolNotEqual,
    TransitivityPremiseArgumentMismatch,
    TransitivityAssertionNotBinary,
    TransitivityAssertionSymbolNotEqual,
    TransitivityAssertionLeftMismatch,
    TransitivityAssertionRightMismatch,

    FunctionPremiseEmpty,
    FunctionPremiseNotBinary(&'a DisplayFormulaBuilder<'a>),
    FunctionPremiseArityMismatch,
    FunctionHypothesisNotBinary(&'a DisplayFormulaBuilder<'a>),
    FunctionHypothesisRelationMismatch(&'a DisplayFormulaBuilder<'a>),
    FunctionHypothesisLeftVarMismatch(&'a DisplayFormulaBuilder<'a>),
    FunctionHypothesisRightVarMismatch(&'a DisplayFormulaBuilder<'a>),
    FunctionAssertionNotBinary,
    FunctionAssertionLeftNotApplication,
    FunctionAssertionRightNotApplication,
    FunctionAssertionSymbolMismatch,
    FunctionAssertionArityMismatch,
    FunctionAssertionInputNotVariable(&'a FormulaBuilder<'a>),
    FunctionRelationNotPreorder,
}

#[derive(Debug)]
pub enum AxiomParsingError<'a> {
    MissingName,
    MissingTagline,
    MissingAssertion,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,
    DuplicateFlagList,
    DuplicatePremise,
    DuplicateAssertion,

    TaglineParsingError(ParagraphParsingError<'a>),
    DescriptionParsingError(&'a TextBuilder<'a>, TextParsingError<'a>),
    FlagListError(FlagListParsingError<'a>),

    VariableError(&'a VariableBuilder<'a>, VariableParsingError<'a>),
    FormulaError(&'a FormulaBuilder<'a>, FormulaParsingError),
}

#[derive(Debug)]
pub enum TheoremParsingError<'a> {
    MissingName,
    MissingTagline,
    MissingAssertion,
    DuplicateName,
    DuplicateTagline,
    DuplicateDescription,
    DuplicateFlagList,
    DuplicatePremise,
    DuplicateAssertion,

    TaglineParsingError(ParagraphParsingError<'a>),
    DescriptionParsingError(&'a TextBuilder<'a>, TextParsingError<'a>),
    FlagListError(FlagListParsingError<'a>),

    VariableError(&'a VariableBuilder<'a>, VariableParsingError<'a>),
    FormulaError(&'a FormulaBuilder<'a>, FormulaParsingError),
}

#[derive(Debug)]
pub enum ProofStepParsingError<'a> {
    TagAlreadyTaken(&'a ProofBuilderStep<'a>),

    MissingJustification,
    DuplicateTags,
    DuplicateJustification,

    SystemChildJustificationNotFound,
    SystemChildJustificationWrongKind,

    TheoremJustificationUnproven,
    TheoremJustificationUsedBeforeProof,
    TheoremJustificationCircularProof,

    HypothesisZeroIndex,
    HypothesisIndexOutOfRange,

    FormulaError(&'a FormulaBuilder<'a>, FormulaParsingError),
}

impl<'a> ProofStepParsingError<'a> {
    fn eprint_system_child_justification_not_found(proof: &ProofBuilder, step: &ProofBuilderStep) {
        let justification = step.justification().unwrap().system_child().unwrap();

        let message = format!(
            "A step of a proof for `{}` references `{}`, but this id doesn't correspond to any known child of the system `{}`.",
            proof.theorem_name(),
            justification.id(),
            proof.system_id(),
        );

        eprint(&message, step.file_location());
    }

    fn eprint(&self, proof: &ProofBuilder, step: &ProofBuilderStep) {
        match self {
            Self::SystemChildJustificationNotFound => {
                Self::eprint_system_child_justification_not_found(proof, step)
            }

            _ => todo!("{:#?}", self),
        }
    }
}

#[derive(Debug)]
pub enum ProofParsingError<'a> {
    ParentNotFound,
    ParentNotTheorem,

    TextError(&'a TextBuilder<'a>, TextParsingError<'a>),
    StepError(&'a ProofBuilderStep<'a>, ProofStepParsingError<'a>),
}

impl<'a> ProofParsingError<'a> {
    fn eprint(&self, proof: &ProofBuilder) {
        match self {
            Self::StepError(step, error) => error.eprint(proof, step),

            _ => todo!("{:#?}", self),
        }
    }
}

#[derive(Debug)]
pub enum TableParsingError<'a> {
    CellError(&'a ParagraphBuilder<'a>, ParagraphParsingError<'a>),
    CaptionError(ParagraphParsingError<'a>),
}

#[derive(Debug)]
pub enum QuoteValueParsingError {
    BibKeyNotFound,
}

#[derive(Debug)]
pub enum QuoteParsingError {
    OriginalError(QuoteValueParsingError),
    ValueError(QuoteValueParsingError),
}

#[derive(Debug)]
pub enum ParsingError<'a> {
    IoError(IoError),
    PestError(PestError<Rule>),
    UrlError(UrlError),

    BookError(&'a BookBuilder<'a>, BookParsingError<'a>),
    ChapterError(&'a ChapterBuilder<'a>, ChapterParsingError<'a>),
    BibliographyError(&'a BibliographyBuilderEntry, BibliographyParsingError<'a>),

    SystemError(&'a SystemBuilder<'a>, SystemParsingError<'a>),
    SystemChildError(SystemBuilderChild<'a>, SystemChildParsingError<'a>),
    ReadableError(ReadableBuilder<'a>, ReadableParsingError<'a>),

    TypeError(&'a TypeBuilder<'a>, TypeParsingError<'a>),
    SymbolError(&'a SymbolBuilder<'a>, SymbolParsingError<'a>),
    DefinitionError(&'a DefinitionBuilder<'a>, DefinitionParsingError<'a>),
    AxiomError(&'a AxiomBuilder<'a>, AxiomParsingError<'a>),
    TheoremError(&'a TheoremBuilder<'a>, TheoremParsingError<'a>),
    ProofError(&'a ProofBuilder<'a>, ProofParsingError<'a>),

    ListItemError(
        &'a ListBuilder<'a>,
        &'a ParagraphBuilder<'a>,
        ParagraphParsingError<'a>,
    ),
    TableError(&'a TableBuilder<'a>, TableParsingError<'a>),
    QuoteError(&'a QuoteBuilder<'a>, QuoteParsingError),
    TextError(&'a TextBuilder<'a>, TextParsingError<'a>),
}

impl<'a> ParsingError<'a> {
    fn eprint(&self) {
        match self {
            Self::TextError(text, error) => error.eprint(text),

            Self::ProofError(proof, error) => error.eprint(proof),

            _ => todo!("{:#?}", self),
        }
    }
}

impl<'a> From<IoError> for ParsingError<'a> {
    fn from(e: IoError) -> Self {
        ParsingError::IoError(e)
    }
}

impl<'a> From<PestError<Rule>> for ParsingError<'a> {
    fn from(e: PestError<Rule>) -> Self {
        ParsingError::PestError(e)
    }
}

impl<'a> From<UrlError> for ParsingError<'a> {
    fn from(e: UrlError) -> Self {
        ParsingError::UrlError(e)
    }
}

#[derive(Default, Debug)]
pub struct ParsingErrorContext<'a> {
    errors: Vec<ParsingError<'a>>,
}

impl<'a> ParsingErrorContext<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn err<E: Into<ParsingError<'a>>>(&mut self, e: E) {
        self.errors.push(e.into());
    }

    pub fn error_found(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn eprint(&self) {
        for error in &self.errors {
            error.eprint()
        }

        eprintln!("Checker exited with errors.");
    }
}
