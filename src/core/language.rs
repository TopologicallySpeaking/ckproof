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

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::lazy::OnceCell;

use super::errors::CheckingError;
use super::substitution::{Substitution, SubstitutionList};
use super::system::{DeductableRef, ProofStep, System};

#[derive(Debug)]
pub struct Type<'a> {
    id: String,
    system_ref: OnceCell<&'a System>,
}

impl<'a> Type<'a> {
    pub fn new(id: String) -> Self {
        Type {
            id,
            system_ref: OnceCell::new(),
        }
    }

    pub fn set_system(&self, system: &'a System) {
        self.system_ref.set(system).unwrap();
    }
}

impl<'a> PartialEq for Type<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<'a> Eq for Type<'a> {}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TypeSignature<'a> {
    Ground(&'a Type<'a>),
    Compound(Box<TypeSignature<'a>>, Box<TypeSignature<'a>>),
}

impl<'a> TypeSignature<'a> {
    fn compound(&self) -> Option<(&TypeSignature<'a>, &TypeSignature<'a>)> {
        match self {
            Self::Compound(input, output) => Some((input, output)),

            _ => None,
        }
    }

    fn input(&self) -> Option<&TypeSignature<'a>> {
        self.compound().map(|(input, _)| input)
    }

    fn apply(self) -> Option<TypeSignature<'a>> {
        match self {
            Self::Compound(_, output) => Some(*output),

            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Symbol<'a> {
    id: String,
    system_ref: OnceCell<&'a System>,

    type_signature: OnceCell<TypeSignature<'a>>,
}

impl<'a> Symbol<'a> {
    pub fn new(id: String) -> Self {
        Symbol {
            id,
            system_ref: OnceCell::new(),

            type_signature: OnceCell::new(),
        }
    }

    pub fn set_system(&self, system: &'a System) {
        self.system_ref.set(system).unwrap();
    }

    pub fn set_type_signature(&self, type_signature: TypeSignature<'a>) {
        self.type_signature.set(type_signature).unwrap();
    }

    fn type_signature(&self) -> &TypeSignature<'a> {
        self.type_signature.get().unwrap()
    }
}

impl<'a> PartialEq for Symbol<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<'a> Eq for Symbol<'a> {}

#[derive(Debug)]
pub struct Definition<'a> {
    id: String,
    system_ref: OnceCell<&'a System>,

    inputs: OnceCell<Vec<&'a Variable<'a>>>,
    expanded: OnceCell<Formula<'a>>,
}

impl<'a> Definition<'a> {
    pub fn new(id: String) -> Self {
        Definition {
            id,
            system_ref: OnceCell::new(),

            inputs: OnceCell::new(),
            expanded: OnceCell::new(),
        }
    }

    pub fn set_system(&self, system_ref: &'a System) {
        self.system_ref.set(system_ref).unwrap();
    }

    pub fn set_inputs(&self, inputs: Vec<&'a Variable<'a>>) {
        self.inputs.set(inputs).unwrap();
    }

    pub fn set_expanded(&self, expanded: Formula<'a>) {
        self.expanded.set(expanded).unwrap();
    }

    fn type_signature(&self) -> TypeSignature<'a> {
        self.expanded.get().unwrap().type_signature()
    }

    pub fn verify(&self) -> bool {
        self.expanded.get().unwrap().verify()
    }

    fn inputs_match(&self, replacements: &[Formula<'a>]) -> bool {
        let inputs = self.inputs.get().unwrap();

        inputs.len() == replacements.len()
            && inputs
                .iter()
                .zip(replacements)
                .all(|(input, replacement)| input.type_signature() == &replacement.type_signature())
    }

    fn expand(&self, replacements: &[Formula<'a>]) -> Formula<'a> {
        let inputs = self.inputs.get().unwrap();
        let expanded = self.expanded.get().unwrap();

        let substitution = inputs.iter().copied().zip(replacements).collect();

        expanded.substitute(&substitution).expand_definitions()
    }
}

impl<'a> PartialEq for Definition<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<'a> Eq for Definition<'a> {}

#[derive(Debug)]
pub struct Variable<'a> {
    id: String,
    type_signature: OnceCell<TypeSignature<'a>>,
}

impl<'a> PartialEq for Variable<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<'a> Eq for Variable<'a> {}

impl<'a> Hash for Variable<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self as *const Variable).hash(state)
    }
}

impl<'a> Variable<'a> {
    pub fn new(id: String) -> Self {
        Variable {
            id,
            type_signature: OnceCell::new(),
        }
    }

    pub fn set_type_signature(&self, type_signature: TypeSignature<'a>) {
        self.type_signature.set(type_signature).unwrap()
    }

    fn type_signature(&self) -> &TypeSignature<'a> {
        self.type_signature.get().unwrap()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Formula<'a> {
    Symbol(&'a Symbol<'a>),
    Variable(&'a Variable<'a>),

    Application(Box<Formula<'a>>, Box<Formula<'a>>),

    Definition(&'a Definition<'a>, Vec<Formula<'a>>),
}

impl<'a> Formula<'a> {
    pub fn type_signature(&self) -> TypeSignature<'a> {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.type_signature().clone(),
            Self::Variable(variable_ref) => variable_ref.type_signature().clone(),

            Self::Application(function, _) => function.type_signature().apply().unwrap(),

            Self::Definition(definition_ref, _) => definition_ref.type_signature(),
        }
    }

    pub fn verify(&self) -> bool {
        match self {
            Self::Symbol(_) | Self::Variable(_) => true,

            Self::Application(function, input) => {
                let signatures_match = || {
                    if let Some(input_signature) = function.type_signature().input() {
                        input_signature == &input.type_signature()
                    } else {
                        false
                    }
                };

                function.verify() && input.verify() && signatures_match()
            }

            Self::Definition(definition_ref, inputs) => definition_ref.inputs_match(inputs),
        }
    }

    pub fn check_deductable(
        &'a self,
        deductable_ref: &DeductableRef<'a>,
        prev_steps: &'a [ProofStep<'a>],
        i: usize,
    ) -> Option<CheckingError> {
        if let Some(assertion_substitution) = Substitution::new(deductable_ref.assertion(), self) {
            let premise_substitutions = deductable_ref.premise().iter().map(|hypothesis| {
                SubstitutionList::find(hypothesis, prev_steps.iter().map(ProofStep::formula))
            });

            let merged_substitutions = premise_substitutions.fold(
                SubstitutionList::new(assertion_substitution),
                |curr, next| curr.merge(next),
            );

            if merged_substitutions.impossible() {
                Some(CheckingError::DeductableNotSubstitutable(i))
            } else {
                None
            }
        } else {
            Some(CheckingError::DeductableAssertionNotSubstitutable(i))
        }
    }

    pub fn symbol(&self) -> Option<&Symbol<'a>> {
        match self {
            Self::Symbol(symbol_ref) => Some(symbol_ref),

            _ => None,
        }
    }

    pub fn application(&self) -> Option<(&Formula<'a>, &Formula<'a>)> {
        match self {
            Self::Application(function, input) => Some((function, input)),

            _ => None,
        }
    }

    pub fn definition(&self) -> Option<(&Definition<'a>, &[Formula<'a>])> {
        match self {
            Self::Definition(definition_ref, inputs) => Some((definition_ref, inputs)),

            _ => None,
        }
    }

    pub fn expand_definitions(&self) -> Formula<'a> {
        match self {
            Self::Symbol(_) | Self::Variable(_) => self.clone(),

            Self::Application(function, input) => Self::Application(
                Box::new(function.expand_definitions()),
                Box::new(input.expand_definitions()),
            ),

            Self::Definition(definition_ref, inputs) => definition_ref.expand(inputs),
        }
    }

    fn substitute(&self, substitution: &HashMap<&Variable<'a>, &Formula<'a>>) -> Formula<'a> {
        match self {
            Self::Symbol(symbol_ref) => Formula::Symbol(*symbol_ref),
            Self::Variable(variable_ref) => substitution[*variable_ref].clone(),

            Self::Application(function, input) => Formula::Application(
                Box::new(function.substitute(substitution)),
                Box::new(input.substitute(substitution)),
            ),

            Self::Definition(definition_ref, inputs) => Formula::Definition(
                *definition_ref,
                inputs
                    .iter()
                    .map(|input| input.substitute(substitution))
                    .collect(),
            ),
        }
    }

    pub fn compatible(&'a self, other: &'a Self) -> bool {
        self.expand_definitions() == other.expand_definitions()
    }
}
