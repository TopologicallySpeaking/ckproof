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

enum FunctionApplicationStackItem<'a> {
    Pair(&'a FormulaBuilder<'a>, &'a FormulaBuilder<'a>),
    Prepared(ProofBuilderSmallStep<'a>),
}

struct FunctionApplicationIter<'a> {
    stack: Vec<FunctionApplicationStackItem<'a>>,
    relation: ReadableBuilder<'a>,
    prev_steps: &'a [ProofBuilderElement<'a>],
}

impl<'a> FunctionApplicationIter<'a> {
    fn new(
        left: &'a FormulaBuilder<'a>,
        right: &'a FormulaBuilder<'a>,
        relation: ReadableBuilder<'a>,
        prev_steps: &'a [ProofBuilderElement<'a>],
    ) -> Self {
        FunctionApplicationIter {
            stack: vec![FunctionApplicationStackItem::Pair(left, right)],
            relation,
            prev_steps,
        }
    }

    fn formula_already_derived(&self, formula: &FormulaBuilder<'a>) -> bool {
        self.prev_steps.iter().any(|step| step.eq_formula(formula))
    }

    fn by_reflexivity(&self, formula: FormulaBuilder<'a>) -> ProofBuilderSmallStep<'a> {
        let reflexive_deductable = self.relation.get_reflexive().unwrap();

        ProofBuilderSmallStep::new(
            ProofBuilderSmallJustification::Deductable(reflexive_deductable),
            formula,
        )
    }

    fn by_symmetry(&self, formula: FormulaBuilder<'a>) -> ProofBuilderSmallStep<'a> {
        let symmetry_deductable = self.relation.get_symmetric().unwrap();

        ProofBuilderSmallStep::new(
            ProofBuilderSmallJustification::Deductable(symmetry_deductable),
            formula,
        )
    }
}

impl<'a> Iterator for FunctionApplicationIter<'a> {
    // TODO: The error of this should contain information about the error.
    type Item = Result<ProofBuilderSmallStep<'a>, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.stack.pop() {
            let (left, right) = match item {
                FunctionApplicationStackItem::Pair(left, right) => (left, right),
                FunctionApplicationStackItem::Prepared(ret) => return Some(Ok(ret)),
            };

            // We need to create a small step to justify this formula.
            let target_formula =
                FormulaBuilder::ReadableApplication(FormulaReadableApplicationBuilder::new(
                    self.relation,
                    vec![left.clone(), right.clone()],
                ));

            // If this was derived in a previous step, then there is no work to do. Move on to the
            // next item in the stack.
            if self.formula_already_derived(&target_formula) {
                continue;
            }

            // If this formula can be derived by reflexivity.
            if left == right {
                return Some(Ok(self.by_reflexivity(target_formula)));
            }

            // If this was derived in a previous step, but backwards, and the relation is
            // symmetric, then we can get what we need by applying that symmetry.
            if self.relation.is_symmetric() {
                let reversed_formula =
                    FormulaBuilder::ReadableApplication(FormulaReadableApplicationBuilder::new(
                        self.relation,
                        vec![right.clone(), left.clone()],
                    ));

                if self.formula_already_derived(&reversed_formula) {
                    return Some(Ok(self.by_symmetry(target_formula)));
                }
            }

            // If all else fails, attempt to derive it by function application.
            if let (Some((left_function, left_inputs)), Some((right_function, right_inputs))) =
                (left.application(), right.application())
            {
                if left_function != right_function {
                    return Some(Err(()));
                }

                if left_inputs.len() != right_inputs.len() {
                    return Some(Err(()));
                }

                let function_deductable = match left_function.get_function(self.relation) {
                    Some(deductable) => deductable,
                    None => return Some(Err(())),
                };

                // We're good to go. Push the work to do on the stack, and move on to the next.
                let target_step = ProofBuilderSmallStep::new(
                    ProofBuilderSmallJustification::Deductable(function_deductable),
                    target_formula,
                );
                self.stack
                    .push(FunctionApplicationStackItem::Prepared(target_step));

                let input_steps = left_inputs
                    .zip(right_inputs)
                    .map(|(left, right)| FunctionApplicationStackItem::Pair(left, right));
                self.stack.extend(input_steps.rev());

                continue;
            }

            // If we've reached here, then every possible method has failed.
            return Some(Err(()));
        }

        None
    }
}

#[derive(Debug)]
pub enum MacroJustificationBuilder {
    Definition,
    FunctionApplication,
    Substitution,
}

impl MacroJustificationBuilder {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::macro_justification);

        match pair.into_inner().next().unwrap().as_rule() {
            Rule::macro_justification_by_definition => Self::Definition,
            Rule::macro_justification_by_function_application => Self::FunctionApplication,
            Rule::macro_justification_by_substitution => Self::Substitution,

            _ => unreachable!(),
        }
    }

    // TODO: This should return a Result.
    fn build_function_application<'a>(
        formula: &'a FormulaBuilder<'a>,
        prev_steps: &'a [ProofBuilderElement<'a>],
    ) -> Option<Vec<ProofBuilderSmallStep<'a>>> {
        if let Some((relation, left, right)) = formula.binary() {
            if !relation.is_preorder() {
                todo!()
            }

            let result: Result<_, _> =
                FunctionApplicationIter::new(left, right, relation, prev_steps).collect();
            result.ok()
        } else {
            todo!()
        }
    }

    fn try_build_substitution<'a>(
        step: &'a ProofBuilderStep<'a>,
        relation: ReadableBuilder<'a>,
        left: &'a FormulaBuilder<'a>,
        right: &'a FormulaBuilder<'a>,
        prev_steps: &'a [ProofBuilderElement<'a>],
    ) -> Option<Vec<ProofBuilderSmallStep<'a>>> {
        let (step_relation, step_left, step_right) = step.formula().binary()?;

        if step_relation != relation {
            return None;
        }

        let transitive_deductable = relation.get_transitive().unwrap();

        let left_steps = FunctionApplicationIter::new(left, step_left, relation, prev_steps);
        let right_steps = FunctionApplicationIter::new(step_right, right, relation, prev_steps);

        let join_left = std::iter::once_with(|| {
            let formula =
                FormulaBuilder::ReadableApplication(FormulaReadableApplicationBuilder::new(
                    relation,
                    vec![left.clone(), step_right.clone()],
                ));

            Ok(ProofBuilderSmallStep::new(
                ProofBuilderSmallJustification::Deductable(transitive_deductable),
                formula,
            ))
        });
        let join_right = std::iter::once_with(|| {
            let formula = FormulaBuilder::ReadableApplication(
                FormulaReadableApplicationBuilder::new(relation, vec![left.clone(), right.clone()]),
            );

            Ok(ProofBuilderSmallStep::new(
                ProofBuilderSmallJustification::Deductable(transitive_deductable),
                formula,
            ))
        });

        let result: Result<_, _> = left_steps
            .chain(right_steps)
            .chain(join_left)
            .chain(join_right)
            .collect();
        result.ok()
    }

    fn build_substitution<'a>(
        formula: &'a FormulaBuilder<'a>,
        prev_steps: &'a [ProofBuilderElement<'a>],
    ) -> Option<Vec<ProofBuilderSmallStep<'a>>> {
        if let Some((relation, left, right)) = formula.binary() {
            if !relation.is_preorder() {
                todo!()
            }

            prev_steps
                .iter()
                .filter_map(ProofBuilderElement::step)
                .find_map(|step| {
                    Self::try_build_substitution(step, relation, left, right, prev_steps)
                })
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
            Self::Substitution => Self::build_substitution(formula, prev_steps),
        }
    }

    fn finish<'b>(&self) -> ProofBlockJustification<'b> {
        match self {
            Self::FunctionApplication => ProofBlockJustification::FunctionApplication,
            Self::Definition => ProofBlockJustification::Definition,
            Self::Substitution => ProofBlockJustification::Substitution,
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
