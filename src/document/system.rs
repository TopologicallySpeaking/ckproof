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

use crate::rendered::{
    AxiomRendered, ProofRendered, ProofRenderedElement, ProofRenderedJustification,
    ProofRenderedStep, SystemRendered,
};

use crate::deduction::directory::{
    AxiomRef, HypothesisRef, LocalCheckableDirectory, SystemRef, TheoremRef,
};
use crate::deduction::system::{Axiom, Proof, ProofJustification, ProofStep, System, Theorem};
use crate::rendered::TheoremRendered;

use super::language::{DisplayFormulaBlock, FormulaBlock, VariableBlock};
use super::structure::{DeductableBlockRef, SystemBlockRef, TheoremBlockRef};
use super::text::{BareText, MathBlock, Paragraph, Text};
use super::Document;

pub struct SystemBlock<'a> {
    id: String,
    name: String,
    tagline: Paragraph<'a>,
    description: Vec<Text<'a>>,

    // TODO: Remove.
    href: String,

    // TODO: Remove.
    count: OnceCell<usize>,
}

impl<'a> SystemBlock<'a> {
    pub fn new(
        id: String,
        name: String,
        tagline: Paragraph<'a>,
        description: Vec<Text<'a>>,
        href: String,
    ) -> Self {
        SystemBlock {
            id,
            name,
            tagline,
            description,

            href,

            count: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.tagline.crosslink(document);

        for text in &self.description {
            text.crosslink(document);
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    // TODO: Remove.
    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap()
    }

    // TODO: Remove.
    pub fn index(&self) -> usize {
        *self.count.get().unwrap()
    }

    // TODO: Remove.
    pub fn checkable(&self) -> System {
        let id = self.id.clone();

        System::new(id)
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

    // TODO: Remove.
    count: OnceCell<usize>,

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
        AxiomBlock {
            id,
            name,

            system_ref,

            tagline,
            description,

            vars,
            premise,
            assertion,

            count: OnceCell::new(),

            href,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.system_ref.crosslink(document);

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
        self.assertion.crosslink(document, &self.vars);
    }

    // TODO: Remove.
    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap()
    }

    // TODO: Remove.
    pub fn index(&self) -> usize {
        *self.count.get().unwrap()
    }

    // TODO: Remove.
    pub fn checkable(&self) -> Axiom {
        let id = self.id.clone();
        let system_ref = SystemRef::new(self.system_ref.index());
        let local_directory =
            LocalCheckableDirectory::new(self.vars.iter().map(VariableBlock::checkable).collect());
        let premise = self
            .premise
            .iter()
            .map(DisplayFormulaBlock::checkable)
            .collect();
        let assertion = self.assertion.checkable();

        Axiom::new(id, system_ref, local_directory, premise, assertion)
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

    // TODO: Remove.
    count: OnceCell<usize>,

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

            count: OnceCell::new(),

            href,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.system_ref.crosslink(document);

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
        self.assertion.crosslink(document, &self.vars);
    }

    pub fn vars(&self) -> &[VariableBlock<'a>] {
        &self.vars
    }

    // TODO: Remove.
    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap()
    }

    // TODO: Remove.
    pub fn index(&self) -> usize {
        *self.count.get().unwrap()
    }

    // TODO: Remove.
    pub fn checkable(&self) -> Theorem {
        let id = self.id.clone();
        let system_ref = SystemRef::new(self.system_ref.index());
        let local_directory =
            LocalCheckableDirectory::new(self.vars.iter().map(VariableBlock::checkable).collect());
        let premise = self
            .premise
            .iter()
            .map(DisplayFormulaBlock::checkable)
            .collect();
        let assertion = self.assertion.checkable();

        Theorem::new(id, system_ref, local_directory, premise, assertion)
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
}

pub enum ProofBlockSmallJustification<'a> {
    Deductable(DeductableBlockRef<'a>),
    Hypothesis(usize),

    Definition,
}

impl<'a> ProofBlockSmallJustification<'a> {
    // TODO: Remove.
    fn checkable(&self) -> ProofJustification {
        match self {
            Self::Deductable(deductable_ref) => match deductable_ref {
                DeductableBlockRef::Axiom(axiom_ref) => {
                    ProofJustification::Axiom(AxiomRef::new(axiom_ref.index()))
                }
                DeductableBlockRef::Theorem(theorem_ref) => {
                    ProofJustification::Theorem(TheoremRef::new(theorem_ref.index()))
                }
            },
            Self::Hypothesis(i) => ProofJustification::Hypothesis(HypothesisRef::new(*i - 1)),
            Self::Definition => ProofJustification::Definition,
        }
    }
}

impl<'a> ProofBlockSmallJustification<'a> {
    fn crosslink(&'a self, document: &'a Document<'a>) {
        if let Self::Deductable(deductable_ref) = self {
            deductable_ref.crosslink(document);
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

    // TODO: Remove.
    fn checkable(&self) -> ProofStep {
        let justification = self.justification.checkable();
        let formula = self.formula.checkable();

        ProofStep::new(justification, formula)
    }
}

pub struct ProofBlockStep<'a> {
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
        justification: ProofBlockJustification<'a>,
        small_steps: Vec<ProofBlockSmallStep<'a>>,
        formula: MathBlock,
        end: String,
        id: String,
        href: String,
        tag: usize,
    ) -> Self {
        ProofBlockStep {
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

    // TODO: Remove.
    fn checkable(&'a self) -> impl Iterator<Item = ProofStep> + '_ {
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

    // TODO: Remove.
    fn checkable(&'a self) -> Option<impl Iterator<Item = ProofStep> + '_> {
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

    // TODO: Remove.
    count: OnceCell<usize>,
}

impl<'a> ProofBlock<'a> {
    pub fn new(theorem_ref: TheoremBlockRef<'a>, elements: Vec<ProofBlockElement<'a>>) -> Self {
        ProofBlock {
            theorem_ref,

            elements,

            count: OnceCell::new(),
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.theorem_ref.crosslink(document);
        let vars = self.theorem_ref.vars();

        for element in &self.elements {
            element.crosslink(document, vars, self);
        }
    }

    // TODO: Remove.
    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap()
    }

    // TODO: Remove.
    pub fn checkable(&'a self) -> Proof {
        let theorem_ref = TheoremRef::new(self.theorem_ref.index());
        let steps = self
            .elements
            .iter()
            .filter_map(ProofBlockElement::checkable)
            .flatten()
            .collect();

        Proof::new(theorem_ref, steps)
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
}
