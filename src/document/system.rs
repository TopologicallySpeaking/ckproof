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

use std::lazy::OnceCell;

use crate::FileLocation;

use crate::rendered::{
    AxiomRendered, ProofRendered, ProofRenderedElement, ProofRenderedJustification,
    ProofRenderedStep, SystemRendered,
};

use crate::core::errors::CheckingError;
use crate::core::system::{Axiom, Proof, ProofJustification, ProofStep, System, Theorem};
use crate::rendered::TheoremRendered;

use super::errors::{DocumentCheckingError, DocumentCheckingErrorContext};
use super::language::{DisplayFormulaBlock, FormulaBlock, VariableBlock};
use super::structure::{DeductableBlockRef, SystemBlockRef, TheoremBlockRef};
use super::text::{BareText, MathBlock, Paragraph, Text};
use super::Document;

pub struct SystemBlock<'a> {
    id: String,
    name: String,
    tagline: Paragraph<'a>,
    description: Vec<Text<'a>>,

    checkable: System,

    // TODO: Remove.
    href: String,
}

impl<'a> SystemBlock<'a> {
    pub fn new(
        id: String,
        name: String,
        tagline: Paragraph<'a>,
        description: Vec<Text<'a>>,
        href: String,
    ) -> Self {
        let checkable = System::new(id.clone());

        SystemBlock {
            id,
            name,
            tagline,
            description,

            checkable,

            href,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.tagline.crosslink(document);

        for text in &self.description {
            text.crosslink(document);
        }
    }

    pub fn checkable(&self) -> &System {
        &self.checkable
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    // TODO: Remove.
    pub fn href(&self) -> &str {
        &self.href
    }

    // TODO: Remove.
    pub fn render(&self) -> SystemRendered {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render();
        let description = self.description.iter().map(Text::render).collect();

        SystemRendered::new(id, name, tagline, description)
    }
}

impl<'a> std::fmt::Debug for SystemBlock<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub struct AxiomBlock<'a> {
    id: String,
    name: String,

    system_ref: SystemBlockRef<'a>,

    tagline: Paragraph<'a>,
    description: Vec<Text<'a>>,

    vars: Vec<VariableBlock<'a>>,
    premise: Vec<DisplayFormulaBlock<'a>>,
    assertion: DisplayFormulaBlock<'a>,

    checkable: Axiom<'a>,

    // TODO: Remove.
    href: String,
}

impl<'a> AxiomBlock<'a> {
    pub fn new(
        id: String,
        name: String,
        system_ref: SystemBlockRef<'a>,
        tagline: Paragraph<'a>,
        description: Vec<Text<'a>>,
        vars: Vec<VariableBlock<'a>>,
        premise: Vec<DisplayFormulaBlock<'a>>,
        assertion: DisplayFormulaBlock<'a>,
        href: String,
    ) -> Self {
        let checkable = Axiom::new(id.clone());

        AxiomBlock {
            id,
            name,

            system_ref,

            tagline,
            description,

            vars,
            premise,
            assertion,

            checkable,

            href,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.system_ref.crosslink(document);
        self.checkable.set_system(self.system_ref.checkable());

        self.tagline.crosslink(document);
        for text in &self.description {
            text.crosslink(document);
        }

        for var in &self.vars {
            var.crosslink(document);
        }

        for hypothesis in &self.premise {
            hypothesis.crosslink(document, &self.vars);
        }
        self.checkable.set_premise(
            self.premise
                .iter()
                .map(DisplayFormulaBlock::checkable)
                .collect(),
        );

        self.assertion.crosslink(document, &self.vars);
        self.checkable.set_assertion(self.assertion.checkable());
    }

    pub fn verify(&self) {
        assert!(self.checkable.verify());
    }

    pub fn checkable(&'a self) -> &Axiom {
        &self.checkable
    }

    // TODO: Remove.
    pub fn render(&self) -> AxiomRendered {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render();
        let description = self.description.iter().map(Text::render).collect();
        let premise = self
            .premise
            .iter()
            .map(DisplayFormulaBlock::render)
            .collect();
        let assertion = self.assertion.render();

        let system_id = self.system_ref.id().to_owned();
        let system_name = self.system_ref.name().to_owned();

        AxiomRendered::new(
            id,
            system_id,
            name,
            system_name,
            tagline,
            description,
            premise,
            assertion,
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn href(&self) -> &str {
        &self.href
    }
}

impl<'a> std::fmt::Debug for AxiomBlock<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Clone, Copy)]
pub enum TheoremKind {
    Lemma,
    Theorem,
}

impl TheoremKind {
    fn render(&self) -> &str {
        match self {
            Self::Lemma => "Lemma",
            Self::Theorem => "Theorem",
        }
    }
}

pub struct TheoremBlock<'a> {
    kind: TheoremKind,
    id: String,
    name: String,

    system_ref: SystemBlockRef<'a>,

    tagline: Paragraph<'a>,
    description: Vec<Text<'a>>,

    vars: Vec<VariableBlock<'a>>,
    premise: Vec<DisplayFormulaBlock<'a>>,
    assertion: DisplayFormulaBlock<'a>,

    checkable: Theorem<'a>,

    // TODO: Remove.
    href: String,
}

impl<'a> TheoremBlock<'a> {
    pub fn new(
        kind: TheoremKind,
        id: String,
        name: String,
        system_ref: SystemBlockRef<'a>,
        tagline: Paragraph<'a>,
        description: Vec<Text<'a>>,
        vars: Vec<VariableBlock<'a>>,
        premise: Vec<DisplayFormulaBlock<'a>>,
        assertion: DisplayFormulaBlock<'a>,
        href: String,
    ) -> Self {
        let checkable = Theorem::new(id.clone());

        TheoremBlock {
            kind,
            id,
            name,

            system_ref,

            tagline,
            description,

            vars,
            premise,
            assertion,

            checkable,

            href,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.system_ref.crosslink(document);
        self.checkable.set_system(self.system_ref.checkable());

        self.tagline.crosslink(document);
        for text in &self.description {
            text.crosslink(document);
        }

        for var in &self.vars {
            var.crosslink(document);
        }

        for hypothesis in &self.premise {
            hypothesis.crosslink(document, &self.vars);
        }
        self.checkable.set_premise(
            self.premise
                .iter()
                .map(DisplayFormulaBlock::checkable)
                .collect(),
        );

        self.assertion.crosslink(document, &self.vars);
        self.checkable.set_assertion(self.assertion.checkable());
    }

    pub fn vars(&self) -> &[VariableBlock<'a>] {
        &self.vars
    }

    pub fn verify(&self) {
        assert!(self.checkable.verify());
    }

    pub fn checkable(&'a self) -> &Theorem {
        &self.checkable
    }

    // TODO: Remove.
    pub fn render(&self) -> TheoremRendered {
        let kind = self.kind.render().to_owned();
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render();
        let description = self.description.iter().map(Text::render).collect();
        let premise = self
            .premise
            .iter()
            .map(DisplayFormulaBlock::render)
            .collect();
        let assertion = self.assertion.render();

        let system_id = self.system_ref.id().to_owned();
        let system_name = self.system_ref.name().to_owned();

        TheoremRendered::new(
            kind,
            id,
            system_id,
            name,
            system_name,
            tagline,
            description,
            premise,
            assertion,
        )
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    // TODO: Remove.
    pub fn href(&self) -> &str {
        &self.href
    }
}

impl<'a> std::fmt::Debug for TheoremBlock<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub enum ProofBlockJustification<'a> {
    Deductable(DeductableBlockRef<'a>),
    Hypothesis(usize),

    Definition,
    Substitution,
}

impl<'a> ProofBlockJustification<'a> {
    fn crosslink(&'a self, document: &'a Document<'a>) {
        if let Self::Deductable(deductable_ref) = self {
            deductable_ref.crosslink(document);
        }
    }

    // TODO: Remove.
    fn render(&self) -> ProofRenderedJustification {
        match self {
            Self::Deductable(deductable_ref) => {
                let name = deductable_ref.name().to_owned();
                let href = deductable_ref.href().to_owned();

                ProofRenderedJustification::SystemChild(name, href)
            }

            Self::Hypothesis(id) => ProofRenderedJustification::Hypothesis(*id),

            Self::Definition => ProofRenderedJustification::Definition,
            Self::Substitution => ProofRenderedJustification::Substitution,
        }
    }

    pub fn deductable(&self) -> Option<&DeductableBlockRef<'a>> {
        match self {
            Self::Deductable(deductable_ref) => Some(deductable_ref),

            _ => None,
        }
    }
}

pub enum ProofBlockSmallJustification<'a> {
    Deductable(DeductableBlockRef<'a>),
    Hypothesis(usize),

    Definition,
}

impl<'a> ProofBlockSmallJustification<'a> {
    fn crosslink(&'a self, document: &'a Document<'a>) {
        if let Self::Deductable(deductable_ref) = self {
            deductable_ref.crosslink(document);
        }
    }

    fn checkable(&'a self) -> ProofJustification {
        match self {
            Self::Deductable(deductable_ref) => {
                ProofJustification::Deductable(deductable_ref.checkable())
            }
            Self::Hypothesis(i) => ProofJustification::Hypothesis(*i - 1),

            Self::Definition => ProofJustification::Definition,
        }
    }
}

pub struct ProofBlockSmallStep<'a> {
    justification: ProofBlockSmallJustification<'a>,
    formula: FormulaBlock<'a>,
}

impl<'a> ProofBlockSmallStep<'a> {
    pub fn new(justification: ProofBlockSmallJustification<'a>, formula: FormulaBlock<'a>) -> Self {
        ProofBlockSmallStep {
            justification,
            formula,
        }
    }

    fn crosslink(&'a self, document: &'a Document<'a>, vars: &'a [VariableBlock<'a>]) {
        self.justification.crosslink(document);
        self.formula.crosslink(document, vars);
    }

    fn checkable(&'a self) -> ProofStep {
        let justification = self.justification.checkable();
        let formula = self.formula.checkable();

        ProofStep::new(justification, formula)
    }
}

pub struct ProofBlockStep<'a> {
    file_location: FileLocation,

    justification: ProofBlockJustification<'a>,
    small_steps: Vec<ProofBlockSmallStep<'a>>,
    formula: MathBlock,
    end: String,

    // TODO: Remove.
    id: String,

    // TODO: Remove.
    href: String,

    // TODO: Remove.
    tag: usize,
}

impl<'a> ProofBlockStep<'a> {
    pub fn new(
        file_location: FileLocation,
        justification: ProofBlockJustification<'a>,
        small_steps: Vec<ProofBlockSmallStep<'a>>,
        formula: MathBlock,
        end: String,
        id: String,
        href: String,
        tag: usize,
    ) -> Self {
        ProofBlockStep {
            file_location,

            justification,
            small_steps,
            formula,
            end,

            id,

            href,

            tag,
        }
    }

    fn crosslink(&'a self, document: &'a Document<'a>, vars: &'a [VariableBlock<'a>]) {
        self.justification.crosslink(document);

        for step in &self.small_steps {
            step.crosslink(document, vars);
        }
    }

    fn checkable(&'a self) -> impl Iterator<Item = ProofStep> {
        self.small_steps.iter().map(ProofBlockSmallStep::checkable)
    }

    // TODO: Remove.
    fn href(&self) -> &str {
        &self.href
    }

    // TODO: Remove.
    fn render(&self) -> ProofRenderedStep {
        let id = self.id.clone();
        let justification = self.justification.render();
        let formula = self.formula.render();
        let end = self.end.clone();
        let tag = self.tag;

        ProofRenderedStep::new(id, justification, formula, end, tag)
    }

    pub fn file_location(&self) -> &FileLocation {
        &self.file_location
    }

    pub fn justification(&self) -> &ProofBlockJustification<'a> {
        &self.justification
    }

    fn num_small_steps(&self) -> usize {
        self.small_steps.len()
    }
}

impl<'a> std::fmt::Debug for ProofBlockStep<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub struct ProofBlockStepRef<'a> {
    index: usize,

    step: OnceCell<&'a ProofBlockStep<'a>>,
}

impl<'a> ProofBlockStepRef<'a> {
    pub fn new(index: usize) -> Self {
        ProofBlockStepRef {
            index,

            step: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, proof_ref: &'a ProofBlock<'a>) {
        let step = proof_ref.elements[self.index].step().unwrap();
        self.step.set(step).unwrap();
    }

    // TODO: Remove.
    pub fn render(&self, text: Option<&BareText>) -> String {
        let proof_step = self.step.get().unwrap();

        format!("<a href=\"{}\">({})</a>", proof_step.href(), proof_step.tag)
    }
}

pub enum ProofBlockElement<'a> {
    Text(Text<'a>),
    Step(ProofBlockStep<'a>),
}

impl<'a> ProofBlockElement<'a> {
    fn crosslink(
        &'a self,
        document: &'a Document<'a>,
        vars: &'a [VariableBlock<'a>],
        proof_ref: &'a ProofBlock<'a>,
    ) {
        match self {
            Self::Text(text) => text.crosslink_proof(document, proof_ref),
            Self::Step(step) => step.crosslink(document, vars),
        }
    }

    fn step(&self) -> Option<&ProofBlockStep<'a>> {
        match self {
            Self::Step(step) => Some(step),

            _ => None,
        }
    }

    fn checkable(&'a self) -> Option<impl Iterator<Item = ProofStep>> {
        self.step().map(ProofBlockStep::checkable)
    }

    // TODO: Remove.
    fn render(&self) -> ProofRenderedElement {
        match self {
            Self::Text(text_ref) => ProofRenderedElement::Text(text_ref.render()),
            Self::Step(step_ref) => ProofRenderedElement::Step(step_ref.render()),
        }
    }
}

pub struct ProofBlock<'a> {
    theorem_ref: TheoremBlockRef<'a>,

    elements: Vec<ProofBlockElement<'a>>,

    checkable: OnceCell<Proof<'a>>,
}

impl<'a> ProofBlock<'a> {
    pub fn new(theorem_ref: TheoremBlockRef<'a>, elements: Vec<ProofBlockElement<'a>>) -> Self {
        ProofBlock {
            theorem_ref,

            elements,

            checkable: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.theorem_ref.crosslink(document);
        let vars = self.theorem_ref.vars();

        for element in &self.elements {
            element.crosslink(document, vars, self);
        }

        let theorem = self.theorem_ref.checkable();
        let steps = self
            .elements
            .iter()
            .filter_map(ProofBlockElement::checkable)
            .flatten()
            .collect();
        self.checkable.set(Proof::new(theorem, steps)).unwrap();
    }

    pub fn verify(&self) {
        assert!(self.checkable.get().unwrap().verify());
    }

    fn get_step(&self, i: usize) -> Result<&ProofBlockStep<'a>, Option<&ProofBlockStep<'a>>> {
        let mut counter = 0;
        for element in self.elements.iter().filter_map(ProofBlockElement::step) {
            let num_small_steps = element.num_small_steps();

            if counter == i {
                return Ok(element);
            } else if counter + num_small_steps > i {
                return Err(Some(element));
            } else {
                counter += num_small_steps
            }
        }

        Err(None)
    }

    pub fn check(&'a self, errors: &mut DocumentCheckingErrorContext<'a>) {
        for error in self.checkable.get().unwrap().check() {
            match error {
                CheckingError::DeductableAssertionNotSubstitutable(i) => match self.get_step(i) {
                    Ok(step) => errors.err(
                        DocumentCheckingError::DeductableAssertionNotSubstitutable(self, step),
                    ),
                    Err(step) => todo!(),
                },

                _ => todo!("{:#?}", error),
            }
        }
    }

    // TODO: Remove.
    pub fn render(&self) -> ProofRendered {
        let theorem_name = self.theorem_ref.name().to_owned();
        let elements = self
            .elements
            .iter()
            .map(ProofBlockElement::render)
            .collect();

        ProofRendered::new(theorem_name, elements)
    }

    pub fn theorem_name(&self) -> &str {
        self.theorem_ref.name()
    }
}
