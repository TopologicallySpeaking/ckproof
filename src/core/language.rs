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
    CheckableDirectory, DefinitionRef, LocalCheckableDirectory, SymbolRef, SystemRef, TypeRef,
    VariableRef,
};
use super::errors::{
    CheckingError, CheckingErrorContext, DefinitionCheckingError, FormulaCheckingError,
    SymbolCheckingError, TypeCheckingError, TypeSignatureCheckingError, VariableCheckingError,
};
use super::substitution::Substitution;

pub struct Type {
    id: String,
    system_ref: SystemRef,
}

impl Type {
    pub fn new(id: String, system_ref: SystemRef) -> Type {
        Type { id, system_ref }
    }

    pub(super) fn verify<F>(
        &self,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(TypeCheckingError) -> CheckingError,
    {
        if !directory.contains_system(self.system_ref) {
            errors.err(generate_error(TypeCheckingError::InvalidSystemRef));
        }
    }
}

pub enum TypeSignature {
    Ground(TypeRef),
    Compound(Box<TypeSignature>, Box<TypeSignature>),
}

impl TypeSignature {
    pub(super) fn verify<F>(
        &self,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(TypeSignatureCheckingError) -> CheckingError + Copy,
    {
        match self {
            Self::Ground(type_ref) => {
                if !directory.contains_type(*type_ref) {
                    errors.err(generate_error(TypeSignatureCheckingError::InvalidTypeRef(
                        *type_ref,
                    )))
                }
            }

            Self::Compound(input, output) => {
                input.verify(directory, errors, generate_error);
                output.verify(directory, errors, generate_error);
            }
        }
    }

    pub fn arity(&self) -> usize {
        match self {
            Self::Ground(_) => 0,
            Self::Compound(_, input) => input.arity() + 1,
        }
    }
}

pub struct Symbol {
    id: String,
    system_ref: SystemRef,
    type_signature: TypeSignature,
}

impl Symbol {
    pub fn new(id: String, system_ref: SystemRef, type_signature: TypeSignature) -> Symbol {
        Symbol {
            id,
            system_ref,
            type_signature,
        }
    }

    pub(super) fn verify<F>(
        &self,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(SymbolCheckingError) -> CheckingError,
    {
        if !directory.contains_system(self.system_ref) {
            errors.err(generate_error(SymbolCheckingError::InvalidSystemRef));
        }

        self.type_signature.verify(directory, errors, |e| {
            generate_error(SymbolCheckingError::TypeSignatureError(e))
        });
    }
}

pub struct Definition {
    id: String,
    system_ref: SystemRef,
    local_directory: LocalCheckableDirectory,
    inputs: Vec<VariableRef>,
    type_signature: TypeSignature,
    expanded: Formula,
}

impl Definition {
    pub fn new(
        id: String,
        system_ref: SystemRef,
        local_directory: LocalCheckableDirectory,
        inputs: Vec<VariableRef>,
        type_signature: TypeSignature,
        expanded: Formula,
    ) -> Definition {
        Definition {
            id,
            system_ref,
            local_directory,
            inputs,
            type_signature,
            expanded,
        }
    }

    pub(super) fn verify<F>(
        &self,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(DefinitionCheckingError) -> CheckingError,
    {
        if !directory.contains_system(self.system_ref) {
            errors.err(generate_error(DefinitionCheckingError::InvalidSystemRef));
        }

        self.local_directory.verify(directory, errors, |e| {
            generate_error(DefinitionCheckingError::LocalError(e))
        });
        self.type_signature.verify(directory, errors, |e| {
            generate_error(DefinitionCheckingError::TypeSignatureError(e))
        });
        self.expanded
            .verify(&self.local_directory, directory, errors, |e| {
                generate_error(DefinitionCheckingError::ExpandedError(e))
            });
    }

    pub(super) fn expand(&self, inputs: &[Formula], directory: &CheckableDirectory) -> Formula {
        assert_eq!(self.inputs.len(), inputs.len());

        let substitution = self.inputs.iter().copied().zip(inputs).collect();
        self.expanded
            .substitute(&substitution)
            .expand_definitions(directory)
    }

    pub(super) fn arity(&self) -> usize {
        self.type_signature.arity()
    }
}

pub struct Variable {
    id: String,
    type_signature: TypeSignature,
}

impl Variable {
    pub fn new(id: String, type_signature: TypeSignature) -> Variable {
        Variable { id, type_signature }
    }

    pub(super) fn verify<F>(
        &self,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(VariableCheckingError) -> CheckingError,
    {
        self.type_signature.verify(directory, errors, |e| {
            generate_error(VariableCheckingError::TypeSignatureError(e))
        });
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Formula {
    Symbol(SymbolRef),
    Variable(VariableRef),

    Application(Box<Formula>, Box<Formula>),

    Definition(DefinitionRef, Vec<Formula>),
}

impl Formula {
    pub(super) fn verify<F>(
        &self,
        local_directory: &LocalCheckableDirectory,
        directory: &CheckableDirectory,
        errors: &mut CheckingErrorContext,
        generate_error: F,
    ) where
        F: Fn(FormulaCheckingError) -> CheckingError + Copy,
    {
        match self {
            Self::Symbol(symbol_ref) => {
                if !directory.contains_symbol(*symbol_ref) {
                    errors.err(generate_error(FormulaCheckingError::InvalidSymbolRef(
                        *symbol_ref,
                    )));
                }
            }
            Self::Variable(variable_ref) => {
                if !local_directory.contains_variable(*variable_ref) {
                    errors.err(generate_error(FormulaCheckingError::InvalidVariableRef(
                        *variable_ref,
                    )));
                }
            }

            Self::Application(function, input) => {
                function.verify(local_directory, directory, errors, generate_error);
                input.verify(local_directory, directory, errors, generate_error);
            }

            Self::Definition(definition_ref, inputs) => {
                if !directory.contains_definition(*definition_ref) {
                    errors.err(generate_error(FormulaCheckingError::InvalidDefinitionRef(
                        *definition_ref,
                    )));
                }

                let definition = &directory[*definition_ref];

                if inputs.len() != definition.arity() {
                    errors.err(generate_error(FormulaCheckingError::DefinitionWrongArity(
                        *definition_ref,
                    )));
                }

                for input in inputs {
                    input.verify(local_directory, directory, errors, generate_error);
                }
            }
        }
    }

    fn expand_definitions(&self, directory: &CheckableDirectory) -> Formula {
        match self {
            Self::Symbol(symbol_ref) => Self::Symbol(*symbol_ref),
            Self::Variable(variable_ref) => Self::Variable(*variable_ref),

            Self::Application(function, input) => Self::Application(
                Box::new(function.expand_definitions(directory)),
                Box::new(input.expand_definitions(directory)),
            ),

            Self::Definition(definition_ref, inputs) => {
                directory[*definition_ref].expand(inputs, directory)
            }
        }
    }

    pub(super) fn substitute(&self, substitution: &Substitution) -> Formula {
        match self {
            Self::Symbol(symbol_ref) => Self::Symbol(*symbol_ref),
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

    pub(super) fn compatible(left: &Self, right: &Self, directory: &CheckableDirectory) -> bool {
        left.expand_definitions(directory) == right.expand_definitions(directory)
    }

    pub(super) fn symbol(&self) -> Option<SymbolRef> {
        match self {
            Self::Symbol(symbol_ref) => Some(*symbol_ref),
            _ => None,
        }
    }

    pub(super) fn application(&self) -> Option<(&Formula, &Formula)> {
        match self {
            Self::Application(function, input) => Some((function, input)),
            _ => None,
        }
    }

    pub(super) fn definition(&self) -> Option<(DefinitionRef, &[Formula])> {
        match self {
            Self::Definition(definition_ref, inputs) => Some((*definition_ref, inputs)),
            _ => None,
        }
    }
}
