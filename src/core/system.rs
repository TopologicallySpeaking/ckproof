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

use super::errors::CheckingError;
use super::language::Formula;

#[derive(Debug)]
pub struct System {
    id: String,
}

impl System {
    pub fn new(id: String) -> Self {
        System { id }
    }
}

#[derive(Debug)]
pub struct Axiom<'a> {
    id: String,
    system_ref: OnceCell<&'a System>,

    premise: OnceCell<Vec<Formula<'a>>>,
    assertion: OnceCell<Formula<'a>>,
}

impl<'a> Axiom<'a> {
    pub fn new(id: String) -> Self {
        Axiom {
            id,
            system_ref: OnceCell::new(),

            premise: OnceCell::new(),
            assertion: OnceCell::new(),
        }
    }

    pub fn set_system(&self, system_ref: &'a System) {
        self.system_ref.set(system_ref).unwrap()
    }

    pub fn set_premise(&self, premise: Vec<Formula<'a>>) {
        self.premise.set(premise).unwrap()
    }

    pub fn set_assertion(&self, assertion: Formula<'a>) {
        self.assertion.set(assertion).unwrap()
    }

    fn premise(&self) -> &[Formula<'a>] {
        self.premise.get().unwrap()
    }

    fn assertion(&self) -> &Formula<'a> {
        self.assertion.get().unwrap()
    }

    pub fn verify(&self) -> bool {
        let premise = self.premise.get().unwrap();
        let assertion = self.assertion.get().unwrap();

        premise.iter().all(Formula::verify) && assertion.verify()
    }
}

#[derive(Debug)]
pub struct Theorem<'a> {
    id: String,
    system_ref: OnceCell<&'a System>,

    premise: OnceCell<Vec<Formula<'a>>>,
    assertion: OnceCell<Formula<'a>>,
}

impl<'a> Theorem<'a> {
    pub fn new(id: String) -> Self {
        Theorem {
            id,
            system_ref: OnceCell::new(),

            premise: OnceCell::new(),
            assertion: OnceCell::new(),
        }
    }

    pub fn set_system(&self, system_ref: &'a System) {
        self.system_ref.set(system_ref).unwrap()
    }

    pub fn set_premise(&self, premise: Vec<Formula<'a>>) {
        self.premise.set(premise).unwrap()
    }

    pub fn set_assertion(&self, assertion: Formula<'a>) {
        self.assertion.set(assertion).unwrap()
    }

    fn premise(&self) -> &[Formula<'a>] {
        self.premise.get().unwrap()
    }

    fn assertion(&self) -> &Formula<'a> {
        self.assertion.get().unwrap()
    }

    pub fn verify(&self) -> bool {
        let premise = self.premise.get().unwrap();
        let assertion = self.assertion.get().unwrap();

        premise.iter().all(Formula::verify) && assertion.verify()
    }
}

#[derive(Debug)]
pub enum DeductableRef<'a> {
    Axiom(&'a Axiom<'a>),
    Theorem(&'a Theorem<'a>),
}

impl<'a> DeductableRef<'a> {
    pub fn premise(&self) -> &[Formula<'a>] {
        match self {
            Self::Axiom(axiom_ref) => axiom_ref.premise(),
            Self::Theorem(theorem_ref) => theorem_ref.premise(),
        }
    }

    pub fn assertion(&self) -> &Formula<'a> {
        match self {
            Self::Axiom(axiom_ref) => axiom_ref.assertion(),
            Self::Theorem(theorem_ref) => theorem_ref.assertion(),
        }
    }
}

#[derive(Debug)]
pub enum ProofJustification<'a> {
    Deductable(DeductableRef<'a>),
    Hypothesis(usize),

    Definition,
}

#[derive(Debug)]
pub struct ProofStep<'a> {
    justification: ProofJustification<'a>,
    formula: Formula<'a>,
}

impl<'a> ProofStep<'a> {
    pub fn new(justification: ProofJustification<'a>, formula: Formula<'a>) -> Self {
        ProofStep {
            justification,
            formula,
        }
    }

    fn verify(&self) -> bool {
        self.formula.verify()
    }

    fn check(
        &'a self,
        prev_steps: &'a [ProofStep<'a>],
        premise: &[Formula<'a>],
        i: usize,
    ) -> Option<CheckingError> {
        match &self.justification {
            ProofJustification::Deductable(deductable_ref) => {
                self.formula.check_deductable(deductable_ref, prev_steps, i)
            }

            ProofJustification::Hypothesis(hypothesis_index) => {
                if self.formula == premise[*hypothesis_index] {
                    None
                } else {
                    Some(CheckingError::HypothesisMismatch(i))
                }
            }

            ProofJustification::Definition => {
                if prev_steps
                    .iter()
                    .any(|prev_step| prev_step.formula().compatible(self.formula()))
                {
                    None
                } else {
                    Some(CheckingError::DefinitionMismatch(i))
                }
            }
        }
    }

    pub fn formula(&self) -> &Formula<'a> {
        &self.formula
    }
}

#[derive(Debug)]
pub struct Proof<'a> {
    theorem_ref: &'a Theorem<'a>,
    steps: Vec<ProofStep<'a>>,
}

impl<'a> Proof<'a> {
    pub fn new(theorem_ref: &'a Theorem<'a>, steps: Vec<ProofStep<'a>>) -> Self {
        Proof { theorem_ref, steps }
    }

    pub fn verify(&self) -> bool {
        self.steps.iter().all(ProofStep::verify)
    }

    pub fn check(&'a self) -> Box<dyn Iterator<Item = CheckingError> + '_> {
        if self.steps.is_empty() {
            return Box::new(std::iter::once(CheckingError::EmptyProof));
        }

        let premise = self.theorem_ref.premise();
        let step_errors = (0..self.steps.len()).filter_map(move |i| {
            let prev_steps = &self.steps[0..i];

            self.steps[i].check(prev_steps, premise, i)
        });

        Box::new(step_errors.chain(
            if &self.steps.last().unwrap().formula == self.theorem_ref.assertion() {
                None
            } else {
                Some(CheckingError::AssertionMismatch)
            },
        ))
    }
}
