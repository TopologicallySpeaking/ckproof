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

use crate::map_ident;
use crate::rendered::{DefinitionRendered, Denoted, DenotedStyle, SymbolRendered, TypeRendered};

use crate::core::language::{Definition, Formula, Symbol, Type, TypeSignature, Variable};

use super::structure::{DefinitionBlockRef, SymbolBlockRef, SystemBlockRef, TypeBlockRef};
use super::text::{MathBlock, MathElement, Paragraph, Text};
use super::Document;

pub struct TypeBlock<'a> {
    id: String,
    name: String,

    system_ref: SystemBlockRef<'a>,

    tagline: Paragraph<'a>,
    description: Vec<Text<'a>>,

    checkable: Type<'a>,
}

impl<'a> TypeBlock<'a> {
    pub fn new(
        id: String,
        name: String,
        system_ref: SystemBlockRef<'a>,
        tagline: Paragraph<'a>,
        description: Vec<Text<'a>>,
    ) -> Self {
        let checkable = Type::new(id.clone());

        TypeBlock {
            id,
            name,

            system_ref,

            tagline,
            description,

            checkable,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.system_ref.crosslink(document);
        self.checkable.set_system(self.system_ref.checkable());

        self.tagline.crosslink(document);
        for text in &self.description {
            text.crosslink(document);
        }
    }

    pub fn checkable(&'a self) -> &Type {
        &self.checkable
    }

    // TODO: Remove.
    pub fn render(&self) -> TypeRendered {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render();
        let description = self.description.iter().map(Text::render).collect();

        let system_id = self.system_ref.id().to_owned();
        let system_name = self.system_ref.name().to_owned();

        TypeRendered::new(id, system_id, name, system_name, tagline, description)
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

impl<'a> std::fmt::Debug for TypeBlock<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub enum TypeSignatureBlock<'a> {
    Ground(TypeBlockRef<'a>),
    Compound(Box<TypeSignatureBlock<'a>>, Box<TypeSignatureBlock<'a>>),
}

impl<'a> TypeSignatureBlock<'a> {
    fn crosslink(&'a self, document: &'a Document<'a>) {
        match self {
            Self::Ground(type_ref) => type_ref.crosslink(document),
            Self::Compound(left, right) => {
                left.crosslink(document);
                right.crosslink(document);
            }
        }
    }

    fn is_compound(&self) -> bool {
        match self {
            Self::Ground(_) => false,
            Self::Compound(_, _) => true,
        }
    }

    fn checkable(&'a self) -> TypeSignature {
        match self {
            Self::Ground(type_ref) => TypeSignature::Ground(type_ref.checkable()),
            Self::Compound(input, output) => {
                TypeSignature::Compound(Box::new(input.checkable()), Box::new(output.checkable()))
            }
        }
    }

    // TODO: Remove.
    pub fn render(&self) -> String {
        // TODO: Render without so many parentheses.
        match self {
            Self::Ground(type_ref) => type_ref.id().to_owned(),

            Self::Compound(input, output) => {
                if input.is_compound() {
                    format!("({}) \u{2192} {}", input.render(), output.render())
                } else {
                    format!("{} \u{2192} {}", input.render(), output.render())
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

pub struct SymbolBlock<'a> {
    id: String,
    name: String,

    system_ref: SystemBlockRef<'a>,

    tagline: Paragraph<'a>,
    description: Vec<Text<'a>>,

    type_signature: TypeSignatureBlock<'a>,
    display: Display,

    checkable: Symbol<'a>,

    // TODO: Remove.
    href: String,
}

impl<'a> SymbolBlock<'a> {
    pub fn new(
        id: String,
        name: String,
        system_ref: SystemBlockRef<'a>,
        tagline: Paragraph<'a>,
        description: Vec<Text<'a>>,
        type_signature: TypeSignatureBlock<'a>,
        display: Display,
        href: String,
    ) -> Self {
        let checkable = Symbol::new(id.clone());

        SymbolBlock {
            id,
            name,

            system_ref,

            tagline,
            description,

            type_signature,
            display,

            checkable,

            href,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.system_ref.crosslink(document);
        self.checkable.set_system(self.system_ref.checkable());

        self.tagline.crosslink(document);
        for text in &self.description {
            text.crosslink(document);
        }

        self.type_signature.crosslink(document);
        self.checkable
            .set_type_signature(self.type_signature.checkable());
    }

    pub fn checkable(&'a self) -> &Symbol {
        &self.checkable
    }

    // TODO: Remove.
    pub fn render(&self) -> SymbolRendered {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render();
        let description = self.description.iter().map(Text::render).collect();
        let denoted = self.display.render();
        let type_signature = self.type_signature.render();

        let system_id = self.system_ref.id().to_owned();
        let system_name = self.system_ref.name().to_owned();

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

    pub fn href(&self) -> &str {
        &self.href
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<'a> std::fmt::Debug for SymbolBlock<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub struct DefinitionBlock<'a> {
    id: String,
    name: String,

    system_ref: SystemBlockRef<'a>,

    tagline: Paragraph<'a>,
    description: Vec<Text<'a>>,

    display: Display,
    inputs: Vec<VariableBlock<'a>>,
    type_signature: TypeSignatureBlock<'a>,
    expanded: DisplayFormulaBlock<'a>,

    checkable: Definition<'a>,

    // TODO: Remove.
    href: String,
}

impl<'a> DefinitionBlock<'a> {
    pub fn new(
        id: String,
        name: String,
        system_ref: SystemBlockRef<'a>,
        tagline: Paragraph<'a>,
        description: Vec<Text<'a>>,
        display: Display,
        inputs: Vec<VariableBlock<'a>>,
        type_signature: TypeSignatureBlock<'a>,
        expanded: DisplayFormulaBlock<'a>,
        href: String,
    ) -> Self {
        let checkable = Definition::new(id.clone());

        DefinitionBlock {
            id,
            name,

            system_ref,

            tagline,
            description,

            type_signature,
            display,
            inputs,
            expanded,

            checkable,

            href,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.system_ref.crosslink(document);
        self.checkable.set_system(self.system_ref.checkable());

        self.tagline.crosslink(document);
        for text in &self.description {
            text.crosslink(document);
        }

        for input in &self.inputs {
            input.crosslink(document);
        }
        self.checkable
            .set_inputs(self.inputs.iter().map(VariableBlock::checkable).collect());

        self.type_signature.crosslink(document);

        self.expanded.crosslink(document, &self.inputs);
        self.checkable.set_expanded(self.expanded.checkable());
    }

    pub fn verify(&self) {
        assert!(self.checkable.verify());
    }

    pub fn checkable(&'a self) -> &Definition {
        &self.checkable
    }

    // TODO: Remove.
    pub fn render(&self) -> DefinitionRendered {
        let id = self.id.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render();
        let description = self.description.iter().map(Text::render).collect();
        let denoted = self.display.render();
        let type_signature = self.type_signature.render();
        let expanded = self.expanded.render();
        let example = self
            .display
            .example(self.inputs.iter().map(|var| var.id.as_ref()))
            .render();

        let system_id = self.system_ref.id().to_owned();
        let system_name = self.system_ref.name().to_owned();

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

    pub fn href(&self) -> &str {
        &self.href
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<'a> std::fmt::Debug for DefinitionBlock<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub struct VariableBlock<'a> {
    id: String,
    type_signature: TypeSignatureBlock<'a>,

    checkable: Variable<'a>,
}

impl<'a> VariableBlock<'a> {
    pub fn new(id: String, type_signature: TypeSignatureBlock<'a>) -> Self {
        let checkable = Variable::new(id.clone());

        VariableBlock {
            id,
            type_signature,

            checkable,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        self.type_signature.crosslink(document);
        self.checkable
            .set_type_signature(self.type_signature.checkable());
    }

    fn checkable(&'a self) -> &Variable {
        &self.checkable
    }
}

impl<'a> std::fmt::Debug for VariableBlock<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub struct VariableBlockRef<'a> {
    index: usize,
    var: OnceCell<&'a VariableBlock<'a>>,
}

impl<'a> VariableBlockRef<'a> {
    pub fn new(index: usize) -> Self {
        VariableBlockRef {
            index,
            var: OnceCell::new(),
        }
    }

    fn crosslink(&'a self, vars: &'a [VariableBlock<'a>]) {
        self.var.set(&vars[self.index]).unwrap();
    }

    fn checkable(&'a self) -> &Variable {
        self.var.get().unwrap().checkable()
    }
}

pub enum FormulaBlock<'a> {
    Symbol(SymbolBlockRef<'a>),
    Variable(VariableBlockRef<'a>),

    Application(Box<FormulaBlock<'a>>, Box<FormulaBlock<'a>>),

    Definition(DefinitionBlockRef<'a>, Vec<FormulaBlock<'a>>),
}

impl<'a> FormulaBlock<'a> {
    pub fn crosslink(&'a self, document: &'a Document<'a>, vars: &'a [VariableBlock<'a>]) {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.crosslink(document),
            Self::Variable(variable_ref) => variable_ref.crosslink(vars),

            Self::Application(left, right) => {
                left.crosslink(document, vars);
                right.crosslink(document, vars);
            }

            Self::Definition(definition_ref, inputs) => {
                definition_ref.crosslink(document);
                for input in inputs {
                    input.crosslink(document, vars);
                }
            }
        }
    }

    pub fn checkable(&'a self) -> Formula<'a> {
        match self {
            Self::Symbol(symbol_ref) => Formula::Symbol(symbol_ref.checkable()),
            Self::Variable(variable_ref) => Formula::Variable(variable_ref.checkable()),

            Self::Application(left, right) => {
                Formula::Application(Box::new(left.checkable()), Box::new(right.checkable()))
            }

            Self::Definition(definition_ref, inputs) => Formula::Definition(
                definition_ref.checkable(),
                inputs.iter().map(FormulaBlock::checkable).collect(),
            ),
        }
    }
}

pub struct DisplayFormulaBlock<'a> {
    display: MathBlock,
    contents: FormulaBlock<'a>,
}

impl<'a> DisplayFormulaBlock<'a> {
    pub fn new(display: MathBlock, contents: FormulaBlock<'a>) -> Self {
        DisplayFormulaBlock { display, contents }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>, vars: &'a [VariableBlock<'a>]) {
        self.contents.crosslink(document, vars);
    }

    pub fn checkable(&'a self) -> Formula<'a> {
        self.contents.checkable()
    }

    // TODO: Remove.
    pub fn render(&self) -> String {
        self.display.render()
    }
}
