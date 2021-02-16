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

use crate::deduction::directory::{
    AxiomRef, HypothesisRef, LocalCheckableDirectory, SystemRef, TheoremRef,
};
use crate::deduction::system::{Axiom, Proof, ProofJustification, ProofStep, Theorem};

use crate::rendered::{
    AxiomRendered, ProofRendered, ProofRenderedElement, ProofRenderedJustification,
    ProofRenderedStep, TheoremRendered,
};

use super::directory::{
    AxiomBlockRef, BlockDirectory, ProofBlockRef, SystemBlockRef, TheoremBlockRef,
};
use super::language::{DisplayFormulaBlock, VariableBlock};
use super::text::{Paragraph, Text};

pub struct AxiomBlock {
    id: String,
    name: String,
    system: SystemBlockRef,
    href: String,
    tagline: Paragraph,
    description: Vec<Text>,

    vars: Vec<VariableBlock>,
    premise: Vec<DisplayFormulaBlock>,
    assertion: DisplayFormulaBlock,
}

impl AxiomBlock {
    pub fn new(
        id: String,
        name: String,
        system: SystemBlockRef,
        href: String,
        tagline: Paragraph,
        description: Vec<Text>,
        vars: Vec<VariableBlock>,
        premise: Vec<DisplayFormulaBlock>,
        assertion: DisplayFormulaBlock,
    ) -> AxiomBlock {
        AxiomBlock {
            id,
            name,
            system,
            href,
            tagline,
            description,

            vars,
            premise,
            assertion,
        }
    }

    pub fn checkable(&self) -> Axiom {
        let id = self.id.clone();
        let system = SystemRef::new(self.system.get());

        let vars = self.vars.iter().map(VariableBlock::checkable).collect();
        let local_directory = LocalCheckableDirectory::new(vars);

        let premise = self
            .premise
            .iter()
            .map(DisplayFormulaBlock::checkable)
            .collect();
        let assertion = self.assertion.checkable();

        Axiom::new(id, system, local_directory, premise, assertion)
    }

    pub fn render(&self, directory: &BlockDirectory) -> AxiomRendered {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render(directory);
        let description = self
            .description
            .iter()
            .map(|text| text.render(directory))
            .collect();
        let premise = self
            .premise
            .iter()
            .map(|formula| formula.render())
            .collect();
        let assertion = self.assertion.render();

        let system = &directory[self.system];
        let system_id = system.id().to_owned();
        let system_name = system.name().to_owned();

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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn href(&self) -> &str {
        &self.href
    }
}

pub struct TheoremBlock {
    id: String,
    name: String,
    system: SystemBlockRef,
    href: String,
    tagline: Paragraph,
    description: Vec<Text>,

    vars: Vec<VariableBlock>,
    premise: Vec<DisplayFormulaBlock>,
    assertion: DisplayFormulaBlock,
}

impl TheoremBlock {
    pub fn new(
        id: String,
        name: String,
        system: SystemBlockRef,
        href: String,
        tagline: Paragraph,
        description: Vec<Text>,
        vars: Vec<VariableBlock>,
        premise: Vec<DisplayFormulaBlock>,
        assertion: DisplayFormulaBlock,
    ) -> TheoremBlock {
        TheoremBlock {
            id,
            name,
            system,
            href,
            tagline,
            description,

            vars,
            premise,
            assertion,
        }
    }

    pub fn checkable(&self) -> Theorem {
        let id = self.id.clone();
        let system = SystemRef::new(self.system.get());

        let vars = self.vars.iter().map(VariableBlock::checkable).collect();
        let local_directory = LocalCheckableDirectory::new(vars);

        let premise = self
            .premise
            .iter()
            .map(|formula| formula.checkable())
            .collect();
        let assertion = self.assertion.checkable();

        Theorem::new(id, system, local_directory, premise, assertion)
    }

    pub fn render(&self, directory: &BlockDirectory) -> TheoremRendered {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render(directory);
        let description = self
            .description
            .iter()
            .map(|text| text.render(directory))
            .collect();
        let premise = self
            .premise
            .iter()
            .map(|formula| formula.render())
            .collect();
        let assertion = self.assertion.render();

        let system = &directory[self.system];
        let system_id = system.id().to_owned();
        let system_name = system.name().to_owned();

        TheoremRendered::new(
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

    pub fn href(&self) -> &str {
        &self.href
    }
}

#[derive(Debug)]
pub enum ProofBlockJustification {
    Axiom(AxiomBlockRef),
    Theorem(TheoremBlockRef),
    Hypothesis(usize),

    Definition,
}

impl ProofBlockJustification {
    fn checkable(&self, formula: &DisplayFormulaBlock) -> Vec<ProofStep> {
        match self {
            Self::Axiom(axiom_ref) => {
                let justification = ProofJustification::Axiom(AxiomRef::new(axiom_ref.get()));
                let formula = formula.checkable();
                let step = ProofStep::new(justification, formula);

                vec![step]
            }

            Self::Theorem(theorem_ref) => {
                let justification = ProofJustification::Theorem(TheoremRef::new(theorem_ref.get()));
                let formula = formula.checkable();
                let step = ProofStep::new(justification, formula);

                vec![step]
            }

            Self::Hypothesis(id) => {
                let justification = ProofJustification::Hypothesis(HypothesisRef::new(*id - 1));
                let formula = formula.checkable();
                let step = ProofStep::new(justification, formula);

                vec![step]
            }

            Self::Definition => {
                let justification = ProofJustification::Definition;
                let formula = formula.checkable();
                let step = ProofStep::new(justification, formula);

                vec![step]
            }
        }
    }

    fn render(&self, directory: &BlockDirectory) -> ProofRenderedJustification {
        match self {
            Self::Axiom(axiom_ref) => {
                let axiom = &directory[*axiom_ref];
                let name = axiom.name.clone();
                let href = axiom.href.clone();

                ProofRenderedJustification::SystemChild(name, href)
            }

            Self::Theorem(theorem_ref) => {
                let theorem = &directory[*theorem_ref];
                let name = theorem.name.clone();
                let href = theorem.href.clone();

                ProofRenderedJustification::SystemChild(name, href)
            }

            Self::Hypothesis(id) => ProofRenderedJustification::Hypothesis(*id),

            Self::Definition => ProofRenderedJustification::Definition,
        }
    }
}

pub struct ProofBlockStep {
    id: String,
    href: String,
    justification: ProofBlockJustification,
    formula: DisplayFormulaBlock,
    end: String,
}

impl ProofBlockStep {
    pub fn new(
        id: String,
        href: String,
        justification: ProofBlockJustification,
        formula: DisplayFormulaBlock,
        end: String,
    ) -> ProofBlockStep {
        ProofBlockStep {
            id,
            href,
            justification,
            formula,
            end,
        }
    }

    fn checkable(&self) -> Vec<ProofStep> {
        self.justification.checkable(&self.formula)
    }

    fn render(&self, directory: &BlockDirectory, tag: usize) -> ProofRenderedStep {
        let id = self.id.clone();
        let justification = self.justification.render(directory);
        let formula = self.formula.render();
        let end = self.end.clone();

        ProofRenderedStep::new(id, justification, formula, end, tag)
    }

    pub fn href(&self) -> &str {
        &self.href
    }
}

pub enum ProofBlockElement {
    Text(Text),
    Step,
}

impl ProofBlockElement {
    fn render(
        &self,
        self_ref: ProofBlockRef,
        steps: &[ProofBlockStep],
        directory: &BlockDirectory,
        step_counter: &mut usize,
    ) -> ProofRenderedElement {
        match self {
            Self::Text(text) => {
                ProofRenderedElement::Text(text.render_with_proof_steps(directory, self_ref))
            }

            Self::Step => {
                let step = &steps[*step_counter];
                let ret = ProofRenderedElement::Step(step.render(directory, *step_counter + 1));

                *step_counter += 1;
                ret
            }
        }
    }
}

pub struct ProofBlock {
    self_ref: ProofBlockRef,
    theorem_ref: TheoremBlockRef,
    steps: Vec<ProofBlockStep>,
    elements: Vec<ProofBlockElement>,
}

impl ProofBlock {
    pub fn new(
        self_ref: ProofBlockRef,
        theorem_ref: TheoremBlockRef,
        steps: Vec<ProofBlockStep>,
        elements: Vec<ProofBlockElement>,
    ) -> ProofBlock {
        ProofBlock {
            self_ref,
            theorem_ref,
            steps,
            elements,
        }
    }

    pub fn checkable(&self) -> Proof {
        let steps = self
            .steps
            .iter()
            .flat_map(|step| step.checkable())
            .collect();

        Proof::new(TheoremRef::new(self.theorem_ref.get()), steps)
    }

    pub fn render(&self, directory: &BlockDirectory) -> ProofRendered {
        let theorem = &directory[self.theorem_ref];
        let theorem_name = theorem.name.clone();
        let mut step_counter = 0;
        let elements = self
            .elements
            .iter()
            .map(|element| element.render(self.self_ref, &self.steps, directory, &mut step_counter))
            .collect();

        ProofRendered::new(theorem_name, elements)
    }

    pub fn step(&self, step: usize) -> &ProofBlockStep {
        &self.steps[step]
    }
}
