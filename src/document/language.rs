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

use crate::map_ident;

use crate::deduction::directory::LocalCheckableDirectory;
use crate::deduction::{Definition, Formula, Symbol, System, Type, TypeSignature, Variable};

use crate::rendered::{
    DefinitionRendered, Denoted, DenotedStyle, SymbolRendered, SystemRendered, TypeRendered,
};

use super::directory::{
    BlockDirectory, DefinitionBlockRef, SymbolBlockRef, SystemBlockRef, TypeBlockRef,
    VariableBlockRef,
};
use super::text::{MathBlock, MathElement, Paragraph, Text};

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

pub enum TypeSignatureBlock {
    Ground(TypeBlockRef),
    Compound(Box<TypeSignatureBlock>, Box<TypeSignatureBlock>),
}

impl TypeSignatureBlock {
    pub fn checkable(&self) -> TypeSignature {
        match self {
            Self::Ground(type_ref) => TypeSignature::Ground((*type_ref).into()),
            Self::Compound(input, output) => {
                TypeSignature::Compound(Box::new(input.checkable()), Box::new(output.checkable()))
            }
        }
    }

    pub fn is_ground(&self) -> bool {
        match self {
            Self::Ground(_) => true,
            Self::Compound(_, _) => false,
        }
    }

    pub fn is_compound(&self) -> bool {
        match self {
            Self::Ground(_) => false,
            Self::Compound(_, _) => true,
        }
    }

    pub fn render(&self, directory: &BlockDirectory) -> String {
        // TODO: Render without so many parentheses.
        match self {
            Self::Ground(type_ref) => directory[*type_ref].id.to_owned(),

            Self::Compound(input, output) => {
                if input.is_compound() {
                    format!(
                        "({}) \u{2192} {}",
                        input.render(directory),
                        output.render(directory)
                    )
                } else {
                    format!(
                        "{} \u{2192} {}",
                        input.render(directory),
                        output.render(directory)
                    )
                }
            }
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

    fn example<'a, I>(&self, mut inputs: I) -> MathBlock
    where
        I: ExactSizeIterator<Item = &'a str>,
    {
        match self.style {
            DisplayStyle::Prefix => todo!(),
            DisplayStyle::Infix => {
                assert_eq!(inputs.len(), 2);
                let first = map_ident(inputs.next().unwrap()).to_owned();
                let second = map_ident(inputs.next().unwrap()).to_owned();

                MathBlock::new(vec![
                    MathElement::Variable(first.to_owned()),
                    MathElement::Operator(map_ident(&self.id).to_owned()),
                    MathElement::Variable(second.to_owned()),
                ])
            }

            _ => todo!(),
        }
    }

    fn render(&self) -> Option<Denoted> {
        match self.style {
            DisplayStyle::Prefix => Some(Denoted::new(
                DenotedStyle::Prefix,
                map_ident(&self.id).to_owned(),
            )),
            DisplayStyle::Infix => Some(Denoted::new(
                DenotedStyle::Infix,
                map_ident(&self.id).to_owned(),
            )),

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

pub struct DefinitionBlock {
    id: String,
    name: String,
    href: String,
    system: SystemBlockRef,
    tagline: Paragraph,
    description: Vec<Text>,

    type_signature: TypeSignatureBlock,
    inputs: Vec<VariableBlock>,
    display: Display,
    expanded: DisplayFormulaBlock,
}

impl DefinitionBlock {
    pub fn new(
        id: String,
        name: String,
        href: String,
        system: SystemBlockRef,
        tagline: Paragraph,
        description: Vec<Text>,
        type_signature: TypeSignatureBlock,
        inputs: Vec<VariableBlock>,
        display: Display,
        expanded: DisplayFormulaBlock,
    ) -> DefinitionBlock {
        DefinitionBlock {
            id,
            name,
            href,
            system,
            tagline,
            description,

            type_signature,
            inputs,
            display,
            expanded,
        }
    }

    pub fn checkable(&self) -> Definition {
        let id = self.id.clone();
        let system = self.system.into();

        let vars = self.inputs.iter().map(VariableBlock::checkable).collect();
        let local_directory = LocalCheckableDirectory::new(vars);

        let type_signature = self.type_signature.checkable();
        let expanded = self.expanded.checkable();

        Definition::new(id, system, local_directory, type_signature, expanded)
    }

    pub fn render(&self, directory: &BlockDirectory) -> DefinitionRendered {
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
        let expanded = self.expanded.render();
        let example = self
            .display
            .example(self.inputs.iter().map(|var| var.id.as_ref()))
            .render();

        let system = &directory[self.system];
        let system_id = system.id.clone();
        let system_name = system.name.clone();

        DefinitionRendered::new(
            id,
            system_id,
            name,
            system_name,
            tagline,
            description,
            denoted,
            type_signature,
            expanded,
            example,
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
    Definition(DefinitionBlockRef),

    SymbolApplication(SymbolBlockRef, Vec<FormulaBlock>),
    VariableApplication(VariableBlockRef, Vec<FormulaBlock>),
    DefinitionApplication(DefinitionBlockRef, Vec<FormulaBlock>),
}

impl FormulaBlock {
    fn checkable(&self) -> Formula {
        match self {
            Self::Symbol(symbol_ref) => Formula::Symbol((*symbol_ref).into()),
            Self::Variable(variable_ref) => Formula::Variable((*variable_ref).into()),
            Self::Definition(definition_ref) => Formula::Definition((*definition_ref).into()),

            Self::SymbolApplication(symbol_ref, inputs) => {
                let inputs = inputs.iter().map(|formula| formula.checkable()).collect();
                Formula::SymbolApplication((*symbol_ref).into(), inputs)
            }
            Self::VariableApplication(variable_ref, inputs) => {
                let inputs = inputs.iter().map(|formula| formula.checkable()).collect();
                Formula::VariableApplication((*variable_ref).into(), inputs)
            }
            Self::DefinitionApplication(definition_ref, inputs) => {
                let inputs = inputs.iter().map(|formula| formula.checkable()).collect();
                Formula::DefinitionApplication((*definition_ref).into(), inputs)
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

    pub fn checkable(&self) -> Formula {
        self.contents.checkable()
    }

    pub fn render(&self) -> String {
        self.display.render()
    }
}
