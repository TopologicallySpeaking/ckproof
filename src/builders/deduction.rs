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

use std::cell::Cell;

use pest::iterators::{Pair, Pairs};

use crate::document::deduction::{
    AxiomBlock, ProofBlock, ProofBlockElement, ProofBlockJustification, ProofBlockStep,
    TheoremBlock,
};

use super::directory::{
    AxiomBuilderRef, BuilderDirectory, LocalIndex, ProofBuilderRef, ProofBuilderStepRef,
    SystemBuilderChild, SystemBuilderRef, TagIndex, TheoremBuilderRef,
};
use super::errors::{ParsingError, ParsingErrorContext};
use super::language::{FormulaBuilder, VariableBuilder};
use super::text::{ParagraphBuilder, TextBuilder};
use super::{BlockLocation, Rule};

struct AxiomBuilderEntries {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder>,
    descriptions: Vec<Vec<TextBuilder>>,

    vars: Vec<VariableBuilder>,
    premises: Vec<Vec<FormulaBuilder>>,
    assertions: Vec<FormulaBuilder>,

    verified: Cell<bool>,
}

impl AxiomBuilderEntries {
    fn from_pest(pairs: Pairs<Rule>) -> AxiomBuilderEntries {
        let mut names = Vec::with_capacity(1);
        let mut taglines = Vec::with_capacity(1);
        let mut descriptions = Vec::with_capacity(1);
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
                Rule::var_declaration => vars.push(VariableBuilder::from_pest(pair)),
                Rule::premise => {
                    let premise = pair.into_inner().map(FormulaBuilder::from_pest).collect();

                    premises.push(premise);
                }
                Rule::assertion => {
                    let assertion = FormulaBuilder::from_pest(pair.into_inner().next().unwrap());

                    assertions.push(assertion);
                }

                _ => unreachable!(),
            }
        }

        AxiomBuilderEntries {
            names,
            taglines,
            descriptions,

            vars,
            premises,
            assertions,

            verified: Cell::new(false),
        }
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        self_ref: AxiomBuilderRef,
        min_serial: BlockLocation,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.names.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::AxiomMissingName(self_ref));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::AxiomDuplicateName(self_ref));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::AxiomMissingTagline(self_ref));
            }

            1 => self.taglines[0].verify_structure(directory, errors),

            _ => {
                found_error = true;
                errors.err(ParsingError::AxiomDuplicateTagline(self_ref));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for paragraph in &self.descriptions[0] {
                    paragraph.verify_structure(directory, errors)
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::AxiomDuplicateDescription(self_ref))
            }
        }

        for var in &self.vars {
            var.verify_structure(parent_system, min_serial, directory, errors);
        }

        self.verified.set(!found_error);
    }

    fn build_formulas(
        &self,
        self_ref: AxiomBuilderRef,
        parent_system: &str,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(self.verified.get());

        let local_index = {
            let mut tmp = directory.get_local(parent_system);
            tmp.add_vars(self_ref.into(), &self.vars, errors);
            tmp
        };

        if !self.premises.is_empty() {
            for formula in &self.premises[0] {
                formula.build(&local_index, directory, &self.vars, errors);
            }
        }

        self.assertions[0].build(&local_index, directory, &self.vars, errors);
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&self) -> &ParagraphBuilder {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&self) -> &[TextBuilder] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }

    fn vars(&self) -> &[VariableBuilder] {
        assert!(self.verified.get());
        &self.vars
    }

    fn premise(&self) -> &[FormulaBuilder] {
        assert!(self.verified.get());
        if self.premises.is_empty() {
            &[]
        } else {
            &self.premises[0]
        }
    }

    fn assertion(&self) -> &FormulaBuilder {
        assert!(self.verified.get());
        &self.assertions[0]
    }
}

pub struct AxiomBuilder {
    id: String,
    system_id: String,
    href: String,
    serial: BlockLocation,

    entries: AxiomBuilderEntries,

    self_ref: Option<AxiomBuilderRef>,
    system_ref: Cell<Option<SystemBuilderRef>>,
}

impl AxiomBuilder {
    pub fn from_pest(pair: Pair<Rule>, serial: BlockLocation, href: &str) -> AxiomBuilder {
        assert_eq!(pair.as_rule(), Rule::axiom_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();
        let href = format!("{}#{}_{}", href, system_id, id);

        let entries = AxiomBuilderEntries::from_pest(inner);

        AxiomBuilder {
            id,
            system_id,
            href,
            serial,

            entries,

            self_ref: None,
            system_ref: Cell::new(None),
        }
    }

    pub fn set_self_ref(&mut self, axiom_ref: AxiomBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(axiom_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        assert!(self.system_ref.get().is_none());
        let self_ref = self.self_ref.unwrap();

        self.system_ref
            .set(directory.search_system(&self.system_id));
        if self.system_ref.get().is_none() {
            errors.err(ParsingError::AxiomParentNotFound(self_ref));
        }

        self.entries
            .verify_structure(&self.system_id, self_ref, self.serial, directory, errors);
    }

    pub fn finish(&self) -> AxiomBlock {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();
        let system = self.system_ref.get().unwrap().finish();
        let href = self.href.clone();
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
            .map(FormulaBuilder::finish)
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

    pub fn build_formulas(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        self.entries
            .build_formulas(self.self_ref.unwrap(), &self.system_id, directory, errors);
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn system_id(&self) -> &str {
        &self.system_id
    }
}

struct TheoremBuilderEntries {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder>,
    descriptions: Vec<Vec<TextBuilder>>,

    vars: Vec<VariableBuilder>,
    premises: Vec<Vec<FormulaBuilder>>,
    assertions: Vec<FormulaBuilder>,

    verified: Cell<bool>,
}

impl TheoremBuilderEntries {
    fn from_pest(pairs: Pairs<Rule>) -> TheoremBuilderEntries {
        let mut names = Vec::with_capacity(1);
        let mut taglines = Vec::with_capacity(1);
        let mut descriptions = Vec::with_capacity(1);
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
                Rule::var_declaration => vars.push(VariableBuilder::from_pest(pair)),
                Rule::premise => {
                    let premise = pair.into_inner().map(FormulaBuilder::from_pest).collect();

                    premises.push(premise);
                }
                Rule::assertion => {
                    let assertion = FormulaBuilder::from_pest(pair.into_inner().next().unwrap());

                    assertions.push(assertion);
                }

                _ => unreachable!(),
            }
        }

        TheoremBuilderEntries {
            names,
            taglines,
            descriptions,

            vars,
            premises,
            assertions,

            verified: Cell::new(false),
        }
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        self_ref: TheoremBuilderRef,
        min_serial: BlockLocation,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.names.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::TheoremMissingName(self_ref));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::TheoremDuplicateName(self_ref));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::TheoremMissingTagline(self_ref));
            }

            1 => self.taglines[0].verify_structure(directory, errors),

            _ => {
                found_error = true;
                errors.err(ParsingError::TheoremDuplicateTagline(self_ref));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for paragraph in &self.descriptions[0] {
                    paragraph.verify_structure(directory, errors)
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::TheoremDuplicateDescription(self_ref))
            }
        }

        for var in &self.vars {
            var.verify_structure(parent_system, min_serial, directory, errors);
        }

        self.verified.set(!found_error);
    }

    fn build_formulas(
        &self,
        self_ref: TheoremBuilderRef,
        parent_system: &str,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(self.verified.get());

        let local_index = {
            let mut tmp = directory.get_local(parent_system);
            tmp.add_vars(self_ref.into(), &self.vars, errors);
            tmp
        };

        if !self.premises.is_empty() {
            for formula in &self.premises[0] {
                formula.build(&local_index, directory, &self.vars, errors);
            }
        }

        self.assertions[0].build(&local_index, directory, &self.vars, errors);
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&self) -> &ParagraphBuilder {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&self) -> &[TextBuilder] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }

    fn vars(&self) -> &[VariableBuilder] {
        assert!(self.verified.get());
        &self.vars
    }

    fn premise(&self) -> &[FormulaBuilder] {
        assert!(self.verified.get());
        if self.premises.is_empty() {
            &[]
        } else {
            &self.premises[0]
        }
    }

    fn assertion(&self) -> &FormulaBuilder {
        assert!(self.verified.get());
        &self.assertions[0]
    }
}

pub struct TheoremBuilder {
    id: String,
    system_id: String,
    href: String,
    serial: BlockLocation,

    entries: TheoremBuilderEntries,

    self_ref: Option<TheoremBuilderRef>,
    system_ref: Cell<Option<SystemBuilderRef>>,
}

impl TheoremBuilder {
    pub fn from_pest(pair: Pair<Rule>, serial: BlockLocation, href: &str) -> TheoremBuilder {
        assert_eq!(pair.as_rule(), Rule::theorem_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();
        let href = format!("{}#{}_{}", href, system_id, id);

        let entries = TheoremBuilderEntries::from_pest(inner);

        TheoremBuilder {
            id,
            system_id,
            href,
            serial,

            entries,

            self_ref: None,
            system_ref: Cell::new(None),
        }
    }

    pub fn set_self_ref(&mut self, self_ref: TheoremBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(self_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        assert!(self.system_ref.get().is_none());
        let self_ref = self.self_ref.unwrap();

        self.system_ref
            .set(directory.search_system(&self.system_id));
        if self.system_ref.get().is_none() {
            errors.err(ParsingError::TheoremParentNotFound(self_ref));
        }

        self.entries
            .verify_structure(&self.system_id, self_ref, self.serial, directory, errors);
    }

    pub fn build_formulas(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        self.entries
            .build_formulas(self.self_ref.unwrap(), &self.system_id, directory, errors);
    }

    pub fn finish(&self) -> TheoremBlock {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();
        let system = self.system_ref.get().unwrap().finish();
        let href = self.href.clone();
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
            .map(FormulaBuilder::finish)
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

    pub fn premise(&self) -> &[FormulaBuilder] {
        self.entries.premise()
    }
}

struct SystemChildJustificationBuilder {
    id: String,

    child: Cell<Option<SystemBuilderChild>>,
}

impl SystemChildJustificationBuilder {
    fn from_pest(pair: Pair<Rule>) -> SystemChildJustificationBuilder {
        assert_eq!(pair.as_rule(), Rule::ident);

        let id = pair.as_str().to_owned();

        SystemChildJustificationBuilder {
            id,

            child: Cell::new(None),
        }
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        step_ref: ProofBuilderStepRef,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        self.child
            .set(directory.search_system_child(parent_system, &self.id));

        match self.child.get() {
            None => errors.err(ParsingError::ProofStepSystemChildJustificationNotFound(
                step_ref,
            )),

            Some(SystemBuilderChild::Axiom(_)) | Some(SystemBuilderChild::Theorem(_)) => {}

            _ => errors.err(ParsingError::ProofStepSystemChildJustificationWrongKind(
                step_ref,
            )),
        }
    }

    fn finish(&self) -> ProofBlockJustification {
        match self.child.get().unwrap() {
            SystemBuilderChild::Axiom(axiom_ref) => {
                ProofBlockJustification::Axiom(axiom_ref.finish())
            }
            SystemBuilderChild::Theorem(theorem_ref) => {
                ProofBlockJustification::Theorem(theorem_ref.finish())
            }

            _ => unreachable!(),
        }
    }
}

enum ProofJustificationBuilder {
    SystemChild(SystemChildJustificationBuilder),
    Hypothesis(usize),
}

impl ProofJustificationBuilder {
    fn from_pest(pair: Pair<Rule>) -> ProofJustificationBuilder {
        assert_eq!(pair.as_rule(), Rule::proof_justification);
        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::ident => Self::SystemChild(SystemChildJustificationBuilder::from_pest(pair)),

            _ => unreachable!(),
        }
    }

    fn hypothesis_from_pest(pair: Pair<Rule>) -> ProofJustificationBuilder {
        assert_eq!(pair.as_rule(), Rule::integer);

        ProofJustificationBuilder::Hypothesis(pair.as_str().parse().unwrap_or_else(|_| todo!()))
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        step_ref: ProofBuilderStepRef,
        theorem_ref: TheoremBuilderRef,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        match self {
            Self::SystemChild(builder) => {
                builder.verify_structure(parent_system, step_ref, directory, errors)
            }

            Self::Hypothesis(id) => {
                let premise_len = directory[theorem_ref].premise().len();

                if *id == 0 {
                    todo!()
                }

                if *id > premise_len {
                    todo!()
                }
            }
        }
    }

    fn finish(&self) -> ProofBlockJustification {
        match self {
            Self::SystemChild(builder) => builder.finish(),

            Self::Hypothesis(id) => ProofBlockJustification::Hypothesis(*id),
        }
    }
}

struct ProofBuilderMeta {
    justifications: Vec<ProofJustificationBuilder>,
    tags: Vec<String>,

    self_ref: Cell<Option<ProofBuilderStepRef>>,
    verified: Cell<bool>,
}

impl ProofBuilderMeta {
    fn from_pest(pair: Pair<Rule>) -> ProofBuilderMeta {
        assert_eq!(pair.as_rule(), Rule::proof_meta);

        let mut justifications = Vec::with_capacity(1);
        let mut tags = Vec::with_capacity(1);

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::proof_justification => {
                    justifications.push(ProofJustificationBuilder::from_pest(pair))
                }
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

            self_ref: Cell::new(None),
            verified: Cell::new(false),
        }
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        self_ref: ProofBuilderStepRef,
        theorem_ref: TheoremBuilderRef,
        directory: &BuilderDirectory,
        tags: &mut TagIndex,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.justifications.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::ProofStepMissingJustification(self_ref));
            }

            1 => self.justifications[0].verify_structure(
                parent_system,
                self_ref,
                theorem_ref,
                directory,
                errors,
            ),

            _ => {
                found_error = true;
                errors.err(ParsingError::ProofStepDuplicateJustification(self_ref));
            }
        }

        match self.tags.len() {
            0 => {}
            1 => tags.add_tag(&self.tags[0], self_ref, errors),

            _ => {
                found_error = true;
                errors.err(ParsingError::ProofStepDuplicateTags(self_ref));
            }
        }

        self.self_ref.set(Some(self_ref));
        self.verified.set(!found_error);
    }

    fn justification(&self) -> &ProofJustificationBuilder {
        assert!(self.verified.get());
        &self.justifications[0]
    }
}

struct ProofBuilderStep {
    id: String,
    href: String,
    meta: ProofBuilderMeta,
    formula: FormulaBuilder,
    end: String,
}

impl ProofBuilderStep {
    fn from_pest(pair: Pair<Rule>, id: String, href: String) -> ProofBuilderStep {
        assert_eq!(pair.as_rule(), Rule::proof_step);

        let mut inner = pair.into_inner();
        let meta = ProofBuilderMeta::from_pest(inner.next().unwrap());
        let formula = FormulaBuilder::from_pest(inner.next().unwrap());

        let end_pair = inner.next().unwrap();
        let end_inner = end_pair.into_inner().next().unwrap();
        let end = end_inner.as_str().to_owned();

        ProofBuilderStep {
            id,
            href,
            meta,
            formula,
            end,
        }
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        self_ref: ProofBuilderStepRef,
        theorem_ref: TheoremBuilderRef,
        directory: &BuilderDirectory,
        tags: &mut TagIndex,
        errors: &mut ParsingErrorContext,
    ) {
        self.meta.verify_structure(
            parent_system,
            self_ref,
            theorem_ref,
            directory,
            tags,
            errors,
        );
    }

    fn build_formulas(
        &self,
        local_index: &LocalIndex,
        directory: &BuilderDirectory,
        vars: &[VariableBuilder],
        errors: &mut ParsingErrorContext,
    ) {
        self.formula.build(local_index, directory, vars, errors);
    }

    fn finish(&self) -> ProofBlockStep {
        let id = self.id.clone();
        let href = self.href.clone();
        let justification = self.meta.justification().finish();
        let formula = self.formula.finish();
        let end = self.end.clone();

        ProofBlockStep::new(id, href, justification, formula, end)
    }
}

enum ProofBuilderElement {
    Text(TextBuilder),
    Step,
}

impl ProofBuilderElement {
    fn from_pest(
        pair: Pair<Rule>,
        system_id: &str,
        theorem_id: &str,
        href: &str,
        steps: &mut Vec<ProofBuilderStep>,
        step_counter: &mut usize,
    ) -> ProofBuilderElement {
        match pair.as_rule() {
            Rule::text_block => Self::Text(TextBuilder::from_pest(pair)),
            Rule::proof_step => {
                let id = format!("{}_{}_proof_{}", system_id, theorem_id, *step_counter + 1);
                let href = format!("{}_{}", href, *step_counter + 1);
                *step_counter += 1;

                steps.push(ProofBuilderStep::from_pest(pair, id, href));

                Self::Step
            }

            _ => unreachable!(),
        }
    }

    fn verify_structure(
        &self,
        directory: &BuilderDirectory,
        tags: &TagIndex,
        errors: &mut ParsingErrorContext,
    ) {
        match self {
            Self::Text(text) => text.verify_structure_with_tags(directory, tags, errors),
            Self::Step => {}
        }
    }

    fn finish(&self) -> ProofBlockElement {
        match self {
            Self::Text(text) => ProofBlockElement::Text(text.finish()),
            Self::Step => ProofBlockElement::Step,
        }
    }
}

pub struct ProofBuilder {
    theorem_id: String,
    system_id: String,

    elements: Vec<ProofBuilderElement>,
    steps: Vec<ProofBuilderStep>,

    self_ref: Option<ProofBuilderRef>,
    theorem_ref: Cell<Option<TheoremBuilderRef>>,
}

impl ProofBuilder {
    pub fn from_pest(pair: Pair<Rule>, href: &str) -> ProofBuilder {
        assert_eq!(pair.as_rule(), Rule::proof_block);

        let mut inner = pair.into_inner();
        let theorem_id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();
        // FIXME: This href is non-unique if there are two proofs of the same theorem on a single
        // page.
        let href = format!("{}#{}_{}_proof", href, system_id, theorem_id);

        let mut steps = Vec::new();
        let mut step_counter = 0;
        let elements = inner
            .map(|pair| {
                ProofBuilderElement::from_pest(
                    pair,
                    &system_id,
                    &theorem_id,
                    &href,
                    &mut steps,
                    &mut step_counter,
                )
            })
            .collect();

        ProofBuilder {
            theorem_id,
            system_id,

            steps,
            elements,

            self_ref: None,
            theorem_ref: Cell::new(None),
        }
    }

    pub fn set_self_ref(&mut self, proof_ref: ProofBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(proof_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        let self_ref = self.self_ref.unwrap();

        if let Some(child) = directory.search_system_child(&self.system_id, &self.theorem_id) {
            self.theorem_ref.set(child.theorem());

            if self.theorem_ref.get().is_none() {
                errors.err(ParsingError::ProofParentNotTheorem(self_ref));
            }
        } else {
            errors.err(ParsingError::ProofParentNotFound(self_ref));
        }

        let mut tags = TagIndex::new();
        for (i, step) in self.steps.iter().enumerate() {
            step.verify_structure(
                &self.system_id,
                self_ref.step(i),
                self.theorem_ref.get().unwrap(),
                directory,
                &mut tags,
                errors,
            );
        }

        for element in &self.elements {
            element.verify_structure(directory, &mut tags, errors);
        }
    }

    pub fn build_formulas(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        let theorem = &directory[self.theorem_ref.get().unwrap()];
        let vars = &theorem.entries.vars;

        let local_index = {
            let mut tmp = directory.get_local(&self.system_id);
            tmp.add_vars(self.self_ref.unwrap().into(), vars, errors);
            tmp
        };

        for step in &self.steps {
            step.build_formulas(&local_index, directory, vars, errors);
        }
    }

    pub fn finish(&self) -> ProofBlock {
        let theorem_ref = self.theorem_ref.get().unwrap().finish();
        let steps = self.steps.iter().map(ProofBuilderStep::finish).collect();
        let elements = self
            .elements
            .iter()
            .map(ProofBuilderElement::finish)
            .collect();

        ProofBlock::new(theorem_ref, steps, elements)
    }
}
