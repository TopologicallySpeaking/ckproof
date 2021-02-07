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

// NOTE: This entire thing has become too messy to salvage. This was intentional, as when I started
// I wasn't really sure what the expected behaviour was. That's why there's no documentation or
// tests as well. But, it's now time to rewrite it correctly.

use std::collections::HashMap;
use std::iter::FromIterator;
use std::ops::Index;

use crate::document::directory::{
    AxiomBlockRef, DefinitionBlockRef, SymbolBlockRef, SystemBlockRef, TheoremBlockRef,
    TypeBlockRef, VariableBlockRef,
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
pub struct DefinitionRef(usize);

impl From<DefinitionBlockRef> for DefinitionRef {
    fn from(definition_ref: DefinitionBlockRef) -> DefinitionRef {
        DefinitionRef(definition_ref.get())
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

pub struct Definition {
    id: String,
    system: SystemRef,
    local_directory: LocalCheckableDirectory,
    type_signature: TypeSignature,
    expanded: Formula,
}

impl Definition {
    pub fn new(
        id: String,
        system: SystemRef,
        local_directory: LocalCheckableDirectory,
        type_signature: TypeSignature,
        expanded: Formula,
    ) -> Definition {
        Definition {
            id,
            system,
            local_directory,
            type_signature,
            expanded,
        }
    }

    pub fn expand(
        &self,
        inputs: &[Formula],
        directory: &CheckableDirectory,
        local_directory: &LocalCheckableDirectory,
    ) -> Formula {
        let substitution = &self
            .local_directory
            .vars()
            .iter()
            .enumerate()
            .zip(inputs)
            .map(|((i, var), substitution)| {
                assert_eq!(
                    var.type_signature,
                    substitution.type_signature(directory, local_directory)
                );
                (VariableRef(i), substitution.clone())
            })
            .collect();

        self.expanded.substitute(substitution)
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

#[derive(Clone, Debug)]
pub enum Formula {
    Symbol(SymbolRef),
    Variable(VariableRef),
    Definition(DefinitionRef),

    SymbolApplication(SymbolRef, Vec<Formula>),
    VariableApplication(VariableRef, Vec<Formula>),
    DefinitionApplication(DefinitionRef, Vec<Formula>),
}

impl Formula {
    fn substitute(&self, substitution: &Substitution) -> Formula {
        match self {
            Self::Symbol(_) => todo!(),
            Self::Variable(variable_ref) => substitution[variable_ref].clone(),
            Self::Definition(_) => todo!(),

            Self::SymbolApplication(symbol_ref, inputs) => Self::SymbolApplication(
                *symbol_ref,
                inputs
                    .iter()
                    .map(|input| input.substitute(substitution))
                    .collect(),
            ),
            Self::VariableApplication(_, _) => todo!(),
            Self::DefinitionApplication(_, _) => todo!(),
        }
    }

    fn get_substitution(
        &self,
        other: &Formula,
        directory: &CheckableDirectory,
    ) -> Option<Substitution> {
        match self {
            Self::Symbol(self_ref) => {
                let other_ref = if let Self::Symbol(ref other_ref) = other {
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

            Self::Definition(_) => todo!(),

            Self::SymbolApplication(self_ref, self_inputs) => {
                let (other_ref, other_inputs) =
                    if let Self::SymbolApplication(other_ref, ref other_inputs) = other {
                        (other_ref, other_inputs)
                    } else {
                        return None;
                    };

                if self_ref == other_ref {
                    assert_eq!(self_inputs.len(), other_inputs.len());
                    let mut input_substitutions = self_inputs
                        .iter()
                        .zip(other_inputs)
                        .filter_map(|(s, o)| s.get_substitution(o, directory));

                    input_substitutions.next().and_then(|first| {
                        input_substitutions
                            .try_fold(first, |curr, next| curr.merge(&next, directory))
                    })
                } else {
                    None
                }
            }

            Self::VariableApplication(_, _) => todo!(),

            Self::DefinitionApplication(self_ref, self_inputs) => match other {
                Self::DefinitionApplication(other_ref, other_inputs) => {
                    if self_ref == other_ref {
                        assert_eq!(self_inputs.len(), other_inputs.len());
                        let mut input_substitutions = self_inputs
                            .iter()
                            .zip(other_inputs)
                            .filter_map(|(s, o)| s.get_substitution(o, directory));

                        input_substitutions.next().and_then(|first| {
                            input_substitutions
                                .try_fold(first, |curr, next| curr.merge(&next, directory))
                        })
                    } else {
                        None
                    }
                }

                _ => todo!(),
            },
        }
    }

    fn find_substitutions(
        &self,
        other: &[ProofStep],
        directory: &CheckableDirectory,
    ) -> SubstitutionList {
        other
            .iter()
            .filter_map(|step| self.get_substitution(&step.formula, directory))
            .collect()
    }

    fn type_signature(
        &self,
        directory: &CheckableDirectory,
        local_directory: &LocalCheckableDirectory,
    ) -> TypeSignature {
        match self {
            Self::Symbol(_) => todo!(),
            Self::Variable(variable_ref) => local_directory[variable_ref].type_signature.clone(),
            Self::Definition(_) => todo!(),

            Self::SymbolApplication(_, _) => todo!(),
            Self::VariableApplication(_, _) => todo!(),
            Self::DefinitionApplication(_, _) => todo!(),
        }
    }

    fn matches_symbol(&self, other_symbol_ref: SymbolRef, directory: &CheckableDirectory) -> bool {
        todo!()
    }

    fn matches_variable(
        &self,
        other_variable_ref: VariableRef,
        directory: &CheckableDirectory,
    ) -> bool {
        match self {
            Self::Variable(variable_ref) => *variable_ref == other_variable_ref,

            Self::Definition(_) => todo!(),
            Self::DefinitionApplication(definition_ref, inputs) => {
                let definition = &directory[definition_ref];
                let local_directory = &definition.local_directory;

                definition
                    .expand(inputs, directory, local_directory)
                    .matches_variable(other_variable_ref, directory)
            }

            _ => false,
        }
    }

    fn matches_definition(
        &self,
        other_definition_ref: DefinitionRef,
        directory: &CheckableDirectory,
    ) -> bool {
        todo!()
    }

    fn matches_symbol_application(
        &self,
        other_symbol_ref: SymbolRef,
        other_inputs: &[Formula],
        directory: &CheckableDirectory,
    ) -> bool {
        match self {
            Self::SymbolApplication(symbol_ref, inputs) => {
                *symbol_ref == other_symbol_ref
                    && inputs.len() == other_inputs.len()
                    && inputs
                        .iter()
                        .zip(other_inputs)
                        .all(|(formula, other_formula)| formula.matches(other_formula, directory))
            }

            Self::Definition(_) => todo!(),
            Self::DefinitionApplication(definition_ref, inputs) => {
                let definition = &directory[definition_ref];
                let local_directory = &definition.local_directory;

                definition
                    .expand(inputs, directory, local_directory)
                    .matches_symbol_application(other_symbol_ref, other_inputs, directory)
            }

            _ => false,
        }
    }

    fn matches_variable_application(
        &self,
        other_variable_ref: VariableRef,
        other_inputs: &[Formula],
        directory: &CheckableDirectory,
    ) -> bool {
        todo!()
    }

    fn matches_definition_application(
        &self,
        other_definition_ref: DefinitionRef,
        other_inputs: &[Formula],
        directory: &CheckableDirectory,
    ) -> bool {
        match self {
            Self::DefinitionApplication(definition_ref, inputs) => {
                *definition_ref == other_definition_ref
                    && inputs.len() == other_inputs.len()
                    && inputs
                        .iter()
                        .zip(other_inputs)
                        .all(|(formula, other_formula)| formula.matches(other_formula, directory))
            }

            _ => {
                let other_definition = &directory[&other_definition_ref];
                other_definition.expanded.matches(self, directory)
            }
        }
    }

    fn matches(&self, other: &Self, directory: &CheckableDirectory) -> bool {
        match self {
            Self::Symbol(symbol_ref) => other.matches_symbol(*symbol_ref, directory),
            Self::Variable(variable_ref) => other.matches_variable(*variable_ref, directory),
            Self::Definition(definition_ref) => {
                other.matches_definition(*definition_ref, directory)
            }

            Self::SymbolApplication(symbol_ref, inputs) => {
                other.matches_symbol_application(*symbol_ref, inputs, directory)
            }
            Self::VariableApplication(variable_ref, inputs) => {
                other.matches_variable_application(*variable_ref, inputs, directory)
            }
            Self::DefinitionApplication(definition_ref, inputs) => {
                other.matches_definition_application(*definition_ref, inputs, directory)
            }
        }
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

    fn check(
        &self,
        formula: &Formula,
        prev_steps: &[ProofStep],
        directory: &CheckableDirectory,
    ) -> Option<CheckerError> {
        let required_substitution =
            if let Some(substitution) = self.assertion.get_substitution(formula, directory) {
                substitution
            } else {
                todo!()
            };

        let sufficient_substitutions: SubstitutionList = self
            .premise
            .iter()
            .map(|premise_formula| premise_formula.find_substitutions(prev_steps, directory))
            .fold(SubstitutionList::empty(), |curr, next| {
                curr.merge(next, directory)
            });

        if sufficient_substitutions.contains(&required_substitution, directory) {
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

    fn check(
        &self,
        formula: &Formula,
        prev_steps: &[ProofStep],
        directory: &CheckableDirectory,
    ) -> Option<CheckerError> {
        let required_substitution =
            if let Some(substitution) = self.assertion.get_substitution(formula, directory) {
                substitution
            } else {
                todo!()
            };

        let sufficient_substitutions = self
            .premise
            .iter()
            .map(|premise_formula| premise_formula.find_substitutions(prev_steps, directory))
            .fold(SubstitutionList::empty(), |curr, next| {
                curr.merge(next, directory)
            });

        if sufficient_substitutions.contains(&required_substitution, directory) {
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

    Definition,
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

                axiom.check(formula, prev_steps, directory)
            }

            Self::Theorem(theorem_ref) => {
                let theorem = &directory[theorem_ref];

                theorem.check(formula, prev_steps, directory)
            }

            Self::Hypothesis(id) => {
                let hypothesis = &premise[*id - 1];

                if formula.matches(hypothesis, directory) {
                    None
                } else {
                    todo!()
                }
            }

            Self::Definition => {
                if prev_steps
                    .iter()
                    .any(|prev_step| prev_step.formula.matches(formula, directory))
                {
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

                if self.steps.last().unwrap().formula.matches(goal, directory) {
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

    fn merge(&self, other: &Self, directory: &CheckableDirectory) -> Option<Substitution> {
        let mut map = self.map.clone();

        for (variable_ref, required_formula) in other.map.iter() {
            if let Some(target_formula) = map.get(variable_ref) {
                if !target_formula.matches(required_formula, directory) {
                    return None;
                }
            } else {
                map.insert(*variable_ref, required_formula.clone());
            }
        }

        Some(Substitution { map })
    }

    fn merge_list<'a>(
        self,
        list: &'a SubstitutionList,
        directory: &'a CheckableDirectory,
    ) -> impl Iterator<Item = Substitution> + 'a {
        list.substitutions
            .iter()
            .filter_map(move |other| self.merge(other, directory))
    }

    fn allows(&self, other: &Self, directory: &CheckableDirectory) -> bool {
        self.map.iter().all(|(variable_ref, required_formula)| {
            if let Some(other_formula) = other.map.get(variable_ref) {
                other_formula.matches(required_formula, directory)
            } else {
                true
            }
        })
    }
}

impl Index<&VariableRef> for Substitution {
    type Output = Formula;

    fn index(&self, index: &VariableRef) -> &Self::Output {
        &self.map[index]
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

    fn merge(self, other: SubstitutionList, directory: &CheckableDirectory) -> SubstitutionList {
        SubstitutionList {
            substitutions: self
                .substitutions
                .into_iter()
                .flat_map(|substitution| substitution.merge_list(&other, directory))
                .collect(),
        }
    }

    fn contains(&self, target_substitution: &Substitution, directory: &CheckableDirectory) -> bool {
        self.substitutions.iter().any(|sufficient_substitution| {
            sufficient_substitution.allows(target_substitution, directory)
        })
    }
}

impl FromIterator<(VariableRef, Formula)> for Substitution {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (VariableRef, Formula)>,
    {
        Substitution {
            map: iter.into_iter().collect(),
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
