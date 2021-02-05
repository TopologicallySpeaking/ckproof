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

use itertools::Itertools;

use crate::map_ident;

use crate::deduction::directory::{CheckableDirectory, LocalCheckableDirectory};
use crate::deduction::{Formula, Symbol, System, Type, TypeSignature, Variable};

use crate::rendered::{Denoted, DenotedStyle, SymbolRendered, SystemRendered, TypeRendered};

use super::directory::{
    BlockDirectory, SymbolBlockRef, SystemBlockRef, TypeBlockRef, VariableBlockRef,
};
use super::text::{MathBlock, Paragraph, Text};

pub struct SystemBlock {
    id: String,
    name: String,
    href: String,
    tagline: Paragraph,
    description: Vec<Text>,
}

impl SystemBlock {
    pub fn new(
        id: String,
        name: String,
        href: String,
        tagline: Paragraph,
        description: Vec<Text>,
    ) -> SystemBlock {
        SystemBlock {
            id,
            name,
            href,
            tagline,
            description,
        }
    }

    pub fn checkable(&self) -> System {
        System::new(self.id.clone())
    }

    pub fn render(&self, directory: &BlockDirectory) -> SystemRendered {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render(directory);
        let description = self
            .description
            .iter()
            .map(|text| text.render(directory))
            .collect();

        SystemRendered::new(id, name, tagline, description)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn href(&self) -> &str {
        &self.href
    }
}

pub struct TypeBlock {
    id: String,
    name: String,
    href: String,
    system: SystemBlockRef,
    tagline: Paragraph,
    description: Vec<Text>,
}

impl TypeBlock {
    pub fn new(
        id: String,
        name: String,
        href: String,
        system: SystemBlockRef,
        tagline: Paragraph,
        description: Vec<Text>,
    ) -> TypeBlock {
        TypeBlock {
            id,
            name,
            href,
            system,
            tagline,
            description,
        }
    }

    pub fn checkable(&self) -> Type {
        Type::new(self.id.clone(), self.system.into())
    }

    pub fn render(&self, directory: &BlockDirectory) -> TypeRendered {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render(directory);
        let description = self
            .description
            .iter()
            .map(|text| text.render(directory))
            .collect();

        let system = &directory[self.system];
        let system_id = system.id.clone();
        let system_name = system.name.clone();

        TypeRendered::new(id, system_id, name, system_name, tagline, description)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn href(&self) -> &str {
        &self.href
    }
}

pub struct TypeSignatureBlock {
    inputs: Vec<TypeSignatureBlock>,
    output: TypeBlockRef,
    variable: bool,
}

impl TypeSignatureBlock {
    pub fn new(
        inputs: Vec<TypeSignatureBlock>,
        output: TypeBlockRef,
        variable: bool,
    ) -> TypeSignatureBlock {
        TypeSignatureBlock {
            inputs,
            output,
            variable,
        }
    }

    fn checkable(&self) -> TypeSignature {
        let inputs = self
            .inputs
            .iter()
            .map(TypeSignatureBlock::checkable)
            .collect();
        let output = self.output.into();
        let variable = self.variable;

        TypeSignature::new(inputs, output, variable)
    }

    fn render(&self, directory: &BlockDirectory) -> String {
        let output_id = directory[self.output].id.clone();

        if self.inputs.is_empty() {
            output_id.to_owned()
        } else {
            let inputs: String = self
                .inputs
                .iter()
                .map(|input| input.render(directory))
                .intersperse(", ".to_owned())
                .collect();

            format!("({}) \u{2192} {}", inputs, output_id)
        }
    }
}

#[derive(Clone)]
pub enum DisplayStyle {
    Prefix,
    Infix,
    Suffix,
    Standard,
}

#[derive(Clone)]
pub struct Display {
    style: DisplayStyle,
    id: String,
}

impl Display {
    pub fn new(style: DisplayStyle, id: String) -> Display {
        Display { style, id }
    }

    fn render(&self) -> Option<Denoted> {
        match self.style {
            DisplayStyle::Prefix => Some(Denoted::new(DenotedStyle::Prefix, map_ident(&self.id))),
            DisplayStyle::Infix => Some(Denoted::new(DenotedStyle::Infix, map_ident(&self.id))),

            _ => todo!(),
        }
    }
}

pub struct SymbolBlock {
    id: String,
    name: String,
    href: String,
    system: SystemBlockRef,
    tagline: Paragraph,
    description: Vec<Text>,

    type_signature: TypeSignatureBlock,
    display: Display,
}

impl SymbolBlock {
    pub fn new(
        id: String,
        name: String,
        href: String,
        system: SystemBlockRef,
        tagline: Paragraph,
        description: Vec<Text>,

        type_signature: TypeSignatureBlock,
        display: Display,
    ) -> SymbolBlock {
        SymbolBlock {
            id,
            name,
            href,
            system,
            tagline,
            description,

            type_signature,
            display,
        }
    }

    pub fn checkable(&self) -> Symbol {
        let id = self.id.clone();
        let system = self.system.into();
        let type_signature = self.type_signature.checkable();

        Symbol::new(id, system, type_signature)
    }

    pub fn render(&self, directory: &BlockDirectory) -> SymbolRendered {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render(directory);
        let description = self
            .description
            .iter()
            .map(|text| text.render(directory))
            .collect();
        let denoted = self.display.render();
        let type_signature = self.type_signature.render(directory);

        let system = &directory[self.system];
        let system_id = system.id.clone();
        let system_name = system.name.clone();

        SymbolRendered::new(
            id,
            system_id,
            name,
            system_name,
            tagline,
            description,
            denoted,
            type_signature,
        )
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn href(&self) -> &str {
        &self.href
    }
}

pub struct VariableBlock {
    id: String,
    type_signature: TypeSignatureBlock,
}

impl VariableBlock {
    pub fn new(id: String, type_signature: TypeSignatureBlock) -> VariableBlock {
        VariableBlock { id, type_signature }
    }

    pub fn checkable(&self) -> Variable {
        let id = self.id.clone();
        let type_signature = self.type_signature.checkable();

        Variable::new(id, type_signature)
    }
}

#[derive(Debug)]
pub enum FormulaBlock {
    Symbol(SymbolBlockRef),
    Variable(VariableBlockRef),

    SymbolApplication(SymbolBlockRef, Vec<FormulaBlock>),
    VariableApplication(VariableBlockRef, Vec<FormulaBlock>),
}

impl FormulaBlock {
    fn checkable(
        &self,
        directory: &CheckableDirectory,
        local_directory: &LocalCheckableDirectory,
    ) -> Formula {
        match self {
            Self::Symbol(symbol_ref) => Formula::symbol(directory, (*symbol_ref).into()),
            Self::Variable(variable_ref) => {
                Formula::variable(local_directory, (*variable_ref).into())
            }

            Self::SymbolApplication(symbol_ref, inputs) => {
                let inputs = inputs
                    .iter()
                    .map(|formula| formula.checkable(directory, local_directory))
                    .collect();
                Formula::symbol_application(directory, (*symbol_ref).into(), inputs)
            }
            Self::VariableApplication(variable_ref, inputs) => {
                let inputs = inputs
                    .iter()
                    .map(|formula| formula.checkable(directory, local_directory))
                    .collect();
                Formula::variable_application(local_directory, (*variable_ref).into(), inputs)
            }
        }
    }
}

pub struct DisplayFormulaBlock {
    display: MathBlock,
    contents: FormulaBlock,
}

impl DisplayFormulaBlock {
    pub fn new(display: MathBlock, contents: FormulaBlock) -> DisplayFormulaBlock {
        DisplayFormulaBlock { display, contents }
    }

    pub fn checkable(
        &self,
        directory: &CheckableDirectory,
        local_directory: &LocalCheckableDirectory,
    ) -> Formula {
        self.contents.checkable(directory, local_directory)
    }

    pub fn render(&self) -> String {
        self.display.render()
    }
}
