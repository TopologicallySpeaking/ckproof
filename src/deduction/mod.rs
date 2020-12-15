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

use std::collections::HashMap;
use std::iter::FromIterator;

use crate::document::directory::{
    AxiomBlockRef, SymbolBlockRef, SystemBlockRef, TheoremBlockRef, TypeBlockRef, VariableBlockRef,
};

pub mod directory;
mod errors;

use directory::{CheckableDirectory, LocalCheckableDirectory};
use errors::CheckerError;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SystemRef(usize);

impl From<SystemBlockRef> for SystemRef {
    fn from(system_ref: SystemBlockRef) -> SystemRef {
        SystemRef(system_ref.get())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TypeRef(usize);

impl From<TypeBlockRef> for TypeRef {
    fn from(type_ref: TypeBlockRef) -> TypeRef {
        TypeRef(type_ref.get())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SymbolRef(usize);

impl From<SymbolBlockRef> for SymbolRef {
    fn from(symbol_ref: SymbolBlockRef) -> SymbolRef {
        SymbolRef(symbol_ref.get())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct VariableRef(usize);

impl From<VariableBlockRef> for VariableRef {
    fn from(variable_ref: VariableBlockRef) -> VariableRef {
        VariableRef(variable_ref.get())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AxiomRef(usize);

impl From<AxiomBlockRef> for AxiomRef {
    fn from(axiom_ref: AxiomBlockRef) -> AxiomRef {
        AxiomRef(axiom_ref.get())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TheoremRef(usize);

impl From<TheoremBlockRef> for TheoremRef {
    fn from(theorem_ref: TheoremBlockRef) -> TheoremRef {
        TheoremRef(theorem_ref.get())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ProofRef(usize);

pub struct System {
    id: String,
}

impl System {
    pub fn new(id: String) -> System {
        System { id }
    }
}

pub struct Type {
    id: String,

    system: SystemRef,
}

impl Type {
    pub fn new(id: String, system: SystemRef) -> Type {
        Type { id, system }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TypeSignature {
    inputs: Vec<TypeSignature>,
    output: TypeRef,
    variable: bool,
}

impl TypeSignature {
    pub fn new(inputs: Vec<TypeSignature>, output: TypeRef, variable: bool) -> TypeSignature {
        TypeSignature {
            inputs,
            output,
            variable,
        }
    }

    fn arity(&self) -> usize {
        self.inputs.len()
    }

    fn compatible(&self, other: &Self) -> bool {
        let inputs_compatible = if self.inputs.len() == other.inputs.len() {
            self.inputs
                .iter()
                .zip(&other.inputs)
                .all(|(self_input, other_input)| self_input.compatible(other_input))
        } else {
            false
        };
        let outputs_compatible = self.output == other.output;

        // It is unacceptable for `self` to require a variable and for other to not be a variable.
        // Anything else is allowed.
        let variables_compatible = !self.variable || other.variable;

        inputs_compatible && outputs_compatible && variables_compatible
    }

    fn applied(&self) -> TypeSignature {
        TypeSignature {
            inputs: vec![],
            output: self.output,
            variable: self.variable,
        }
    }
}

pub struct Symbol {
    id: String,
    system: SystemRef,
    type_signature: TypeSignature,
}

impl Symbol {
    pub fn new(id: String, system: SystemRef, type_signature: TypeSignature) -> Symbol {
        assert!(!type_signature.variable);

        Symbol {
            id,
            system,
            type_signature,
        }
    }
}

pub struct Variable {
    id: String,
    type_signature: TypeSignature,
}

impl Variable {
    pub fn new(id: String, type_signature: TypeSignature) -> Variable {
        assert!(type_signature.variable);

        Variable { id, type_signature }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum FormulaInner {
    Symbol(SymbolRef),
    Variable(VariableRef),

    SymbolApplication(SymbolRef, Vec<Formula>),
    VariableApplication(VariableRef, Vec<Formula>),
}

impl FormulaInner {
    fn get_substitution(&self, other: &Formula) -> Option<Substitution> {
        match self {
            Self::Symbol(self_ref) => {
                let other_ref = if let Self::Symbol(other_ref) = &other.inner {
                    other_ref
                } else {
                    return None;
                };

                if self_ref == other_ref {
                    Some(Substitution::empty())
                } else {
                    None
                }
            }

            Self::Variable(variable_ref) => Some(Substitution::new(*variable_ref, other.clone())),

            Self::SymbolApplication(self_ref, self_inputs) => {
                let (other_ref, other_inputs) =
                    if let Self::SymbolApplication(other_ref, other_inputs) = &other.inner {
                        (other_ref, other_inputs)
                    } else {
                        return None;
                    };

                if self_ref == other_ref {
                    // The `self_inputs` and `other_inputs` are guaranteed to be of the same
                    // length, because if they weren't, then the constructor which created them
                    // would have panicked.
                    let mut input_substitutions = self_inputs
                        .iter()
                        .zip(other_inputs)
                        .filter_map(|(s, o)| s.get_substitution(o));

                    input_substitutions.next().and_then(|first| {
                        input_substitutions.try_fold(first, |curr, next| curr.merge(&next))
                    })
                } else {
                    None
                }
            }

            Self::VariableApplication(_, _) => todo!(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Formula {
    inner: FormulaInner,
    type_signature: TypeSignature,
}

impl Formula {
    pub fn symbol(directory: &CheckableDirectory, symbol_ref: SymbolRef) -> Formula {
        let symbol = &directory[&symbol_ref];
        let type_signature = symbol.type_signature.clone();

        Formula {
            inner: FormulaInner::Symbol(symbol_ref),
            type_signature,
        }
    }

    pub fn variable(
        local_directory: &LocalCheckableDirectory,
        variable_ref: VariableRef,
    ) -> Formula {
        let variable = &local_directory[&variable_ref];
        let type_signature = variable.type_signature.clone();

        Formula {
            inner: FormulaInner::Variable(variable_ref),
            type_signature,
        }
    }

    pub fn symbol_application(
        directory: &CheckableDirectory,
        symbol_ref: SymbolRef,
        inputs: Vec<Formula>,
    ) -> Formula {
        let symbol = &directory[&symbol_ref];
        assert_eq!(symbol.type_signature.arity(), inputs.len());

        let input_signatures = inputs.iter().map(|formula| &formula.type_signature);
        for (symbol_input, input_signature) in
            symbol.type_signature.inputs.iter().zip(input_signatures)
        {
            assert!(symbol_input.compatible(input_signature));
        }

        let type_signature = symbol.type_signature.applied();

        Formula {
            inner: FormulaInner::SymbolApplication(symbol_ref, inputs),
            type_signature,
        }
    }

    pub fn variable_application(
        local_directory: &LocalCheckableDirectory,
        variable_ref: VariableRef,
        inputs: Vec<Formula>,
    ) -> Formula {
        let variable = &local_directory[&variable_ref];
        assert_eq!(variable.type_signature.arity(), inputs.len());

        let input_signatures = inputs.iter().map(|formula| &formula.type_signature);
        for (variable_input, input_signature) in
            variable.type_signature.inputs.iter().zip(input_signatures)
        {
            assert!(variable_input.compatible(input_signature));
        }

        let type_signature = variable.type_signature.applied();

        Formula {
            inner: FormulaInner::VariableApplication(variable_ref, inputs),
            type_signature,
        }
    }

    fn get_substitution(&self, other: &Self) -> Option<Substitution> {
        self.inner.get_substitution(other)
    }

    fn find_substitutions(&self, other: &[ProofStep]) -> SubstitutionList {
        other
            .iter()
            .filter_map(|step| self.get_substitution(&step.formula))
            .collect()
    }
}

pub struct Axiom {
    id: String,
    system: SystemRef,
    local_directory: LocalCheckableDirectory,

    premise: Vec<Formula>,
    assertion: Formula,
}

impl Axiom {
    pub fn new(
        id: String,
        system: SystemRef,
        local_directory: LocalCheckableDirectory,
        premise: Vec<Formula>,
        assertion: Formula,
    ) -> Axiom {
        Axiom {
            id,
            system,
            local_directory,

            premise,
            assertion,
        }
    }

    fn check(&self, formula: &Formula, prev_steps: &[ProofStep]) -> Option<CheckerError> {
        let required_substitution =
            if let Some(substitution) = self.assertion.get_substitution(formula) {
                substitution
            } else {
                todo!()
            };

        let sufficient_substitutions: SubstitutionList = self
            .premise
            .iter()
            .map(|premise_formula| premise_formula.find_substitutions(prev_steps))
            .collect();

        if sufficient_substitutions.contains(&required_substitution) {
            None
        } else {
            todo!()
        }
    }
}

pub struct Theorem {
    id: String,
    system: SystemRef,
    local_directory: LocalCheckableDirectory,

    premise: Vec<Formula>,
    assertion: Formula,
}

impl Theorem {
    pub fn new(
        id: String,
        system: SystemRef,
        local_directory: LocalCheckableDirectory,
        premise: Vec<Formula>,
        assertion: Formula,
    ) -> Theorem {
        Theorem {
            id,
            system,
            local_directory,

            premise,
            assertion,
        }
    }

    fn check(&self, formula: &Formula, prev_steps: &[ProofStep]) -> Option<CheckerError> {
        let required_substitution =
            if let Some(substitution) = self.assertion.get_substitution(formula) {
                substitution
            } else {
                todo!()
            };

        let sufficient_substitutions: SubstitutionList = self
            .premise
            .iter()
            .map(|premise_formula| premise_formula.find_substitutions(prev_steps))
            .collect();

        if sufficient_substitutions.contains(&required_substitution) {
            None
        } else {
            todo!()
        }
    }
}

pub enum ProofJustification {
    Axiom(AxiomRef),
    Theorem(TheoremRef),
    Hypothesis(usize),
}

impl ProofJustification {
    fn check(
        &self,
        formula: &Formula,
        premise: &[Formula],
        directory: &CheckableDirectory,
        prev_steps: &[ProofStep],
    ) -> Option<CheckerError> {
        match self {
            Self::Axiom(axiom_ref) => {
                let axiom = &directory[axiom_ref];

                axiom.check(formula, prev_steps)
            }

            Self::Theorem(theorem_ref) => {
                let theorem = &directory[theorem_ref];

                theorem.check(formula, prev_steps)
            }

            Self::Hypothesis(id) => {
                let hypothesis = &premise[*id - 1];

                if formula == hypothesis {
                    None
                } else {
                    todo!()
                }
            }
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

    fn check(
        &self,
        premise: &[Formula],
        directory: &CheckableDirectory,
        prev_steps: &[ProofStep],
    ) -> Option<CheckerError> {
        self.justification
            .check(&self.formula, premise, directory, prev_steps)
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

    pub fn check(&self, directory: &CheckableDirectory) -> Option<CheckerError> {
        let premise = &directory[&self.theorem_ref].premise;

        (0..self.steps.len())
            .filter_map(|i| {
                let prev_steps = &self.steps[0..i];

                self.steps[i].check(premise, directory, prev_steps)
            })
            .next()
            .or_else(|| {
                let goal = &directory[&self.theorem_ref].assertion;

                if &self.steps.last().unwrap().formula == goal {
                    None
                } else {
                    todo!()
                }
            })
    }
}

#[derive(Clone)]
struct Substitution {
    map: HashMap<VariableRef, Formula>,
}

impl Substitution {
    fn empty() -> Substitution {
        Substitution {
            map: HashMap::new(),
        }
    }

    fn new(variable_ref: VariableRef, formula: Formula) -> Substitution {
        let mut map = HashMap::new();
        map.insert(variable_ref, formula);
        Substitution { map }
    }

    fn merge(&self, other: &Self) -> Option<Substitution> {
        let mut map = self.map.clone();

        for (variable_ref, required_formula) in other.map.iter() {
            if let Some(target_formula) = map.get(variable_ref) {
                if target_formula != required_formula {
                    return None;
                }
            } else {
                map.insert(*variable_ref, required_formula.clone());
            }
        }

        Some(Substitution { map })
    }

    fn merge_list(self, list: &SubstitutionList) -> impl Iterator<Item = Substitution> + '_ {
        list.substitutions
            .iter()
            .filter_map(move |other| self.merge(other))
    }

    fn allows(&self, other: &Self) -> bool {
        self.map.iter().all(|(variable_ref, required_formula)| {
            if let Some(other_formula) = other.map.get(variable_ref) {
                other_formula == required_formula
            } else {
                true
            }
        })
    }
}

struct SubstitutionList {
    substitutions: Vec<Substitution>,
}

impl SubstitutionList {
    fn empty() -> SubstitutionList {
        SubstitutionList {
            substitutions: vec![Substitution::empty()],
        }
    }

    fn merge(&mut self, other: &SubstitutionList) {
        self.substitutions = self
            .substitutions
            .drain(..)
            .flat_map(|substitution| substitution.merge_list(other))
            .collect();
    }

    fn contains(&self, target_substitution: &Substitution) -> bool {
        self.substitutions
            .iter()
            .any(|sufficient_substitution| sufficient_substitution.allows(target_substitution))
    }
}

impl Extend<SubstitutionList> for SubstitutionList {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = SubstitutionList>,
    {
        for item in iter {
            self.merge(&item);
        }
    }
}

impl FromIterator<Substitution> for SubstitutionList {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Substitution>,
    {
        SubstitutionList {
            substitutions: iter.into_iter().collect(),
        }
    }
}

impl FromIterator<SubstitutionList> for SubstitutionList {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = SubstitutionList>,
    {
        let mut iter = iter.into_iter();

        match iter.next() {
            Some(mut first) => {
                first.extend(iter);
                first
            }

            None => Self::empty(),
        }
    }
}
