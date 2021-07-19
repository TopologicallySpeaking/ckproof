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

use super::directory::{
    AxiomRef, CheckableDirectory, HypothesisRef, LocalCheckableDirectory, ProofStepRef, SystemRef,
    TheoremRef,
};
use super::errors::{
    AxiomCheckingError, CheckingError, CheckingErrorContext, ProofCheckingError,
    ProofStepCheckingError, TheoremCheckingError,
};
use super::language::Formula;
use super::substitution::{Substitution, SubstitutionList};

pub struct System {
    id: String,
}

impl System {
    pub fn new(id: String) -> System {
        System { id }
    }
}

pub struct Axiom {
    id: String,
    system_ref: SystemRef,
    local_directory: LocalCheckableDirectory,
    premise: Vec<Formula>,
    assertion: Formula,
}

impl Axiom {
    pub fn new(
        id: String,
        system_ref: SystemRef,
        local_directory: LocalCheckableDirectory,
        premise: Vec<Formula>,
        assertion: Formula,
    ) -> Axiom {
        Axiom {
            id,
            system_ref,
            local_directory,
            premise,
            assertion,
        }
    }

    pub(super) fn verify<F>(
        &self,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(AxiomCheckingError) -> CheckingError,
    {
        if !directory.contains_system(self.system_ref) {
            errors.err(generate_error(AxiomCheckingError::InvalidSystemRef));
        }

        self.local_directory.verify(directory, errors, |e| {
            generate_error(AxiomCheckingError::LocalError(e))
        });

        for (i, hypothesis) in self.premise.iter().enumerate() {
            hypothesis.verify(&self.local_directory, directory, errors, |e| {
                generate_error(AxiomCheckingError::PremiseError(HypothesisRef(i), e))
            });
        }
        self.assertion
            .verify(&self.local_directory, directory, errors, |e| {
                generate_error(AxiomCheckingError::AssertionError(e))
            });
    }

    pub(super) fn assertion(&self) -> &Formula {
        &self.assertion
    }

    pub(super) fn premise(&self) -> &[Formula] {
        &self.premise
    }
}

pub struct Theorem {
    id: String,
    system_ref: SystemRef,
    local_directory: LocalCheckableDirectory,
    premise: Vec<Formula>,
    assertion: Formula,
}

impl Theorem {
    pub fn new(
        id: String,
        system_ref: SystemRef,
        local_directory: LocalCheckableDirectory,
        premise: Vec<Formula>,
        assertion: Formula,
    ) -> Theorem {
        Theorem {
            id,
            system_ref,
            local_directory,
            premise,
            assertion,
        }
    }

    pub(super) fn verify<F>(
        &self,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(TheoremCheckingError) -> CheckingError,
    {
        if !directory.contains_system(self.system_ref) {
            errors.err(generate_error(TheoremCheckingError::InvalidSystemRef));
        }

        self.local_directory.verify(directory, errors, |e| {
            generate_error(TheoremCheckingError::LocalError(e))
        });

        for (i, hypothesis) in self.premise.iter().enumerate() {
            hypothesis.verify(&self.local_directory, directory, errors, |e| {
                generate_error(TheoremCheckingError::PremiseError(HypothesisRef(i), e))
            });
        }
        self.assertion
            .verify(&self.local_directory, directory, errors, |e| {
                generate_error(TheoremCheckingError::AssertionError(e))
            });
    }

    pub(super) fn assertion(&self) -> &Formula {
        &self.assertion
    }

    pub(super) fn premise(&self) -> &[Formula] {
        &self.premise
    }
}

pub enum ProofJustification {
    Axiom(AxiomRef),
    Theorem(TheoremRef),
    Hypothesis(HypothesisRef),
    Definition,
}

impl ProofJustification {
    fn verify<F>(
        &self,
        num_hypotheses: usize,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ProofStepCheckingError) -> CheckingError,
    {
        match self {
            Self::Axiom(axiom_ref) => {
                if !directory.contains_axiom(*axiom_ref) {
                    errors.err(generate_error(ProofStepCheckingError::InvalidAxiomRef));
                }
            }

            Self::Theorem(theorem_ref) => {
                if !directory.contains_theorem(*theorem_ref) {
                    errors.err(generate_error(ProofStepCheckingError::InvalidTheoremRef));
                }
            }

            Self::Hypothesis(hypothesis_ref) => {
                if hypothesis_ref.0 >= num_hypotheses {
                    errors.err(generate_error(ProofStepCheckingError::InvalidHypothesisRef));
                }
            }

            Self::Definition => {}
        }
    }
}

pub struct ProofStep {
    justification: ProofJustification,
    formula: Formula,
}

impl ProofStep {
    pub fn new(justification: ProofJustification, formula: Formula) -> ProofStep {
        ProofStep {
            justification,
            formula,
        }
    }

    fn verify<F>(
        &self,
        num_hypotheses: usize,
        local_directory: &LocalCheckableDirectory,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ProofStepCheckingError) -> CheckingError,
    {
        self.justification
            .verify(num_hypotheses, directory, errors, |e| generate_error(e));

        self.formula
            .verify(local_directory, directory, errors, |e| {
                generate_error(ProofStepCheckingError::FormulaError(e))
            });
    }

    fn check<F>(
        &self,
        prev_steps: &[ProofStep],
        premise: &[Formula],
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ProofStepCheckingError) -> CheckingError,
    {
        match &self.justification {
            ProofJustification::Axiom(axiom_ref) => {
                let axiom = &directory[*axiom_ref];

                if let Some(assertion_substitution) =
                    Substitution::new(axiom.assertion(), &self.formula, directory)
                {
                    let premise_substitutions = axiom.premise().iter().map(|hypothesis| {
                        SubstitutionList::find(
                            hypothesis,
                            prev_steps.iter().map(|step| &step.formula),
                            directory,
                        )
                    });

                    let merged_substitutions = premise_substitutions.fold(
                        SubstitutionList::new(assertion_substitution),
                        |curr, next| curr.merge(next, directory),
                    );

                    if merged_substitutions.impossible() {
                        errors.err(generate_error(
                            ProofStepCheckingError::AxiomNotSubstitutable,
                        ));
                    }
                } else {
                    errors.err(generate_error(
                        ProofStepCheckingError::AxiomAssertionNotSubstitutable,
                    ));
                }
            }

            ProofJustification::Theorem(theorem_ref) => {
                let theorem = &directory[*theorem_ref];

                if let Some(assertion_substitution) =
                    Substitution::new(theorem.assertion(), &self.formula, directory)
                {
                    let premise_substitutions = theorem.premise().iter().map(|hypothesis| {
                        SubstitutionList::find(
                            hypothesis,
                            prev_steps.iter().map(|step| &step.formula),
                            directory,
                        )
                    });

                    let merged_substitutions = premise_substitutions.fold(
                        SubstitutionList::new(assertion_substitution),
                        |curr, next| curr.merge(next, directory),
                    );

                    if merged_substitutions.impossible() {
                        errors.err(generate_error(
                            ProofStepCheckingError::TheoremNotSubstitutable,
                        ));
                    }
                } else {
                    errors.err(generate_error(
                        ProofStepCheckingError::TheoremAssertionNotSubstitutable,
                    ));
                };
            }

            ProofJustification::Hypothesis(hypothesis_ref) => {
                if self.formula != premise[hypothesis_ref.0] {
                    errors.err(generate_error(ProofStepCheckingError::HypothesisMismatch))
                }
            }

            ProofJustification::Definition => {
                if !prev_steps.iter().any(|prev_step| {
                    Formula::compatible(&prev_step.formula, &self.formula, directory)
                }) {
                    errors.err(generate_error(ProofStepCheckingError::DefinitionMismatch))
                }
            }
        }
    }
}

pub struct Proof {
    theorem_ref: TheoremRef,
    steps: Vec<ProofStep>,
}

impl Proof {
    pub fn new(theorem_ref: TheoremRef, steps: Vec<ProofStep>) -> Proof {
        Proof { theorem_ref, steps }
    }

    pub(super) fn verify<F>(
        &self,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ProofCheckingError) -> CheckingError,
    {
        if !directory.contains_theorem(self.theorem_ref) {
            errors.err(generate_error(ProofCheckingError::InvalidTheoremRef));
            return;
        }

        let theorem = &directory[self.theorem_ref];
        let num_hypotheses = theorem.premise.len();
        let local_directory = &theorem.local_directory;

        for (i, step) in self.steps.iter().enumerate() {
            step.verify(num_hypotheses, local_directory, directory, errors, |e| {
                generate_error(ProofCheckingError::StepError(ProofStepRef(i), e))
            });
        }
    }

    pub(super) fn check<F>(
        &self,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ProofCheckingError) -> CheckingError,
    {
        let theorem = &directory[self.theorem_ref];

        if self.steps.is_empty() {
            errors.err(generate_error(ProofCheckingError::Empty));
            return;
        }

        for i in 0..self.steps.len() {
            let prev_steps = &self.steps[0..i];
            self.steps[i].check(prev_steps, &theorem.premise, directory, errors, |e| {
                generate_error(ProofCheckingError::StepError(ProofStepRef(i), e))
            });
        }

        if self.steps.last().unwrap().formula != theorem.assertion {
            errors.err(generate_error(ProofCheckingError::AssertionMismatch));
        }
    }
}
