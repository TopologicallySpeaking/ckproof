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

use crate::deduction::directory::{CheckableDirectory, LocalCheckableDirectory};
use crate::deduction::{Axiom, Proof, ProofJustification, ProofStep, Theorem};

use crate::rendered::{
    AxiomRendered, ProofRendered, ProofRenderedElement, ProofRenderedJustification,
    ProofRenderedStep, TheoremRendered,
};

use super::directory::{AxiomBlockRef, BlockDirectory, SystemBlockRef, TheoremBlockRef};
use super::language::{FormulaBlock, VariableBlock};
use super::text::{Paragraph, Text};

pub struct AxiomBlock {
    id: String,
    name: String,
    system: SystemBlockRef,
    href: String,
    tagline: Paragraph,
    description: Vec<Text>,

    vars: Vec<VariableBlock>,
    premise: Vec<FormulaBlock>,
    assertion: FormulaBlock,
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
        premise: Vec<FormulaBlock>,
        assertion: FormulaBlock,
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

    pub fn checkable(&self, directory: &CheckableDirectory) -> Axiom {
        let id = self.id.clone();
        let system = self.system.into();

        let vars = self.vars.iter().map(VariableBlock::checkable).collect();
        let local_directory = LocalCheckableDirectory::new(vars);

        let premise = self
            .premise
            .iter()
            .map(|formula| formula.checkable(directory, &local_directory))
            .collect();
        let assertion = self.assertion.checkable(directory, &local_directory);

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
            .map(|formula| formula.render(directory, &self.vars))
            .collect();
        let assertion = self.assertion.render(directory, &self.vars);

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
    premise: Vec<FormulaBlock>,
    assertion: FormulaBlock,
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
        premise: Vec<FormulaBlock>,
        assertion: FormulaBlock,
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

    pub fn checkable(&self, directory: &CheckableDirectory) -> Theorem {
        let id = self.id.clone();
        let system = self.system.into();

        let vars = self.vars.iter().map(VariableBlock::checkable).collect();
        let local_directory = LocalCheckableDirectory::new(vars);

        let premise = self
            .premise
            .iter()
            .map(|formula| formula.checkable(directory, &local_directory))
            .collect();
        let assertion = self.assertion.checkable(directory, &local_directory);

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
            .map(|formula| formula.render(directory, &self.vars))
            .collect();
        let assertion = self.assertion.render(directory, &self.vars);

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
}

impl ProofBlockJustification {
    fn checkable(
        &self,
        formula: &FormulaBlock,
        directory: &CheckableDirectory,
        local_directory: &LocalCheckableDirectory,
    ) -> Vec<ProofStep> {
        match self {
            Self::Axiom(axiom_ref) => {
                let justification = ProofJustification::Axiom((*axiom_ref).into());
                let formula = formula.checkable(directory, local_directory);
                let step = ProofStep::new(justification, formula);

                vec![step]
            }

            Self::Theorem(theorem_ref) => {
                let justification = ProofJustification::Theorem((*theorem_ref).into());
                let formula = formula.checkable(directory, local_directory);
                let step = ProofStep::new(justification, formula);

                vec![step]
            }

            Self::Hypothesis(id) => {
                let justification = ProofJustification::Hypothesis(*id);
                let formula = formula.checkable(directory, local_directory);
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
        }
    }
}

#[derive(Debug)]
pub struct ProofBlockStep {
    id: String,
    href: String,
    justification: ProofBlockJustification,
    formula: FormulaBlock,
    end: String,
}

impl ProofBlockStep {
    pub fn new(
        id: String,
        href: String,
        justification: ProofBlockJustification,
        formula: FormulaBlock,
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

    fn checkable(
        &self,
        directory: &CheckableDirectory,
        local_directory: &LocalCheckableDirectory,
    ) -> Vec<ProofStep> {
        self.justification
            .checkable(&self.formula, directory, local_directory)
    }

    fn render(
        &self,
        directory: &BlockDirectory,
        vars: &[VariableBlock],
        tag: usize,
    ) -> ProofRenderedStep {
        let id = self.id.clone();
        let justification = self.justification.render(directory);
        let formula = self.formula.render(directory, vars);
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
        steps: &[ProofBlockStep],
        directory: &BlockDirectory,
        vars: &[VariableBlock],
        step_counter: &mut usize,
    ) -> ProofRenderedElement {
        match self {
            Self::Text(text) => ProofRenderedElement::Text(text.render(directory)),
            Self::Step => {
                let step = &steps[*step_counter];
                let ret =
                    ProofRenderedElement::Step(step.render(directory, vars, *step_counter + 1));

                *step_counter += 1;
                ret
            }
        }
    }
}

pub struct ProofBlock {
    theorem_ref: TheoremBlockRef,
    steps: Vec<ProofBlockStep>,
    elements: Vec<ProofBlockElement>,
}

impl ProofBlock {
    pub fn new(
        theorem_ref: TheoremBlockRef,
        steps: Vec<ProofBlockStep>,
        elements: Vec<ProofBlockElement>,
    ) -> ProofBlock {
        ProofBlock {
            theorem_ref,
            steps,
            elements,
        }
    }

    pub fn checkable(
        &self,
        block_directory: &BlockDirectory,
        checkable_directory: &CheckableDirectory,
    ) -> Proof {
        let theorem = &block_directory[self.theorem_ref];
        let vars = theorem.vars.iter().map(VariableBlock::checkable).collect();
        let local_directory = LocalCheckableDirectory::new(vars);

        let steps = self
            .steps
            .iter()
            .flat_map(|step| step.checkable(checkable_directory, &local_directory))
            .collect();

        Proof::new(self.theorem_ref.into(), steps)
    }

    pub fn render(&self, directory: &BlockDirectory) -> ProofRendered {
        let theorem = &directory[self.theorem_ref];
        let theorem_name = theorem.name.clone();
        let mut step_counter = 0;
        let elements = self
            .elements
            .iter()
            .map(|element| element.render(&self.steps, directory, &theorem.vars, &mut step_counter))
            .collect();

        ProofRendered::new(theorem_name, elements)
    }

    pub fn step(&self, step: usize) -> &ProofBlockStep {
        &self.steps[step]
    }
}
