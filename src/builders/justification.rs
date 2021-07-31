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

use pest::iterators::Pair;

use crate::document::structure::{AxiomBlockRef, DeductableBlockRef, TheoremBlockRef};
use crate::document::system::ProofBlockJustification;

use super::errors::{ParsingError, ParsingErrorContext, ProofParsingError, ProofStepParsingError};
use super::index::BuilderIndex;
use super::language::{FormulaBuilder, FormulaReadableApplicationBuilder, ReadableBuilder};
use super::system::{
    DeductableBuilder, ProofBuilder, ProofBuilderElement, ProofBuilderSmallJustification,
    ProofBuilderSmallStep, ProofBuilderStep, SystemBuilderChild, TheoremBuilder,
};
use super::Rule;

#[derive(Debug)]
pub struct SystemChildJustificationBuilder<'a> {
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
        let child = match index.search_system_child(&proof_ref.system_id(), &self.id) {
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

                if proof_ref.serial() < first_proof.serial() {
                    errors.err(ParsingError::ProofError(
                        proof_ref,
                        ProofParsingError::StepError(
                            step_ref,
                            ProofStepParsingError::TheoremJustificationUsedBeforeProof,
                        ),
                    ));

                    false
                } else if proof_ref.serial() == first_proof.serial() {
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

    fn build_small_steps(
        &'a self,
        formula: &FormulaBuilder<'a>,
    ) -> Option<Vec<ProofBuilderSmallStep>> {
        let justification = match *self.child.get().unwrap() {
            SystemBuilderChild::Axiom(axiom_ref) => {
                ProofBuilderSmallJustification::Deductable(DeductableBuilder::Axiom(axiom_ref))
            }
            SystemBuilderChild::Theorem(theorem_ref) => {
                ProofBuilderSmallJustification::Deductable(DeductableBuilder::Theorem(theorem_ref))
            }

            _ => unreachable!(),
        };

        Some(vec![ProofBuilderSmallStep::new(
            justification,
            formula.clone(),
        )])
    }

    fn finish<'b>(&self) -> ProofBlockJustification<'b> {
        match *self.child.get().unwrap() {
            SystemBuilderChild::Axiom(axiom) => {
                let axiom_location = axiom.location();
                let axiom_ref = AxiomBlockRef::new(axiom_location);

                ProofBlockJustification::Deductable(DeductableBlockRef::Axiom(axiom_ref))
            }
            SystemBuilderChild::Theorem(theorem) => {
                let theorem_location = theorem.location();
                let theorem_ref = TheoremBlockRef::new(theorem_location);

                ProofBlockJustification::Deductable(DeductableBlockRef::Theorem(theorem_ref))
            }

            _ => unreachable!(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug)]
pub enum MacroJustificationBuilder {
    Definition,
    FunctionApplication,
}

impl MacroJustificationBuilder {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::macro_justification);

        match pair.into_inner().next().unwrap().as_rule() {
            Rule::macro_justification_by_definition => Self::Definition,
            Rule::macro_justification_by_function_application => Self::FunctionApplication,

            _ => unreachable!(),
        }
    }

    fn build_function_application_iter<'a>(
        left: &'a FormulaBuilder<'a>,
        right: &'a FormulaBuilder<'a>,
        relation: ReadableBuilder<'a>,
        prev_steps: &'a [ProofBuilderElement<'a>],
    ) -> Box<dyn Iterator<Item = Option<ProofBuilderSmallStep<'a>>> + 'a> {
        let formula = FormulaBuilder::ReadableApplication(FormulaReadableApplicationBuilder::new(
            relation,
            vec![left.clone(), right.clone()],
        ));
        let formula_reversed = FormulaBuilder::ReadableApplication(
            FormulaReadableApplicationBuilder::new(relation, vec![right.clone(), left.clone()]),
        );

        // If this formula has already been derived.
        if prev_steps.iter().any(|step| step.eq_formula(&formula)) {
            Box::new(std::iter::empty())
        }
        // If this formula has been derived, but backwards. For example, we need a = b but we have
        // b = a, but we can switch the order by symmetry.
        else if relation.is_symmetric()
            && prev_steps
                .iter()
                .any(|step| step.eq_formula(&formula_reversed))
        {
            let deductable_ref = relation.get_symmetric().unwrap();
            let ret = Some(ProofBuilderSmallStep::new(
                ProofBuilderSmallJustification::Deductable(deductable_ref),
                formula,
            ));

            Box::new(std::iter::once(ret))
        }
        // If this formula can be derived by relfexivity.
        else if left == right {
            let deductable_ref = relation.get_reflexive().unwrap();
            let ret = Some(ProofBuilderSmallStep::new(
                ProofBuilderSmallJustification::Deductable(deductable_ref),
                formula,
            ));

            Box::new(std::iter::once(ret))
        }
        // Test if we can use function application over the relation.
        else if let (Some((left_function, left_inputs)), Some((right_function, right_inputs))) =
            (left.application(), right.application())
        {
            if left_inputs.len() != right_inputs.len() {
                todo!()
            }

            let input_steps =
                left_inputs
                    .zip(right_inputs)
                    .flat_map(move |(left_input, right_input)| {
                        Self::build_function_application_iter(
                            left_input,
                            right_input,
                            relation,
                            prev_steps,
                        )
                    });

            if left_function != right_function {
                todo!()
            }

            if let Some(deductable_ref) = left_function.get_function(relation) {
                Box::new(
                    input_steps.chain(std::iter::once(Some(ProofBuilderSmallStep::new(
                        ProofBuilderSmallJustification::Deductable(deductable_ref),
                        formula,
                    )))),
                )
            } else {
                todo!()
            }
        }
        // This statement cannot be derived by simple function application.
        else {
            todo!()
        }
    }

    fn build_function_application<'a>(
        formula: &'a FormulaBuilder<'a>,
        prev_steps: &'a [ProofBuilderElement<'a>],
    ) -> Option<Vec<ProofBuilderSmallStep<'a>>> {
        if let Some((relation, left, right)) = formula.binary() {
            if !relation.is_preorder() {
                todo!()
            }

            Self::build_function_application_iter(left, right, relation, prev_steps).collect()
        } else {
            todo!()
        }
    }

    fn build_small_steps<'a>(
        &'a self,
        formula: &'a FormulaBuilder<'a>,
        prev_steps: &'a [ProofBuilderElement<'a>],
    ) -> Option<Vec<ProofBuilderSmallStep>> {
        match self {
            Self::Definition => Some(vec![ProofBuilderSmallStep::new(
                ProofBuilderSmallJustification::Definition,
                formula.clone(),
            )]),

            Self::FunctionApplication => Self::build_function_application(formula, prev_steps),
        }
    }

    fn finish<'b>(&self) -> ProofBlockJustification<'b> {
        match self {
            Self::FunctionApplication => ProofBlockJustification::FunctionApplication,
            Self::Definition => ProofBlockJustification::Definition,
        }
    }
}

#[derive(Debug)]
pub enum ProofJustificationBuilder<'a> {
    SystemChild(SystemChildJustificationBuilder<'a>),
    Macro(MacroJustificationBuilder),
    // TODO: Create a HypothesisJustificationBuilder which references the hypothesis itself instead
    // of its index.
    Hypothesis(usize),
}

impl<'a> ProofJustificationBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::ident => Self::SystemChild(SystemChildJustificationBuilder::from_pest(pair)),
            Rule::macro_justification => Self::Macro(MacroJustificationBuilder::from_pest(pair)),

            _ => unreachable!(),
        }
    }

    pub fn hypothesis_from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::integer);

        ProofJustificationBuilder::Hypothesis(pair.as_str().parse().unwrap())
    }

    pub fn verify_structure(
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

    pub fn build_small_steps(
        &'a self,
        formula: &'a FormulaBuilder<'a>,
        prev_steps: &'a [ProofBuilderElement<'a>],
        errors: &mut ParsingErrorContext,
    ) -> Option<Vec<ProofBuilderSmallStep>> {
        match self {
            Self::SystemChild(justification) => justification.build_small_steps(formula),
            Self::Macro(justification) => justification.build_small_steps(formula, prev_steps),
            Self::Hypothesis(id) => Some(vec![ProofBuilderSmallStep::new(
                ProofBuilderSmallJustification::Hypothesis(*id),
                formula.clone(),
            )]),
        }
    }

    pub fn finish<'b>(&self) -> ProofBlockJustification<'b> {
        match self {
            Self::SystemChild(builder) => builder.finish(),
            Self::Macro(builder) => builder.finish(),
            Self::Hypothesis(id) => ProofBlockJustification::Hypothesis(*id),
        }
    }

    pub fn system_child(&self) -> Option<&SystemChildJustificationBuilder<'a>> {
        match self {
            Self::SystemChild(builder) => Some(builder),

            _ => None,
        }
    }
}
