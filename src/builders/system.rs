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

use std::cell::{Cell, RefCell};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::lazy::OnceCell;

use pest::iterators::{Pair, Pairs};

use crate::document::deduction::{
    AxiomBlock, ProofBlock, ProofBlockElement, ProofBlockJustification, ProofBlockStep,
    TheoremBlock,
};
use crate::document::directory::{
    AxiomBlockRef, BlockReference, ProofBlockRef, ProofBlockStepRef, SystemBlockRef,
    TheoremBlockRef,
};
use crate::document::language::SystemBlock;

use super::bibliography::BibliographyBuilderEntry;
use super::errors::{
    AxiomParsingError, FlagListParsingError, ParsingError, ParsingErrorContext, ProofParsingError,
    ProofStepParsingError, SystemParsingError, TheoremParsingError,
};
use super::index::{BuilderIndex, LocalBuilderIndex};
use super::language::{
    DefinitionBuilder, DisplayFormulaBuilder, SymbolBuilder, TypeBuilder, VariableBuilder,
};
use super::text::{ParagraphBuilder, TextBuilder};
use super::Rule;

struct SystemBuilderEntries<'a> {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder<'a>>,
    descriptions: Vec<Vec<TextBuilder<'a>>>,

    verified: Cell<bool>,
}

impl<'a> SystemBuilderEntries<'a> {
    fn from_pest(pairs: Pairs<Rule>) -> Self {
        let mut names = Vec::with_capacity(1);
        let mut taglines = Vec::with_capacity(1);
        let mut descriptions = Vec::with_capacity(1);

        for pair in pairs {
            match pair.as_rule() {
                Rule::block_name => {
                    let string = pair.into_inner().next().unwrap();
                    let string_contents = string.into_inner().next().unwrap();
                    let name = string_contents.as_str().to_owned();

                    names.push(name);
                }
                Rule::block_tagline => {
                    let tagline = ParagraphBuilder::from_pest(pair.into_inner().next().unwrap());

                    taglines.push(tagline);
                }
                Rule::block_description => {
                    let description = pair.into_inner().map(TextBuilder::from_pest).collect();

                    descriptions.push(description);
                }

                _ => unreachable!(),
            }
        }

        SystemBuilderEntries {
            names,
            taglines,
            descriptions,

            verified: Cell::new(false),
        }
    }

    fn verify_structure(
        &'a self,
        system_ref: &'a SystemBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.names.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::SystemError(
                    system_ref,
                    SystemParsingError::MissingName,
                ));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::SystemError(
                    system_ref,
                    SystemParsingError::DuplicateName,
                ));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::SystemError(
                    system_ref,
                    SystemParsingError::MissingTagline,
                ));
            }

            1 => {
                let success = self.taglines[0].verify_structure(index, errors, |e| {
                    ParsingError::SystemError(
                        system_ref,
                        SystemParsingError::TaglineParsingError(e),
                    )
                });
                if !success {
                    found_error = true;
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::SystemError(
                    system_ref,
                    SystemParsingError::DuplicateTagline,
                ));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for text in &self.descriptions[0] {
                    let success = text.verify_structure(index, errors, |e| {
                        ParsingError::SystemError(
                            system_ref,
                            SystemParsingError::DescriptionParsingError(text, e),
                        )
                    });

                    if !success {
                        found_error = true;
                    }
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::SystemError(
                    system_ref,
                    SystemParsingError::DuplicateDescription,
                ));
            }
        }

        self.verified.set(!found_error);
    }

    fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        let tagline_refs = self.tagline().bib_refs();
        let description_refs = self.description().iter().flat_map(TextBuilder::bib_refs);

        Box::new(tagline_refs.chain(description_refs))
    }

    fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.tagline().set_local_bib_refs(index);
        for text in self.description() {
            text.set_local_bib_refs(index);
        }
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&'a self) -> &ParagraphBuilder {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&'a self) -> &[TextBuilder] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }
}

pub struct SystemBuilder<'a> {
    id: String,

    entries: SystemBuilderEntries<'a>,

    // TODO: Remove.
    count: OnceCell<usize>,

    // TODO: Remove.
    href: OnceCell<String>,
}

impl<'a> SystemBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::system_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let entries = SystemBuilderEntries::from_pest(inner);

        SystemBuilder {
            id,
            entries,

            count: OnceCell::new(),
            href: OnceCell::new(),
        }
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.entries.verify_structure(self, index, errors);
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        self.entries.bib_refs()
    }

    pub fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.entries.set_local_bib_refs(index)
    }

    // TODO: Remove.
    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap()
    }

    // TODO: Remove.
    pub fn set_href(&self, book_id: &str, chapter_id: &str, page_id: &str) {
        let href = format!("/{}/{}/{}#{}", book_id, chapter_id, page_id, &self.id);
        self.href.set(href).unwrap();
    }

    // TODO: Remove.
    pub fn get_ref(&self) -> SystemBlockRef {
        SystemBlockRef::new(*self.count.get().unwrap())
    }

    // TODO: Remove.
    pub fn finish(&'a self) -> SystemBlock {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();
        let href = self.href.get().unwrap().to_owned();
        let tagline = self.entries.tagline().finish();
        let description = self
            .entries
            .description()
            .iter()
            .map(TextBuilder::finish)
            .collect();

        SystemBlock::new(id, name, href, tagline, description)
    }
}

impl<'a> std::fmt::Debug for SystemBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("System").field(&self.id).finish()
    }
}

#[derive(Clone, Copy)]
pub enum SystemBuilderChild<'a> {
    Type(&'a TypeBuilder<'a>),
    Symbol(&'a SymbolBuilder<'a>),
    Definition(&'a DefinitionBuilder<'a>),
    Axiom(&'a AxiomBuilder<'a>),
    Theorem(&'a TheoremBuilder<'a>),
}

impl<'a> SystemBuilderChild<'a> {
    pub fn set_system_ref(self, system_ref: &'a SystemBuilder<'a>) {
        match self {
            Self::Type(type_ref) => type_ref.set_system_ref(system_ref),
            Self::Symbol(symbol_ref) => symbol_ref.set_system_ref(system_ref),
            Self::Definition(definition_ref) => definition_ref.set_system_ref(system_ref),
            Self::Axiom(axiom_ref) => axiom_ref.set_system_ref(system_ref),
            Self::Theorem(theorem_ref) => theorem_ref.set_system_ref(system_ref),
        }
    }

    pub fn id(self) -> &'a str {
        match self {
            Self::Type(type_ref) => type_ref.id(),
            Self::Symbol(symbol_ref) => symbol_ref.id(),
            Self::Definition(definition_ref) => definition_ref.id(),
            Self::Axiom(axiom_ref) => axiom_ref.id(),
            Self::Theorem(theorem_ref) => theorem_ref.id(),
        }
    }

    pub fn system_id(self) -> &'a str {
        match self {
            Self::Type(type_ref) => type_ref.system_id(),
            Self::Symbol(symbol_ref) => symbol_ref.system_id(),
            Self::Definition(definition_ref) => definition_ref.system_id(),
            Self::Axiom(axiom_ref) => axiom_ref.system_id(),
            Self::Theorem(theorem_ref) => theorem_ref.system_id(),
        }
    }

    pub fn ty(self) -> Option<&'a TypeBuilder<'a>> {
        match self {
            Self::Type(type_ref) => Some(type_ref),

            _ => None,
        }
    }

    pub fn theorem(self) -> Option<&'a TheoremBuilder<'a>> {
        match self {
            Self::Theorem(theorem_ref) => Some(theorem_ref),

            _ => None,
        }
    }

    pub fn get_ref(self) -> BlockReference {
        match self {
            Self::Type(type_ref) => BlockReference::Type(type_ref.get_ref()),
            Self::Symbol(symbol_ref) => BlockReference::Symbol(symbol_ref.get_ref()),
            Self::Definition(definition_ref) => {
                BlockReference::Definition(definition_ref.get_ref())
            }
            Self::Axiom(axiom_ref) => BlockReference::Axiom(axiom_ref.get_ref()),
            Self::Theorem(theorem_ref) => BlockReference::Theorem(theorem_ref.get_ref()),
        }
    }
}

impl<'a> std::fmt::Debug for SystemBuilderChild<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Type(type_ref) => f.debug_tuple("Type").field(&type_ref.id()).finish(),
            Self::Symbol(symbol_ref) => f.debug_tuple("Symbol").field(&symbol_ref.id()).finish(),
            Self::Definition(definition_ref) => f
                .debug_tuple("Definition")
                .field(&definition_ref.id())
                .finish(),
            Self::Axiom(axiom_ref) => f.debug_tuple("Axiom").field(&axiom_ref.id()).finish(),
            Self::Theorem(theorem_ref) => {
                f.debug_tuple("Theorem").field(&theorem_ref.id()).finish()
            }
        }
    }
}

#[derive(Debug)]
pub enum Flag {
    Reflexive,
    Symmetric,
    Transitive,

    Function,
}

impl Flag {
    fn from_pest(pair: Pair<Rule>) -> Flag {
        match pair.as_rule() {
            Rule::flag_reflexive => Flag::Reflexive,
            Rule::flag_symmetric => Flag::Symmetric,
            Rule::flag_transitive => Flag::Transitive,

            Rule::flag_function => Flag::Function,

            _ => unreachable!(),
        }
    }
}

struct FlagList {
    raw_list: Vec<Flag>,

    reflexive: Cell<bool>,
    symmetric: Cell<bool>,
    transitive: Cell<bool>,

    function: Cell<bool>,

    verified: Cell<bool>,
}

impl FlagList {
    fn from_pest(pair: Pair<Rule>) -> FlagList {
        assert_eq!(pair.as_rule(), Rule::flag_list);

        let raw_list = pair.into_inner().map(Flag::from_pest).collect();

        FlagList {
            raw_list,

            reflexive: Cell::new(false),
            symmetric: Cell::new(false),
            transitive: Cell::new(false),

            function: Cell::new(false),

            verified: Cell::new(false),
        }
    }

    fn verify_structure<'a, F>(
        &self,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(FlagListParsingError<'a>) -> ParsingError<'a>,
    {
        assert!(!self.verified.get());
        let mut found_error = false;

        for flag in &self.raw_list {
            match flag {
                Flag::Reflexive => {
                    if self.reflexive.get() {
                        found_error = true;
                        errors.err(generate_error(FlagListParsingError::DuplicateFlag(
                            Flag::Reflexive,
                        )));
                    } else {
                        self.reflexive.set(true);
                    }
                }

                Flag::Symmetric => {
                    if self.symmetric.get() {
                        found_error = true;
                        errors.err(generate_error(FlagListParsingError::DuplicateFlag(
                            Flag::Symmetric,
                        )));
                    } else {
                        self.symmetric.set(true);
                    }
                }

                Flag::Transitive => {
                    if self.transitive.get() {
                        found_error = true;
                        errors.err(generate_error(FlagListParsingError::DuplicateFlag(
                            Flag::Transitive,
                        )));
                    } else {
                        self.transitive.set(true);
                    }
                }

                Flag::Function => {
                    if self.function.get() {
                        found_error = true;
                        errors.err(generate_error(FlagListParsingError::DuplicateFlag(
                            Flag::Function,
                        )));
                    } else {
                        self.function.set(true);
                    }
                }
            }
        }

        self.verified.set(!found_error);
        !found_error
    }

    fn verify_reflexivity<'a, F>(
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) where
        F: Fn(FlagListParsingError<'a>) -> ParsingError<'a>,
    {
        if !deductable_ref.premise().is_empty() {
            errors.err(generate_error(
                FlagListParsingError::ReflexivityPremiseNotEmpty,
            ));
            return;
        }

        if let Some((assertion_function, assertion_left, assertion_right)) =
            deductable_ref.assertion().simple_binary()
        {
            if assertion_left != assertion_right {
                errors.err(generate_error(
                    FlagListParsingError::ReflexivityArgumentMismatch,
                ));
            } else {
                assertion_function.set_reflexive(deductable_ref, errors);
            }
        } else {
            errors.err(generate_error(
                FlagListParsingError::ReflexivityAssertionNotBinary,
            ));
        }
    }

    fn verify_symmetry<'a, F>(
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) where
        F: Fn(FlagListParsingError<'a>) -> ParsingError<'a>,
    {
        let premise = deductable_ref.premise();
        if premise.len() != 1 {
            errors.err(generate_error(
                FlagListParsingError::SymmetryPremiseWrongLength,
            ));
            return;
        }

        let (premise_function, premise_left, premise_right) =
            if let Some(info) = premise[0].simple_binary() {
                info
            } else {
                errors.err(generate_error(
                    FlagListParsingError::SymmetryPremiseNotBinary,
                ));
                return;
            };

        let (assertion_function, assertion_left, assertion_right) =
            if let Some(info) = deductable_ref.assertion().simple_binary() {
                info
            } else {
                errors.err(generate_error(
                    FlagListParsingError::SymmetryAssertionNotBinary,
                ));
                return;
            };

        if premise_function != assertion_function {
            errors.err(generate_error(FlagListParsingError::SymmetrySymbolMismatch));
            return;
        }

        if premise_left != assertion_right || premise_right != assertion_left {
            errors.err(generate_error(
                FlagListParsingError::SymmetryArgumentMismatch,
            ));
        }

        assertion_function.set_symmetric(deductable_ref, errors);
    }

    fn verify_transitivity<'a, F>(
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) where
        F: Fn(FlagListParsingError<'a>) -> ParsingError<'a>,
    {
        let premise = deductable_ref.premise();
        if premise.len() != 2 {
            errors.err(generate_error(
                FlagListParsingError::TransitivityWrongPremiseLength,
            ));
            return;
        }

        let (first_premise_function, first_premise_left, first_premise_right) =
            if let Some(info) = premise[0].simple_binary() {
                info
            } else {
                errors.err(generate_error(
                    FlagListParsingError::TransitivityFirstPremiseNotBinary,
                ));
                return;
            };

        let (second_premise_function, second_premise_left, second_premise_right) =
            if let Some(info) = premise[1].simple_binary() {
                info
            } else {
                errors.err(generate_error(
                    FlagListParsingError::TransitivitySecondPremiseNotBinary,
                ));
                return;
            };

        if first_premise_function != second_premise_function {
            errors.err(generate_error(
                FlagListParsingError::TransitivityPremiseSymbolNotEqual,
            ));
            return;
        }

        if first_premise_right != second_premise_left {
            errors.err(generate_error(
                FlagListParsingError::TransitivityPremiseArgumentMismatch,
            ));
            return;
        }

        let (assertion_function, assertion_left, assertion_right) =
            if let Some(info) = deductable_ref.assertion().simple_binary() {
                info
            } else {
                errors.err(generate_error(
                    FlagListParsingError::TransitivityAssertionNotBinary,
                ));
                return;
            };

        if assertion_function != first_premise_function {
            errors.err(generate_error(
                FlagListParsingError::TransitivityAssertionSymbolNotEqual,
            ));
            return;
        }

        if assertion_left != first_premise_left {
            errors.err(generate_error(
                FlagListParsingError::TransitivityAssertionLeftMismatch,
            ));
            return;
        }

        if assertion_right != second_premise_right {
            errors.err(generate_error(
                FlagListParsingError::TransitivityAssertionRightMismatch,
            ));
            return;
        }

        assertion_function.set_transitive(deductable_ref, errors);
    }

    fn verify_function<'a, F>(
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) where
        F: Fn(FlagListParsingError<'a>) -> ParsingError<'a> + Copy,
    {
        let premise = deductable_ref.premise();
        if premise.is_empty() {
            errors.err(generate_error(FlagListParsingError::FunctionPremiseEmpty));
            return;
        }

        let (relation, assertion_left, assertion_right) =
            if let Some(info) = deductable_ref.assertion().binary() {
                info
            } else {
                errors.err(generate_error(
                    FlagListParsingError::FunctionAssertionNotBinary,
                ));
                return;
            };

        if !relation.is_preorder() {
            errors.err(generate_error(
                FlagListParsingError::FunctionRelationNotPreorder,
            ));
            return;
        }

        let (assertion_left_function, assertion_left_inputs) =
            if let Some(info) = assertion_left.application() {
                info
            } else {
                errors.err(generate_error(
                    FlagListParsingError::FunctionAssertionLeftNotApplication,
                ));
                return;
            };

        let (assertion_right_function, assertion_right_inputs) =
            if let Some(info) = assertion_right.application() {
                info
            } else {
                errors.err(generate_error(
                    FlagListParsingError::FunctionAssertionRightNotApplication,
                ));
                return;
            };

        if assertion_left_function != assertion_right_function {
            errors.err(generate_error(
                FlagListParsingError::FunctionAssertionSymbolMismatch,
            ));
            return;
        }

        if assertion_left_inputs.len() != assertion_right_inputs.len() {
            errors.err(generate_error(
                FlagListParsingError::FunctionAssertionArityMismatch,
            ));
            return;
        }

        if premise.len() != assertion_left_inputs.len() {
            errors.err(generate_error(
                FlagListParsingError::FunctionPremiseArityMismatch,
            ));
            return;
        }

        let iter = premise
            .iter()
            .zip(assertion_left_inputs)
            .zip(assertion_right_inputs);
        for ((hypothesis, assertion_left_input), assertion_right_input) in iter {
            let assertion_left_var = match assertion_left_input.variable() {
                Some(var) => var,
                None => {
                    errors.err(generate_error(
                        FlagListParsingError::FunctionAssertionInputNotVariable(
                            assertion_left_input,
                        ),
                    ));
                    return;
                }
            };

            let assertion_right_var = match assertion_right_input.variable() {
                Some(var) => var,
                None => {
                    errors.err(generate_error(
                        FlagListParsingError::FunctionAssertionInputNotVariable(
                            assertion_right_input,
                        ),
                    ));
                    return;
                }
            };

            let (hypothesis_relation, hypothesis_left, hypothesis_right) =
                if let Some(info) = hypothesis.simple_binary() {
                    info
                } else {
                    errors.err(generate_error(
                        FlagListParsingError::FunctionHypothesisNotBinary(hypothesis),
                    ));
                    return;
                };

            if hypothesis_relation != relation {
                errors.err(generate_error(
                    FlagListParsingError::FunctionHypothesisRelationMismatch(hypothesis),
                ));
                return;
            }

            if hypothesis_left != assertion_left_var {
                errors.err(generate_error(
                    FlagListParsingError::FunctionHypothesisLeftVarMismatch(hypothesis),
                ));
                return;
            }

            if hypothesis_right != assertion_right_var {
                errors.err(generate_error(
                    FlagListParsingError::FunctionHypothesisRightVarMismatch(hypothesis),
                ));
                return;
            }
        }

        assertion_left_function.set_function(deductable_ref, relation, errors);
    }

    fn verify_formulas<'a, F>(
        &self,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) where
        F: Fn(FlagListParsingError<'a>) -> ParsingError<'a> + Copy,
    {
        assert!(self.verified.get());

        if self.reflexive.get() {
            Self::verify_reflexivity(deductable_ref, errors, generate_error);
        }

        if self.symmetric.get() {
            Self::verify_symmetry(deductable_ref, errors, generate_error);
        }

        if self.transitive.get() {
            Self::verify_transitivity(deductable_ref, errors, generate_error);
        }

        if self.function.get() {
            Self::verify_function(deductable_ref, errors, generate_error);
        }
    }
}

struct AxiomBuilderEntries<'a> {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder<'a>>,
    descriptions: Vec<Vec<TextBuilder<'a>>>,

    flag_lists: Vec<FlagList>,
    vars: Vec<VariableBuilder<'a>>,
    premises: Vec<Vec<DisplayFormulaBuilder<'a>>>,
    assertions: Vec<DisplayFormulaBuilder<'a>>,

    verified: Cell<bool>,
}

impl<'a> AxiomBuilderEntries<'a> {
    fn from_pest(pairs: Pairs<Rule>) -> Self {
        let mut names = Vec::with_capacity(1);
        let mut taglines = Vec::with_capacity(1);
        let mut descriptions = Vec::with_capacity(1);
        let mut vars = Vec::new();
        let mut premises = Vec::new();
        let mut assertions = Vec::new();
        let mut flag_lists = Vec::new();

        for pair in pairs {
            match pair.as_rule() {
                Rule::block_name => {
                    let string = pair.into_inner().next().unwrap();
                    let string_contents = string.into_inner().next().unwrap();
                    let name = string_contents.as_str().to_owned();

                    names.push(name);
                }
                Rule::block_tagline => {
                    let tagline = ParagraphBuilder::from_pest(pair.into_inner().next().unwrap());

                    taglines.push(tagline);
                }
                Rule::block_description => {
                    let description = pair.into_inner().map(TextBuilder::from_pest).collect();

                    descriptions.push(description);
                }

                Rule::block_flags => {
                    let flag_list = FlagList::from_pest(pair.into_inner().next().unwrap());

                    flag_lists.push(flag_list);
                }
                Rule::var_declaration => vars.push(VariableBuilder::from_pest(pair)),
                Rule::premise => {
                    let premise = pair
                        .into_inner()
                        .map(DisplayFormulaBuilder::from_pest)
                        .collect();

                    premises.push(premise);
                }
                Rule::assertion => {
                    let assertion =
                        DisplayFormulaBuilder::from_pest(pair.into_inner().next().unwrap());

                    assertions.push(assertion);
                }

                _ => unreachable!(),
            }
        }

        AxiomBuilderEntries {
            names,
            taglines,
            descriptions,

            flag_lists,
            vars,
            premises,
            assertions,

            verified: Cell::new(false),
        }
    }

    fn verify_structure(
        &'a self,
        axiom_ref: &'a AxiomBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.names.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::AxiomError(
                    axiom_ref,
                    AxiomParsingError::MissingName,
                ));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::AxiomError(
                    axiom_ref,
                    AxiomParsingError::DuplicateName,
                ));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::AxiomError(
                    axiom_ref,
                    AxiomParsingError::MissingTagline,
                ));
            }

            1 => {
                let success = self.taglines[0].verify_structure(index, errors, |e| {
                    ParsingError::AxiomError(axiom_ref, AxiomParsingError::TaglineParsingError(e))
                });

                if !success {
                    found_error = true;
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::AxiomError(
                    axiom_ref,
                    AxiomParsingError::DuplicateTagline,
                ));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for text in &self.descriptions[0] {
                    let success = text.verify_structure(index, errors, |e| {
                        ParsingError::AxiomError(
                            axiom_ref,
                            AxiomParsingError::DescriptionParsingError(text, e),
                        )
                    });

                    if !success {
                        found_error = true;
                    }
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::AxiomError(
                    axiom_ref,
                    AxiomParsingError::DuplicateDescription,
                ))
            }
        }

        match self.flag_lists.len() {
            0 => {}
            1 => {
                let success = self.flag_lists[0].verify_structure(errors, |e| {
                    ParsingError::AxiomError(axiom_ref, AxiomParsingError::FlagListError(e))
                });

                if !success {
                    found_error = true;
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::AxiomError(
                    axiom_ref,
                    AxiomParsingError::DuplicateFlagList,
                ));
            }
        }

        for var in &self.vars {
            let success =
                var.verify_structure(&axiom_ref.system_id, axiom_ref.serial, index, errors, |e| {
                    ParsingError::AxiomError(axiom_ref, AxiomParsingError::VariableError(var, e))
                });

            if !success {
                found_error = true
            }
        }

        if self.premises.len() > 1 {
            found_error = true;
            errors.err(ParsingError::AxiomError(
                axiom_ref,
                AxiomParsingError::DuplicatePremise,
            ));
        }

        match self.assertions.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::AxiomError(
                    axiom_ref,
                    AxiomParsingError::MissingAssertion,
                ));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::AxiomError(
                    axiom_ref,
                    AxiomParsingError::DuplicateAssertion,
                ));
            }
        }

        self.verified.set(!found_error);
    }

    fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        let tagline_refs = self.tagline().bib_refs();
        let description_refs = self.description().iter().flat_map(TextBuilder::bib_refs);

        Box::new(tagline_refs.chain(description_refs))
    }

    fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.tagline().set_local_bib_refs(index);

        for paragraph in self.description() {
            paragraph.set_local_bib_refs(index);
        }
    }

    fn build_formulas(
        &'a self,
        axiom_ref: &'a AxiomBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        let local_index = index.get_local(axiom_ref.system_id(), &self.vars);

        for hypothesis in self.premise() {
            hypothesis.build(&local_index, errors, |formula, e| {
                ParsingError::AxiomError(axiom_ref, AxiomParsingError::FormulaError(formula, e))
            });
        }

        self.assertion().build(&local_index, errors, |formula, e| {
            ParsingError::AxiomError(axiom_ref, AxiomParsingError::FormulaError(formula, e))
        });

        if let Some(flag_list) = self.flag_list() {
            flag_list.verify_formulas(DeductableBuilder::Axiom(axiom_ref), errors, |e| {
                ParsingError::AxiomError(axiom_ref, AxiomParsingError::FlagListError(e))
            });
        }
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&'a self) -> &ParagraphBuilder {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&'a self) -> &[TextBuilder] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }

    fn flag_list(&self) -> Option<&FlagList> {
        assert!(self.verified.get());
        self.flag_lists.get(0)
    }

    fn vars(&'a self) -> &[VariableBuilder] {
        assert!(self.verified.get());
        &self.vars
    }

    fn premise(&'a self) -> &[DisplayFormulaBuilder] {
        assert!(self.verified.get());
        if self.premises.is_empty() {
            &[]
        } else {
            &self.premises[0]
        }
    }

    fn assertion(&'a self) -> &DisplayFormulaBuilder {
        assert!(self.verified.get());
        &self.assertions[0]
    }
}

pub struct AxiomBuilder<'a> {
    id: String,
    system_id: String,
    serial: usize,

    system_ref: OnceCell<&'a SystemBuilder<'a>>,
    entries: AxiomBuilderEntries<'a>,

    // TODO: Remove.
    count: OnceCell<usize>,

    // TODO: Remove.
    href: OnceCell<String>,
}

impl<'a> AxiomBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>, serial: usize) -> Self {
        assert_eq!(pair.as_rule(), Rule::axiom_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();

        let entries = AxiomBuilderEntries::from_pest(inner);

        AxiomBuilder {
            id,
            system_id,
            serial,

            system_ref: OnceCell::new(),
            entries,

            count: OnceCell::new(),
            href: OnceCell::new(),
        }
    }

    pub fn set_system_ref(&self, system_ref: &'a SystemBuilder<'a>) {
        self.system_ref.set(system_ref).unwrap();
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.entries.verify_structure(self, index, errors);
    }

    pub fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        self.entries.bib_refs()
    }

    pub fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.entries.set_local_bib_refs(index);
    }

    pub fn build_formulas(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.entries.build_formulas(self, index, errors);
    }

    // TODO: Remove.
    pub fn count(&'a self, count: usize) {
        self.count.set(count).unwrap();

        for (i, var) in self.entries.vars().iter().enumerate() {
            var.count(i);
        }
    }

    // TODO: Remove.
    pub fn get_ref(&self) -> AxiomBlockRef {
        AxiomBlockRef::new(*self.count.get().unwrap())
    }

    // TODO: Remove.
    pub fn set_href(&self, book_id: &str, chapter_id: &str, page_id: &str) {
        let href = format!(
            "/{}/{}/{}#{}_{}",
            book_id, chapter_id, page_id, &self.system_id, &self.id
        );
        self.href.set(href).unwrap()
    }

    // TODO: Remove.
    pub fn finish(&'a self) -> AxiomBlock {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();
        let system = self.system_ref.get().unwrap().get_ref();
        let href = self.href.get().unwrap().to_owned();
        let tagline = self.entries.tagline().finish();
        let description = self
            .entries
            .description()
            .iter()
            .map(TextBuilder::finish)
            .collect();

        let vars = self
            .entries
            .vars()
            .iter()
            .map(VariableBuilder::finish)
            .collect();
        let premise = self
            .entries
            .premise()
            .iter()
            .map(DisplayFormulaBuilder::finish)
            .collect();
        let assertion = self.entries.assertion().finish();

        AxiomBlock::new(
            id,
            name,
            system,
            href,
            tagline,
            description,
            vars,
            premise,
            assertion,
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    pub fn premise(&'a self) -> &[DisplayFormulaBuilder] {
        self.entries.premise()
    }

    pub fn assertion(&'a self) -> &DisplayFormulaBuilder {
        self.entries.assertion()
    }
}

impl<'a> std::fmt::Debug for AxiomBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Axiom").field(&self.id).finish()
    }
}

struct TheoremBuilderEntries<'a> {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder<'a>>,
    descriptions: Vec<Vec<TextBuilder<'a>>>,

    flag_lists: Vec<FlagList>,
    vars: Vec<VariableBuilder<'a>>,
    premises: Vec<Vec<DisplayFormulaBuilder<'a>>>,
    assertions: Vec<DisplayFormulaBuilder<'a>>,

    verified: Cell<bool>,
}

impl<'a> TheoremBuilderEntries<'a> {
    fn from_pest(pairs: Pairs<Rule>) -> Self {
        let mut names = Vec::with_capacity(1);
        let mut taglines = Vec::with_capacity(1);
        let mut descriptions = Vec::with_capacity(1);
        let mut flag_lists = Vec::new();
        let mut vars = Vec::new();
        let mut premises = Vec::new();
        let mut assertions = Vec::new();

        for pair in pairs {
            match pair.as_rule() {
                Rule::block_name => {
                    let string = pair.into_inner().next().unwrap();
                    let string_contents = string.into_inner().next().unwrap();
                    let name = string_contents.as_str().to_owned();

                    names.push(name);
                }
                Rule::block_tagline => {
                    let tagline = ParagraphBuilder::from_pest(pair.into_inner().next().unwrap());

                    taglines.push(tagline);
                }
                Rule::block_description => {
                    let description = pair.into_inner().map(TextBuilder::from_pest).collect();

                    descriptions.push(description);
                }

                Rule::block_flags => {
                    let flag_list = FlagList::from_pest(pair.into_inner().next().unwrap());

                    flag_lists.push(flag_list);
                }
                Rule::var_declaration => vars.push(VariableBuilder::from_pest(pair)),
                Rule::premise => {
                    let premise = pair
                        .into_inner()
                        .map(DisplayFormulaBuilder::from_pest)
                        .collect();

                    premises.push(premise);
                }
                Rule::assertion => {
                    let assertion =
                        DisplayFormulaBuilder::from_pest(pair.into_inner().next().unwrap());

                    assertions.push(assertion);
                }

                _ => unreachable!(),
            }
        }

        TheoremBuilderEntries {
            names,
            taglines,
            descriptions,

            flag_lists,
            vars,
            premises,
            assertions,

            verified: Cell::new(false),
        }
    }

    fn verify_structure(
        &'a self,
        theorem_ref: &'a TheoremBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.names.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::TheoremError(
                    theorem_ref,
                    TheoremParsingError::MissingName,
                ));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::TheoremError(
                    theorem_ref,
                    TheoremParsingError::DuplicateName,
                ));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::TheoremError(
                    theorem_ref,
                    TheoremParsingError::MissingTagline,
                ));
            }

            1 => {
                let success = self.taglines[0].verify_structure(index, errors, |e| {
                    ParsingError::TheoremError(
                        theorem_ref,
                        TheoremParsingError::TaglineParsingError(e),
                    )
                });

                if !success {
                    found_error = true;
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::TheoremError(
                    theorem_ref,
                    TheoremParsingError::DuplicateTagline,
                ));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for text in &self.descriptions[0] {
                    let success = text.verify_structure(index, errors, |e| {
                        ParsingError::TheoremError(
                            theorem_ref,
                            TheoremParsingError::DescriptionParsingError(text, e),
                        )
                    });

                    if !success {
                        found_error = true
                    }
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::TheoremError(
                    theorem_ref,
                    TheoremParsingError::DuplicateDescription,
                ));
            }
        }

        match self.flag_lists.len() {
            0 => {}
            1 => {
                let success = self.flag_lists[0].verify_structure(errors, |e| {
                    ParsingError::TheoremError(theorem_ref, TheoremParsingError::FlagListError(e))
                });

                if !success {
                    found_error = true;
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::TheoremError(
                    theorem_ref,
                    TheoremParsingError::DuplicateFlagList,
                ));
            }
        }

        for var in &self.vars {
            var.verify_structure(
                &theorem_ref.system_id,
                theorem_ref.serial,
                index,
                errors,
                |e| {
                    ParsingError::TheoremError(
                        theorem_ref,
                        TheoremParsingError::VariableError(var, e),
                    )
                },
            );
        }

        if self.premises.len() > 1 {
            found_error = true;
            errors.err(ParsingError::TheoremError(
                theorem_ref,
                TheoremParsingError::DuplicatePremise,
            ));
        }

        match self.assertions.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::TheoremError(
                    theorem_ref,
                    TheoremParsingError::MissingAssertion,
                ));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::TheoremError(
                    theorem_ref,
                    TheoremParsingError::DuplicateAssertion,
                ));
            }
        }

        self.verified.set(!found_error);
    }

    fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        let tagline_refs = self.tagline().bib_refs();
        let description_refs = self.description().iter().flat_map(TextBuilder::bib_refs);

        Box::new(tagline_refs.chain(description_refs))
    }

    fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.tagline().set_local_bib_refs(index);

        for paragraph in self.description() {
            paragraph.set_local_bib_refs(index);
        }
    }

    fn build_formulas(
        &'a self,
        theorem_ref: &'a TheoremBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        let local_index = index.get_local(theorem_ref.system_id(), &self.vars);

        for hypothesis in self.premise() {
            hypothesis.build(&local_index, errors, |formula, e| {
                ParsingError::TheoremError(
                    theorem_ref,
                    TheoremParsingError::FormulaError(formula, e),
                )
            });
        }

        self.assertion().build(&local_index, errors, |formula, e| {
            ParsingError::TheoremError(theorem_ref, TheoremParsingError::FormulaError(formula, e))
        });

        if let Some(flag_list) = self.flag_list() {
            flag_list.verify_formulas(DeductableBuilder::Theorem(theorem_ref), errors, |e| {
                ParsingError::TheoremError(theorem_ref, TheoremParsingError::FlagListError(e))
            });
        }
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&'a self) -> &ParagraphBuilder {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&'a self) -> &[TextBuilder] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }

    fn flag_list(&self) -> Option<&FlagList> {
        assert!(self.verified.get());
        self.flag_lists.get(0)
    }

    fn vars(&'a self) -> &[VariableBuilder] {
        assert!(self.verified.get());
        &self.vars
    }

    fn premise(&'a self) -> &[DisplayFormulaBuilder] {
        assert!(self.verified.get());
        if self.premises.is_empty() {
            &[]
        } else {
            &self.premises[0]
        }
    }

    fn assertion(&'a self) -> &DisplayFormulaBuilder {
        assert!(self.verified.get());
        &self.assertions[0]
    }
}

pub struct TheoremBuilder<'a> {
    id: String,
    system_id: String,
    serial: usize,

    system_ref: OnceCell<&'a SystemBuilder<'a>>,
    entries: TheoremBuilderEntries<'a>,

    proofs: RefCell<Vec<&'a ProofBuilder<'a>>>,

    // TODO: Remove.
    count: OnceCell<usize>,

    // TODO: Remove.
    href: OnceCell<String>,
}

impl<'a> TheoremBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>, serial: usize) -> Self {
        assert_eq!(pair.as_rule(), Rule::theorem_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();

        let entries = TheoremBuilderEntries::from_pest(inner);

        TheoremBuilder {
            id,
            system_id,
            serial,

            system_ref: OnceCell::new(),
            entries,

            proofs: RefCell::new(Vec::new()),

            count: OnceCell::new(),
            href: OnceCell::new(),
        }
    }

    pub fn set_system_ref(&self, system_ref: &'a SystemBuilder<'a>) {
        self.system_ref.set(system_ref).unwrap();
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.entries.verify_structure(self, index, errors)
    }

    pub fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        self.entries.bib_refs()
    }

    pub fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.entries.set_local_bib_refs(index);
    }

    pub fn build_formulas(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.entries.build_formulas(self, index, errors);
    }

    // TODO: Remove.
    pub fn count(&'a self, count: usize) {
        self.count.set(count).unwrap();

        for (i, var) in self.entries.vars().iter().enumerate() {
            var.count(i);
        }
    }

    // TODO: Remove.
    pub fn get_ref(&self) -> TheoremBlockRef {
        TheoremBlockRef::new(*self.count.get().unwrap())
    }

    // TODO: Remove.
    pub fn set_href(&self, book_id: &str, chapter_id: &str, page_id: &str) {
        let href = format!(
            "/{}/{}/{}#{}_{}",
            book_id, chapter_id, page_id, &self.system_id, &self.id
        );
        self.href.set(href).unwrap();
    }

    // TODO: Remove.
    pub fn finish(&'a self) -> TheoremBlock {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();
        let system = self.system_ref.get().unwrap().get_ref();
        let href = self.href.get().unwrap().to_owned();
        let tagline = self.entries.tagline().finish();
        let description = self
            .entries
            .description()
            .iter()
            .map(TextBuilder::finish)
            .collect();

        let vars = self
            .entries
            .vars()
            .iter()
            .map(VariableBuilder::finish)
            .collect();
        let premise = self
            .premise()
            .iter()
            .map(DisplayFormulaBuilder::finish)
            .collect();
        let assertion = self.entries.assertion().finish();

        TheoremBlock::new(
            id,
            name,
            system,
            href,
            tagline,
            description,
            vars,
            premise,
            assertion,
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    pub fn vars(&'a self) -> &[VariableBuilder] {
        self.entries.vars()
    }

    pub fn premise(&'a self) -> &[DisplayFormulaBuilder] {
        self.entries.premise()
    }

    pub fn assertion(&'a self) -> &DisplayFormulaBuilder {
        self.entries.assertion()
    }

    fn add_proof(&self, proof_ref: &'a ProofBuilder<'a>) {
        let mut proofs = self.proofs.borrow_mut();
        proofs.push(proof_ref);
    }

    fn first_proof(&'a self) -> Option<&ProofBuilder> {
        let proofs = self.proofs.borrow();
        proofs.get(0).copied()
    }
}

impl<'a> std::fmt::Debug for TheoremBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Theorem").field(&self.id).finish()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DeductableBuilder<'a> {
    Axiom(&'a AxiomBuilder<'a>),
    Theorem(&'a TheoremBuilder<'a>),
}

impl<'a> DeductableBuilder<'a> {
    fn premise(self) -> &'a [DisplayFormulaBuilder<'a>] {
        match self {
            Self::Axiom(axiom_ref) => axiom_ref.premise(),
            Self::Theorem(theorem_ref) => theorem_ref.premise(),
        }
    }

    fn assertion(self) -> &'a DisplayFormulaBuilder<'a> {
        match self {
            Self::Axiom(axiom_ref) => axiom_ref.assertion(),
            Self::Theorem(theorem_ref) => theorem_ref.assertion(),
        }
    }
}

#[derive(Debug)]
struct SystemChildJustificationBuilder<'a> {
    id: String,

    // TODO: Make this a DeductableBuilder instead of a SystemBuilderChild.
    child: OnceCell<SystemBuilderChild<'a>>,
}

impl<'a> SystemChildJustificationBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::ident);

        let id = pair.as_str().to_owned();

        SystemChildJustificationBuilder {
            id,

            child: OnceCell::new(),
        }
    }

    fn verify_structure(
        &self,
        proof_ref: &'a ProofBuilder<'a>,
        step_ref: &'a ProofBuilderStep<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) -> bool {
        let child = match index.search_system_child(&proof_ref.system_id, &self.id) {
            Some(child) => child,

            None => {
                errors.err(ParsingError::ProofError(
                    proof_ref,
                    ProofParsingError::StepError(
                        step_ref,
                        ProofStepParsingError::SystemChildJustificationNotFound,
                    ),
                ));
                return false;
            }
        };

        match child {
            SystemBuilderChild::Axiom(_) => {
                self.child.set(child).unwrap();
                true
            }

            SystemBuilderChild::Theorem(theorem_ref) => {
                let first_proof = match theorem_ref.first_proof() {
                    Some(first_proof) => first_proof,
                    None => {
                        errors.err(ParsingError::ProofError(
                            proof_ref,
                            ProofParsingError::StepError(
                                step_ref,
                                ProofStepParsingError::TheoremJustificationUnproven,
                            ),
                        ));

                        return false;
                    }
                };

                if proof_ref.serial < first_proof.serial {
                    errors.err(ParsingError::ProofError(
                        proof_ref,
                        ProofParsingError::StepError(
                            step_ref,
                            ProofStepParsingError::TheoremJustificationUsedBeforeProof,
                        ),
                    ));

                    false
                } else if proof_ref.serial == first_proof.serial {
                    errors.err(ParsingError::ProofError(
                        proof_ref,
                        ProofParsingError::StepError(
                            step_ref,
                            ProofStepParsingError::TheoremJustificationCircularProof,
                        ),
                    ));
                    false
                } else {
                    self.child.set(child).unwrap();
                    true
                }
            }

            _ => {
                errors.err(ParsingError::ProofError(
                    proof_ref,
                    ProofParsingError::StepError(
                        step_ref,
                        ProofStepParsingError::SystemChildJustificationWrongKind,
                    ),
                ));

                false
            }
        }
    }

    fn finish(&self) -> ProofBlockJustification {
        match self.child.get().unwrap() {
            SystemBuilderChild::Axiom(axiom_ref) => {
                ProofBlockJustification::Axiom(axiom_ref.get_ref())
            }
            SystemBuilderChild::Theorem(theorem_ref) => {
                ProofBlockJustification::Theorem(theorem_ref.get_ref())
            }

            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
enum MacroJustificationBuilder {
    Definition,
}

impl MacroJustificationBuilder {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::macro_justification);

        match pair.into_inner().next().unwrap().as_rule() {
            Rule::macro_justification_by_definition => Self::Definition,

            _ => unreachable!(),
        }
    }

    fn finish(&self) -> ProofBlockJustification {
        match self {
            Self::Definition => ProofBlockJustification::Definition,
        }
    }
}

#[derive(Debug)]
enum ProofJustificationBuilder<'a> {
    SystemChild(SystemChildJustificationBuilder<'a>),
    Macro(MacroJustificationBuilder),
    // TODO: Create a HypothesisJustificationBuilder which references the hypothesis itself instead
    // of its index.
    Hypothesis(usize),
}

impl<'a> ProofJustificationBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::ident => Self::SystemChild(SystemChildJustificationBuilder::from_pest(pair)),
            Rule::macro_justification => Self::Macro(MacroJustificationBuilder::from_pest(pair)),

            _ => unreachable!(),
        }
    }

    fn hypothesis_from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::integer);

        ProofJustificationBuilder::Hypothesis(pair.as_str().parse().unwrap())
    }

    fn verify_structure(
        &self,
        theorem_ref: &'a TheoremBuilder<'a>,
        proof_ref: &'a ProofBuilder<'a>,
        step_ref: &'a ProofBuilderStep<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) -> bool {
        match self {
            Self::SystemChild(builder) => {
                builder.verify_structure(proof_ref, step_ref, index, errors)
            }

            Self::Macro(_) => true,

            Self::Hypothesis(id) => {
                let premise_len = theorem_ref.premise().len();

                if *id == 0 {
                    errors.err(ParsingError::ProofError(
                        proof_ref,
                        ProofParsingError::StepError(
                            step_ref,
                            ProofStepParsingError::HypothesisZeroIndex,
                        ),
                    ));

                    false
                } else if *id > premise_len {
                    errors.err(ParsingError::ProofError(
                        proof_ref,
                        ProofParsingError::StepError(
                            step_ref,
                            ProofStepParsingError::HypothesisIndexOutOfRange,
                        ),
                    ));

                    false
                } else {
                    true
                }
            }
        }
    }

    // TODO: Remove.
    fn finish(&self) -> ProofBlockJustification {
        match self {
            Self::SystemChild(justification) => justification.finish(),
            Self::Macro(justification) => justification.finish(),
            Self::Hypothesis(id) => ProofBlockJustification::Hypothesis(*id),
        }
    }
}

#[derive(Debug)]
struct ProofBuilderMeta<'a> {
    justifications: Vec<ProofJustificationBuilder<'a>>,
    tags: Vec<String>,

    justification_verified: Cell<bool>,
    tag_verified: Cell<bool>,
}

impl<'a> ProofBuilderMeta<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::proof_meta);

        let mut justifications = Vec::with_capacity(1);
        let mut tags = Vec::with_capacity(1);

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::proof_justification => justifications.push(
                    ProofJustificationBuilder::from_pest(pair.into_inner().next().unwrap()),
                ),
                Rule::integer => {
                    justifications.push(ProofJustificationBuilder::hypothesis_from_pest(pair))
                }
                Rule::tag => tags.push(pair.into_inner().next().unwrap().as_str().to_owned()),

                _ => unreachable!(),
            }
        }

        ProofBuilderMeta {
            justifications,
            tags,

            justification_verified: Cell::new(false),
            tag_verified: Cell::new(false),
        }
    }

    fn build_tag_index(
        &'a self,
        proof_ref: &'a ProofBuilder<'a>,
        step_ref: &'a ProofBuilderStep<'a>,
        tags: &mut HashMap<&'a str, &'a ProofBuilderStep<'a>>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        assert!(!self.justification_verified.get());
        assert!(!self.tag_verified.get());
        let mut found_error = false;

        // FIXME: This is too messy. Clean it up.
        match self.tags.len() {
            0 => {}
            1 => match tags.entry(&self.tags[0]) {
                Entry::Occupied(old_step) => {
                    found_error = true;

                    errors.err(ParsingError::ProofError(
                        proof_ref,
                        ProofParsingError::StepError(
                            step_ref,
                            ProofStepParsingError::TagAlreadyTaken(*old_step.get()),
                        ),
                    ));
                }

                Entry::Vacant(slot) => {
                    slot.insert(step_ref);
                }
            },

            _ => {
                found_error = true;

                errors.err(ParsingError::ProofError(
                    proof_ref,
                    ProofParsingError::StepError(step_ref, ProofStepParsingError::DuplicateTags),
                ));
            }
        }

        self.tag_verified.set(!found_error);
    }

    fn verify_structure(
        &self,
        theorem_ref: &'a TheoremBuilder<'a>,
        proof_ref: &'a ProofBuilder<'a>,
        step_ref: &'a ProofBuilderStep<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        assert!(!self.justification_verified.get());
        assert!(self.tag_verified.get());
        let mut found_error = false;

        match self.justifications.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::ProofError(
                    proof_ref,
                    ProofParsingError::StepError(
                        step_ref,
                        ProofStepParsingError::MissingJustification,
                    ),
                ));
            }

            1 => {
                let success = self.justifications[0].verify_structure(
                    theorem_ref,
                    proof_ref,
                    step_ref,
                    index,
                    errors,
                );

                if !success {
                    found_error = true;
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::ProofError(
                    proof_ref,
                    ProofParsingError::StepError(
                        step_ref,
                        ProofStepParsingError::MissingJustification,
                    ),
                ));
            }
        }

        self.justification_verified.set(!found_error);
    }

    fn justification(&'a self) -> &ProofJustificationBuilder {
        assert!(self.justification_verified.get());
        &self.justifications[0]
    }
}

#[derive(Debug)]
pub struct ProofBuilderStep<'a> {
    meta: ProofBuilderMeta<'a>,
    formula: DisplayFormulaBuilder<'a>,
    end: String,

    // TODO: Remove.
    count: usize,

    // TODO: Remove.
    id: OnceCell<String>,

    // TODO: Remove.
    href: OnceCell<String>,
}

impl<'a> ProofBuilderStep<'a> {
    fn from_pest(pair: Pair<Rule>, count: usize) -> Self {
        assert_eq!(pair.as_rule(), Rule::proof_step);

        let mut inner = pair.into_inner();
        let meta = ProofBuilderMeta::from_pest(inner.next().unwrap());
        let formula = DisplayFormulaBuilder::from_pest(inner.next().unwrap());

        let end_pair = inner.next().unwrap();
        let end_inner = end_pair.into_inner().next().unwrap();
        let end = end_inner.as_str().to_owned();

        ProofBuilderStep {
            meta,
            formula,
            end,

            count,
            id: OnceCell::new(),
            href: OnceCell::new(),
        }
    }

    fn build_tag_index(
        &'a self,
        proof_ref: &'a ProofBuilder<'a>,
        tags: &mut HashMap<&'a str, &'a ProofBuilderStep<'a>>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.meta.build_tag_index(proof_ref, self, tags, errors);
    }

    fn verify_structure(
        &'a self,
        theorem_ref: &'a TheoremBuilder<'a>,
        proof_ref: &'a ProofBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.meta
            .verify_structure(theorem_ref, proof_ref, self, index, errors);
    }

    fn build(
        &'a self,
        proof_ref: &'a ProofBuilder<'a>,
        local_index: &LocalBuilderIndex<'a, '_>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.formula.build(local_index, errors, |formula, e| {
            ParsingError::ProofError(
                proof_ref,
                ProofParsingError::StepError(self, ProofStepParsingError::FormulaError(formula, e)),
            )
        });
    }

    // TODO: Remove.
    fn set_href(
        &self,
        book_id: &str,
        chapter_id: &str,
        page_id: &str,
        theorem_ref: &TheoremBuilder,
    ) {
        // FIXME: This href is non-unique if there are two proofs of the same theorem on a single
        // page.
        let id = format!(
            "{}_{}_proof_{}",
            theorem_ref.system_id(),
            theorem_ref.id(),
            self.count
        );
        let href = format!("/{}/{}/{}#{}", book_id, chapter_id, page_id, &id);
        self.id.set(id).unwrap();
        self.href.set(href).unwrap();
    }

    // TODO: Remove.
    pub fn get_ref(&self) -> ProofBlockStepRef {
        ProofBlockStepRef::new(self.count)
    }

    // TODO: Remove.
    fn finish(&'a self) -> ProofBlockStep {
        let id = self.id.get().unwrap().to_owned();
        let href = self.href.get().unwrap().to_owned();
        let justification = self.meta.justification().finish();
        let formula = self.formula.finish();
        let end = self.end.clone();

        ProofBlockStep::new(id, href, justification, formula, end)
    }
}

#[derive(Debug)]
enum ProofBuilderElement<'a> {
    Text(TextBuilder<'a>),
    Step(ProofBuilderStep<'a>),
}

impl<'a> ProofBuilderElement<'a> {
    fn from_pest(pair: Pair<Rule>, count: &mut usize) -> Self {
        match pair.as_rule() {
            Rule::text_block => Self::Text(TextBuilder::from_pest(pair)),
            Rule::proof_step => {
                let curr_count = *count;
                *count += 1;
                Self::Step(ProofBuilderStep::from_pest(pair, curr_count))
            }

            _ => unreachable!(),
        }
    }

    fn build_tag_index(
        &'a self,
        proof_ref: &'a ProofBuilder<'a>,
        tags: &mut HashMap<&'a str, &'a ProofBuilderStep<'a>>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self {
            Self::Text(_) => {}

            Self::Step(step) => step.build_tag_index(proof_ref, tags, errors),
        }
    }

    fn verify_structure(
        &'a self,
        theorem_ref: &'a TheoremBuilder<'a>,
        proof_ref: &'a ProofBuilder<'a>,
        index: &BuilderIndex<'a>,
        tags: &HashMap<&str, &'a ProofBuilderStep<'a>>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self {
            Self::Text(text) => {
                text.verify_structure_with_tags(index, tags, errors, |e| {
                    ParsingError::ProofError(proof_ref, ProofParsingError::TextError(text, e))
                });
            }

            Self::Step(step) => step.verify_structure(theorem_ref, proof_ref, index, errors),
        }
    }

    fn bib_refs(&self) -> Option<impl Iterator<Item = &BibliographyBuilderEntry>> {
        match self {
            Self::Text(text) => Some(text.bib_refs()),
            Self::Step(_) => None,
        }
    }

    fn set_local_bib_refs(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        if let Self::Text(text) = self {
            text.set_local_bib_refs(index);
        }
    }

    fn build_formulas(
        &'a self,
        proof_ref: &'a ProofBuilder<'a>,
        local_index: &LocalBuilderIndex<'a, '_>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self {
            Self::Text(_) => {}
            Self::Step(step) => step.build(proof_ref, local_index, errors),
        }
    }

    // TODO: Remove.
    fn set_href(
        &self,
        book_id: &str,
        chapter_id: &str,
        page_id: &str,
        theorem_ref: &TheoremBuilder,
    ) {
        match self {
            Self::Text(_) => {}
            Self::Step(step) => step.set_href(book_id, chapter_id, page_id, theorem_ref),
        }
    }

    // TODO: Remove.
    fn finish(&'a self, steps: &mut Vec<ProofBlockStep>) -> ProofBlockElement {
        match self {
            Self::Text(text) => ProofBlockElement::Text(text.finish()),
            Self::Step(step) => {
                steps.push(step.finish());
                ProofBlockElement::Step
            }
        }
    }
}

#[derive(Debug)]
pub struct ProofBuilder<'a> {
    system_id: String,
    theorem_id: String,
    serial: usize,

    elements: Vec<ProofBuilderElement<'a>>,

    theorem_ref: OnceCell<&'a TheoremBuilder<'a>>,

    // TODO: Remove.
    count: OnceCell<usize>,
}

impl<'a> ProofBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>, serial: usize) -> Self {
        assert_eq!(pair.as_rule(), Rule::proof_block);

        let mut inner = pair.into_inner();
        let theorem_id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();

        let mut count = 0;
        let elements = inner
            .map(|pair| ProofBuilderElement::from_pest(pair, &mut count))
            .collect();

        ProofBuilder {
            system_id,
            theorem_id,
            serial,

            elements,

            theorem_ref: OnceCell::new(),

            count: OnceCell::new(),
        }
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        let theorem_ref =
            if let Some(child) = index.search_system_child(&self.system_id, &self.theorem_id) {
                if let Some(theorem_ref) = child.theorem() {
                    theorem_ref
                } else {
                    errors.err(ParsingError::ProofError(
                        self,
                        ProofParsingError::ParentNotTheorem,
                    ));
                    return;
                }
            } else {
                errors.err(ParsingError::ProofError(
                    self,
                    ProofParsingError::ParentNotFound,
                ));
                return;
            };
        self.theorem_ref.set(theorem_ref).unwrap();
        theorem_ref.add_proof(self);

        // TODO: Make a TagIndex struct.
        let mut tags = HashMap::new();
        for element in &self.elements {
            element.build_tag_index(self, &mut tags, errors);
        }
        for element in &self.elements {
            element.verify_structure(theorem_ref, self, index, &tags, errors);
        }
    }

    pub fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        Box::new(
            self.elements
                .iter()
                .filter_map(ProofBuilderElement::bib_refs)
                .flatten(),
        )
    }

    pub fn set_local_bib_refs(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        for element in &self.elements {
            element.set_local_bib_refs(index);
        }
    }

    pub fn build_formulas(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        let theorem = self.theorem_ref.get().unwrap();
        let local_index = index.get_local(theorem.system_id(), theorem.vars());

        for element in &self.elements {
            element.build_formulas(self, &local_index, errors);
        }
    }

    // TODO: Remove.
    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap();
    }

    // TODO: Remove.
    pub fn set_href(&self, book_id: &str, chapter_id: &str, page_id: &str) {
        let theorem_ref = self.theorem_ref.get().unwrap();
        for element in &self.elements {
            element.set_href(book_id, chapter_id, page_id, theorem_ref);
        }
    }

    // TODO: Remove.
    pub fn get_ref(&self) -> ProofBlockRef {
        ProofBlockRef::new(*self.count.get().unwrap())
    }

    // TODO: Remove.
    pub fn finish(&'a self) -> ProofBlock {
        let self_ref = self.get_ref();
        let theorem_ref = self.theorem_ref.get().unwrap().get_ref();
        let mut steps = Vec::new();
        let elements = self
            .elements
            .iter()
            .map(|element| element.finish(&mut steps))
            .collect();

        ProofBlock::new(self_ref, theorem_ref, steps, elements)
    }
}
