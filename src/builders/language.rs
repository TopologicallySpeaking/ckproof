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

use std::cell::{Cell, UnsafeCell};
use std::hash::{Hash, Hasher};

use pest::iterators::{Pair, Pairs};

use crate::document::language::{
    DefinitionBlock, Display, DisplayFormulaBlock, DisplayStyle, FormulaBlock, SymbolBlock,
    SystemBlock, TypeBlock, TypeSignatureBlock, VariableBlock,
};

use super::directory::{
    BibliographyBuilderRef, BuilderDirectory, DefinitionBuilderRef, LocalBibliographyBuilderIndex,
    LocalIndex, ReadSignature, Readable, ReadableKind, SymbolBuilderRef, SystemBuilderRef,
    TypeBuilderRef, VariableBuilderRef,
};
use super::errors::{
    DefinitionParsingError, ParsingError, ParsingErrorContext, SymbolParsingError,
    SystemParsingError, TypeParsingError,
};
use super::text::{MathBuilder, ParagraphBuilder, TextBuilder};
use super::{BlockLocation, Rule};

struct SystemBuilderEntries {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder>,
    descriptions: Vec<Vec<TextBuilder>>,

    verified: Cell<bool>,
}

impl SystemBuilderEntries {
    fn from_pest(pairs: Pairs<Rule>) -> SystemBuilderEntries {
        let mut names = Vec::with_capacity(1);
        let mut taglines = Vec::with_capacity(1);
        let mut descriptions = Vec::with_capacity(1);

        for pair in pairs {
            match pair.as_rule() {
                Rule::block_name => {
                    let string = pair.into_inner().next().unwrap();
                    let string_contents = string.into_inner().next().unwrap();
                    let name = string_contents.as_str().to_owned();

                    names.push(name);
                }
                Rule::block_tagline => {
                    let tagline = ParagraphBuilder::from_pest(pair.into_inner().next().unwrap());

                    taglines.push(tagline);
                }
                Rule::block_description => {
                    let description = pair.into_inner().map(TextBuilder::from_pest).collect();

                    descriptions.push(description);
                }

                _ => unreachable!(),
            }
        }

        SystemBuilderEntries {
            names,
            taglines,
            descriptions,

            verified: Cell::new(false),
        }
    }

    fn verify_structure(
        &self,
        self_ref: SystemBuilderRef,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.names.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::SystemError(
                    self_ref,
                    SystemParsingError::MissingName,
                ));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::SystemError(
                    self_ref,
                    SystemParsingError::DuplicateName,
                ));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::SystemError(
                    self_ref,
                    SystemParsingError::MissingTagline,
                ));
            }

            1 => self.taglines[0].verify_structure(directory, errors, |e| {
                ParsingError::SystemError(self_ref, SystemParsingError::TaglineParsingError(e))
            }),

            _ => {
                found_error = true;
                errors.err(ParsingError::SystemError(
                    self_ref,
                    SystemParsingError::DuplicateTagline,
                ))
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for paragraph in &self.descriptions[0] {
                    paragraph.verify_structure(directory, errors, |e| {
                        ParsingError::SystemError(
                            self_ref,
                            SystemParsingError::DescriptionParsingError(e),
                        )
                    });
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::SystemError(
                    self_ref,
                    SystemParsingError::DuplicateDescription,
                ));
            }
        }

        self.verified.set(!found_error);
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&self) -> &ParagraphBuilder {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&self) -> &[TextBuilder] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }
}

pub struct SystemBuilder {
    id: String,
    href: String,

    entries: SystemBuilderEntries,

    self_ref: Option<SystemBuilderRef>,
}

impl SystemBuilder {
    pub fn from_pest(pair: Pair<Rule>, href: &str) -> SystemBuilder {
        assert_eq!(pair.as_rule(), Rule::system_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let href = format!("{}#{}", href, id);
        let entries = SystemBuilderEntries::from_pest(inner);

        SystemBuilder {
            id,
            href,
            entries,

            self_ref: None,
        }
    }

    pub fn set_self_ref(&mut self, self_ref: SystemBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(self_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        let self_ref = self.self_ref.unwrap();
        self.entries.verify_structure(self_ref, directory, errors)
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        let tagline_refs = self.entries.tagline().bib_refs();
        let description_refs = self
            .entries
            .description()
            .iter()
            .flat_map(TextBuilder::bib_refs);

        Box::new(tagline_refs.chain(description_refs))
    }

    pub fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        self.entries.tagline().set_local_bib_refs(index);

        for text in self.entries.description() {
            text.set_local_bib_refs(index);
        }
    }

    pub fn finish(&self) -> SystemBlock {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();
        let href = self.href.clone();
        let tagline = self.entries.tagline().finish();
        let description = self
            .entries
            .description()
            .iter()
            .map(TextBuilder::finish)
            .collect();

        SystemBlock::new(id, name, href, tagline, description)
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

struct TypeBuilderEntries {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder>,
    descriptions: Vec<Vec<TextBuilder>>,

    verified: Cell<bool>,
}

impl TypeBuilderEntries {
    fn from_pest(pairs: Pairs<Rule>) -> TypeBuilderEntries {
        let mut names = Vec::with_capacity(1);
        let mut taglines = Vec::with_capacity(1);
        let mut descriptions = Vec::with_capacity(1);

        for pair in pairs {
            match pair.as_rule() {
                Rule::block_name => {
                    let string = pair.into_inner().next().unwrap();
                    let string_contents = string.into_inner().next().unwrap();
                    let name = string_contents.as_str().to_owned();

                    names.push(name);
                }
                Rule::block_tagline => {
                    let tagline = ParagraphBuilder::from_pest(pair.into_inner().next().unwrap());

                    taglines.push(tagline);
                }
                Rule::block_description => {
                    let description = pair.into_inner().map(TextBuilder::from_pest).collect();

                    descriptions.push(description);
                }

                _ => unreachable!(),
            }
        }

        TypeBuilderEntries {
            names,
            taglines,
            descriptions,

            verified: Cell::new(false),
        }
    }

    fn verify_structure(
        &self,
        self_ref: TypeBuilderRef,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.names.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::TypeError(
                    self_ref,
                    TypeParsingError::MissingName,
                ));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::TypeError(
                    self_ref,
                    TypeParsingError::DuplicateName,
                ));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::TypeError(
                    self_ref,
                    TypeParsingError::MissingTagline,
                ));
            }

            1 => self.taglines[0].verify_structure(directory, errors, |e| {
                ParsingError::TypeError(self_ref, TypeParsingError::TaglineParsingError(e))
            }),

            _ => {
                found_error = true;
                errors.err(ParsingError::TypeError(
                    self_ref,
                    TypeParsingError::DuplicateTagline,
                ));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for paragraph in &self.descriptions[0] {
                    paragraph.verify_structure(directory, errors, |e| {
                        ParsingError::TypeError(
                            self_ref,
                            TypeParsingError::DescriptionParsingError(e),
                        )
                    })
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::TypeError(
                    self_ref,
                    TypeParsingError::DuplicateDescription,
                ));
            }
        }

        self.verified.set(!found_error);
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&self) -> &ParagraphBuilder {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&self) -> &[TextBuilder] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }
}

pub struct TypeBuilder {
    id: String,
    system_id: String,
    href: String,
    serial: BlockLocation,

    entries: TypeBuilderEntries,

    self_ref: Option<TypeBuilderRef>,
    system_ref: Cell<Option<SystemBuilderRef>>,
}

impl TypeBuilder {
    pub fn from_pest(pair: Pair<Rule>, serial: BlockLocation, href: &str) -> TypeBuilder {
        assert_eq!(pair.as_rule(), Rule::type_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();
        let href = format!("{}#{}", href, id);
        let entries = TypeBuilderEntries::from_pest(inner);

        TypeBuilder {
            id,
            system_id,
            href,
            serial,

            entries,

            self_ref: None,
            system_ref: Cell::new(None),
        }
    }

    pub fn set_self_ref(&mut self, self_ref: TypeBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(self_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        assert!(self.system_ref.get().is_none());
        let self_ref = self.self_ref.unwrap();

        self.system_ref
            .set(directory.search_system(&self.system_id));
        if self.system_ref.get().is_none() {
            errors.err(ParsingError::TypeError(
                self_ref,
                TypeParsingError::ParentNotFound,
            ));
        }

        self.entries.verify_structure(self_ref, directory, errors);
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        let tagline_refs = self.entries.tagline().bib_refs();
        let description_refs = self
            .entries
            .description()
            .iter()
            .flat_map(TextBuilder::bib_refs);

        Box::new(tagline_refs.chain(description_refs))
    }

    pub fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        self.entries.tagline().set_local_bib_refs(index);

        for text in self.entries.description() {
            text.set_local_bib_refs(index)
        }
    }

    pub fn finish(&self) -> TypeBlock {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();
        let href = self.href.clone();
        let system = self.system_ref.get().unwrap().finish();
        let tagline = self.entries.tagline().finish();
        let description = self
            .entries
            .description()
            .iter()
            .map(TextBuilder::finish)
            .collect();

        TypeBlock::new(id, name, href, system, tagline, description)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    pub fn serial(&self) -> BlockLocation {
        self.serial
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct TypeSignatureGroundBuilder {
    id: String,

    type_ref: Cell<Option<TypeBuilderRef>>,
}

impl TypeSignatureGroundBuilder {
    fn from_pest(pair: Pair<Rule>) -> TypeSignatureGroundBuilder {
        assert_eq!(pair.as_rule(), Rule::ident);

        let id = pair.as_str().to_owned();

        TypeSignatureGroundBuilder {
            id,

            type_ref: Cell::new(None),
        }
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        max_serial: BlockLocation,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        if let Some(child) = directory.search_system_child(parent_system, &self.id) {
            if let Some(ty) = child.ty() {
                if directory[ty].serial <= max_serial {
                    self.type_ref.set(Some(ty))
                } else {
                    todo!()
                }
            } else {
                todo!()
            };
        } else {
            todo!()
        }
    }

    fn finish(&self) -> TypeSignatureBlock {
        TypeSignatureBlock::Ground(self.type_ref.get().unwrap().finish())
    }
}

impl Hash for TypeSignatureGroundBuilder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum TypeSignatureBuilder {
    Ground(TypeSignatureGroundBuilder),
    Compound(Box<TypeSignatureBuilder>, Box<TypeSignatureBuilder>),
}

impl TypeSignatureBuilder {
    fn from_pest_item(pair: Pair<Rule>) -> TypeSignatureBuilder {
        match pair.as_rule() {
            Rule::ident => {
                TypeSignatureBuilder::Ground(TypeSignatureGroundBuilder::from_pest(pair))
            }

            Rule::type_signature => TypeSignatureBuilder::from_pest(pair),

            _ => unreachable!(),
        }
    }

    fn from_pest(pair: Pair<Rule>) -> TypeSignatureBuilder {
        let items: Vec<_> = pair
            .into_inner()
            .map(TypeSignatureBuilder::from_pest_item)
            .collect();

        items
            .into_iter()
            .rev()
            .fold_first(|tail, prev| TypeSignatureBuilder::Compound(Box::new(prev), Box::new(tail)))
            .unwrap()
    }

    fn add_inputs<I>(self, inputs: I) -> TypeSignatureBuilder
    where
        I: IntoIterator<Item = TypeSignatureBuilder>,
    {
        inputs.into_iter().fold(self, |tail, prev| {
            TypeSignatureBuilder::Compound(Box::new(prev), Box::new(tail))
        })
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        max_serial: BlockLocation,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        match self {
            Self::Ground(ground) => {
                ground.verify_structure(parent_system, max_serial, directory, errors)
            }
            Self::Compound(input, output) => {
                input.verify_structure(parent_system, max_serial, directory, errors);
                output.verify_structure(parent_system, max_serial, directory, errors);
            }
        }
    }

    fn finish(&self) -> TypeSignatureBlock {
        match self {
            Self::Ground(ground) => ground.finish(),
            Self::Compound(input, output) => {
                TypeSignatureBlock::Compound(Box::new(input.finish()), Box::new(output.finish()))
            }
        }
    }

    fn ground(&self) -> Option<&TypeSignatureGroundBuilder> {
        match self {
            Self::Ground(ground) => Some(ground),
            Self::Compound(_, _) => None,
        }
    }

    fn compound(&self) -> Option<(&TypeSignatureBuilder, &TypeSignatureBuilder)> {
        match self {
            Self::Ground(_) => None,
            Self::Compound(input, output) => Some((input, output)),
        }
    }

    fn inputs(&self) -> TypeSignatureBuilderInputs {
        TypeSignatureBuilderInputs { curr: self }
    }

    fn applied(&self) -> &TypeSignatureBuilder {
        match self {
            Self::Ground(_) => panic!("Tried to apply an input to a ground type"),
            Self::Compound(_, right) => right,
        }
    }
}

struct TypeSignatureBuilderInputs<'a> {
    curr: &'a TypeSignatureBuilder,
}

impl<'a> Iterator for TypeSignatureBuilderInputs<'a> {
    type Item = &'a TypeSignatureBuilder;

    fn next(&mut self) -> Option<Self::Item> {
        match self.curr {
            TypeSignatureBuilder::Ground(ground) => None,
            TypeSignatureBuilder::Compound(input, output) => {
                self.curr = output;

                Some(input)
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ReadStyle {
    Prefix,
    Infix,
}

impl ReadStyle {
    fn from_pest(pair: Pair<Rule>) -> ReadStyle {
        match pair.as_rule() {
            Rule::style_prefix => Self::Prefix,
            Rule::style_infix => Self::Infix,

            _ => unreachable!(),
        }
    }

    fn to_display(&self) -> DisplayStyle {
        match self {
            ReadStyle::Prefix => DisplayStyle::Prefix,
            ReadStyle::Infix => DisplayStyle::Infix,
        }
    }
}

// TODO: Add arithmetic operators, i.e. +-*/=
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ReadOperator {
    Negation,
    Implies,
    And,
}

impl ReadOperator {
    fn from_pest(pair: Pair<Rule>) -> ReadOperator {
        assert_eq!(pair.as_rule(), Rule::read_operator);

        match pair.into_inner().next().unwrap().as_rule() {
            Rule::operator_negation => Self::Negation,
            Rule::operator_implies => Self::Implies,
            Rule::operator_and => Self::And,

            _ => unreachable!(),
        }
    }

    fn to_display(&self) -> String {
        match self {
            Self::Negation => "\u{00AC}".to_owned(),
            Self::Implies => "\u{21D2}".to_owned(),
            Self::And => "\u{2227}".to_owned(),
        }
    }

    fn prec(&self) -> usize {
        match self {
            Self::Negation => todo!(),
            Self::Implies => 0,
            Self::And => 1,
        }
    }

    fn is_left_assoc(&self) -> bool {
        match self {
            Self::Negation => todo!(),
            Self::Implies | Self::And => true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ReadBuilder {
    style: ReadStyle,
    operator: ReadOperator,
}

impl ReadBuilder {
    fn from_pest(pair: Pair<Rule>) -> ReadBuilder {
        assert_eq!(pair.as_rule(), Rule::read);

        let mut inner = pair.into_inner();
        let style = ReadStyle::from_pest(inner.next().unwrap());
        let operator = ReadOperator::from_pest(inner.next().unwrap());

        ReadBuilder { style, operator }
    }

    fn to_display(&self) -> Display {
        let style = self.style.to_display();
        let operator = self.operator.to_display();

        Display::new(style, operator)
    }
}

impl Display {
    fn from_pest(pair: Pair<Rule>) -> Display {
        assert_eq!(pair.as_rule(), Rule::display);

        todo!()
    }
}

struct SymbolBuilderEntries {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder>,
    descriptions: Vec<Vec<TextBuilder>>,
    type_signatures: Vec<TypeSignatureBuilder>,
    reads: Vec<ReadBuilder>,
    displays: Vec<Display>,

    arity: Cell<Option<usize>>,
    verified: Cell<bool>,
}

impl SymbolBuilderEntries {
    fn from_pest(pairs: Pairs<Rule>) -> SymbolBuilderEntries {
        let mut names = Vec::with_capacity(1);
        let mut taglines = Vec::with_capacity(1);
        let mut descriptions = Vec::with_capacity(1);
        let mut type_signatures = Vec::with_capacity(1);
        let mut reads = Vec::with_capacity(1);
        let mut displays = Vec::with_capacity(1);

        for pair in pairs {
            match pair.as_rule() {
                Rule::block_name => {
                    let string = pair.into_inner().next().unwrap();
                    let string_contents = string.into_inner().next().unwrap();
                    let name = string_contents.as_str().to_owned();

                    names.push(name);
                }

                Rule::block_tagline => {
                    let tagline = ParagraphBuilder::from_pest(pair.into_inner().next().unwrap());

                    taglines.push(tagline);
                }

                Rule::block_description => {
                    let description = pair.into_inner().map(TextBuilder::from_pest).collect();

                    descriptions.push(description);
                }

                Rule::block_type_signature => {
                    let type_signature =
                        TypeSignatureBuilder::from_pest(pair.into_inner().next().unwrap());

                    type_signatures.push(type_signature);
                }

                Rule::block_read => {
                    let read = ReadBuilder::from_pest(pair.into_inner().next().unwrap());

                    reads.push(read);
                }

                Rule::block_display => {
                    let display = Display::from_pest(pair.into_inner().next().unwrap());

                    displays.push(display);
                }

                _ => unreachable!(),
            }
        }

        SymbolBuilderEntries {
            names,
            taglines,
            descriptions,
            type_signatures,
            reads,
            displays,

            arity: Cell::new(None),
            verified: Cell::new(false),
        }
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        symbol_ref: SymbolBuilderRef,
        min_serial: BlockLocation,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.names.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::SymbolError(
                    symbol_ref,
                    SymbolParsingError::MissingName,
                ));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::SymbolError(
                    symbol_ref,
                    SymbolParsingError::DuplicateName,
                ));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::SymbolError(
                    symbol_ref,
                    SymbolParsingError::MissingTagline,
                ));
            }

            1 => self.taglines[0].verify_structure(directory, errors, |e| {
                ParsingError::SymbolError(symbol_ref, SymbolParsingError::TaglineParsingError(e))
            }),

            _ => {
                found_error = true;
                errors.err(ParsingError::SymbolError(
                    symbol_ref,
                    SymbolParsingError::DuplicateTagline,
                ));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for paragraph in &self.descriptions[0] {
                    paragraph.verify_structure(directory, errors, |e| {
                        ParsingError::SymbolError(
                            symbol_ref,
                            SymbolParsingError::DescriptionParsingError(e),
                        )
                    });
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::SymbolError(
                    symbol_ref,
                    SymbolParsingError::DuplicateDescription,
                ));
            }
        }

        match self.type_signatures.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::SymbolError(
                    symbol_ref,
                    SymbolParsingError::MissingTypeSignature,
                ));
            }

            1 => self.type_signatures[0].verify_structure(
                parent_system,
                min_serial,
                directory,
                errors,
            ),

            _ => {
                found_error = true;
                errors.err(ParsingError::SymbolError(
                    symbol_ref,
                    SymbolParsingError::DuplicateTypeSignature,
                ));
            }
        }

        if self.reads.len() > 1 {
            found_error = true;
            errors.err(ParsingError::SymbolError(
                symbol_ref,
                SymbolParsingError::DuplicateReads,
            ));
        }

        if self.displays.len() > 1 {
            found_error = true;
            errors.err(ParsingError::SymbolError(
                symbol_ref,
                SymbolParsingError::DuplicateDisplays,
            ));
        }

        self.verified.set(!found_error);
        self.arity.set(Some(self.type_signature().inputs().count()));
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&self) -> &ParagraphBuilder {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&self) -> &[TextBuilder] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }

    fn type_signature(&self) -> &TypeSignatureBuilder {
        assert!(self.verified.get());
        &self.type_signatures[0]
    }

    fn arity(&self) -> usize {
        assert!(self.verified.get());
        self.arity.get().unwrap()
    }

    fn read(&self) -> Option<ReadBuilder> {
        assert!(self.verified.get());
        self.reads.get(0).copied()
    }

    fn display(&self) -> Display {
        assert!(self.verified.get());

        if let Some(display) = self.displays.get(0) {
            display.clone()
        } else {
            if let Some(read) = self.reads.get(0) {
                read.to_display()
            } else {
                todo!()
            }
        }
    }
}

pub struct SymbolBuilder {
    id: String,
    system_id: String,
    href: String,
    serial: BlockLocation,

    entries: SymbolBuilderEntries,

    self_ref: Option<SymbolBuilderRef>,
    system_ref: Cell<Option<SystemBuilderRef>>,
}

impl SymbolBuilder {
    pub fn from_pest(pair: Pair<Rule>, serial: BlockLocation, href: &str) -> SymbolBuilder {
        assert_eq!(pair.as_rule(), Rule::symbol_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();
        let href = format!("{}#{}_{}", href, system_id, id);

        let entries = SymbolBuilderEntries::from_pest(inner);

        SymbolBuilder {
            id,
            system_id,
            href,
            serial,

            entries,

            self_ref: None,
            system_ref: Cell::new(None),
        }
    }

    pub fn set_self_ref(&mut self, self_ref: SymbolBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(self_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        assert!(self.system_ref.get().is_none());
        let self_ref = self.self_ref.unwrap();

        self.system_ref
            .set(directory.search_system(&self.system_id));
        if self.system_ref.get().is_none() {
            errors.err(ParsingError::SymbolError(
                self_ref,
                SymbolParsingError::ParentNotFound,
            ));
        }

        self.entries
            .verify_structure(&self.system_id, self_ref, self.serial, directory, errors);
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        let tagline = self.entries.tagline().bib_refs();
        let description = self
            .entries
            .description()
            .iter()
            .flat_map(TextBuilder::bib_refs);

        Box::new(tagline.chain(description))
    }

    pub fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        self.entries.tagline().set_local_bib_refs(index);

        for text in self.entries.description() {
            text.set_local_bib_refs(index)
        }
    }

    pub fn finish(&self) -> SymbolBlock {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();
        let href = self.href.clone();
        let system = self.system_ref.get().unwrap().finish();
        let tagline = self.entries.tagline().finish();
        let description = self
            .entries
            .description()
            .iter()
            .map(TextBuilder::finish)
            .collect();

        let type_signature = self.entries.type_signature().finish();
        let display = self.entries.display();

        SymbolBlock::new(
            id,
            name,
            href,
            system,
            tagline,
            description,
            type_signature,
            display,
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    pub fn type_signature(&self) -> &TypeSignatureBuilder {
        self.entries.type_signature()
    }

    pub fn read_signature(&self) -> Option<ReadSignature> {
        self.entries.read().map(|read| {
            ReadSignature::new(
                read,
                self.entries.type_signature().inputs().cloned().collect(),
            )
        })
    }

    pub fn as_readable(&self) -> Readable {
        Readable::symbol(self.self_ref.unwrap(), self.entries.arity())
    }
}

struct DefinitionBuilderEntries {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder>,
    descriptions: Vec<Vec<TextBuilder>>,
    inputs: Vec<Vec<VariableBuilder>>,
    reads: Vec<ReadBuilder>,
    displays: Vec<Display>,
    expansions: Vec<DisplayFormulaBuilder>,

    verified: Cell<bool>,
    type_signature: UnsafeCell<Option<TypeSignatureBuilder>>,
}

impl DefinitionBuilderEntries {
    pub fn from_pest(pairs: Pairs<Rule>) -> DefinitionBuilderEntries {
        let mut names = Vec::with_capacity(1);
        let mut taglines = Vec::with_capacity(1);
        let mut descriptions = Vec::with_capacity(1);
        let mut inputs = Vec::with_capacity(1);
        let mut reads = Vec::with_capacity(1);
        let mut displays = Vec::with_capacity(1);
        let mut expansions = Vec::with_capacity(1);

        for pair in pairs {
            match pair.as_rule() {
                Rule::block_name => {
                    let string = pair.into_inner().next().unwrap();
                    let string_contents = string.into_inner().next().unwrap();
                    let name = string_contents.as_str().to_owned();

                    names.push(name);
                }

                Rule::block_tagline => {
                    let tagline = ParagraphBuilder::from_pest(pair.into_inner().next().unwrap());

                    taglines.push(tagline);
                }

                Rule::block_description => {
                    let description = pair.into_inner().map(TextBuilder::from_pest).collect();

                    descriptions.push(description);
                }

                Rule::block_inputs => {
                    let input = pair.into_inner().map(VariableBuilder::from_pest).collect();

                    inputs.push(input);
                }

                Rule::block_read => {
                    let read = ReadBuilder::from_pest(pair.into_inner().next().unwrap());

                    reads.push(read);
                }

                Rule::block_display => {
                    let display = Display::from_pest(pair.into_inner().next().unwrap());

                    displays.push(display);
                }

                Rule::expanded => {
                    let expanded =
                        DisplayFormulaBuilder::from_pest(pair.into_inner().next().unwrap());

                    expansions.push(expanded);
                }

                _ => unreachable!(),
            }
        }

        DefinitionBuilderEntries {
            names,
            taglines,
            descriptions,
            inputs,
            reads,
            displays,
            expansions,

            verified: Cell::new(false),
            type_signature: UnsafeCell::new(None),
        }
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        definition_ref: DefinitionBuilderRef,
        min_serial: BlockLocation,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.names.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::DefinitionError(
                    definition_ref,
                    DefinitionParsingError::MissingName,
                ));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::DefinitionError(
                    definition_ref,
                    DefinitionParsingError::DuplicateName,
                ));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::DefinitionError(
                    definition_ref,
                    DefinitionParsingError::MissingTagline,
                ));
            }

            1 => self.taglines[0].verify_structure(directory, errors, |e| {
                ParsingError::DefinitionError(
                    definition_ref,
                    DefinitionParsingError::TaglineParsingError(e),
                )
            }),

            _ => {
                found_error = true;
                errors.err(ParsingError::DefinitionError(
                    definition_ref,
                    DefinitionParsingError::DuplicateTagline,
                ));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for paragraph in &self.descriptions[0] {
                    paragraph.verify_structure(directory, errors, |e| {
                        ParsingError::DefinitionError(
                            definition_ref,
                            DefinitionParsingError::DescriptionParsingError(e),
                        )
                    })
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::DefinitionError(
                    definition_ref,
                    DefinitionParsingError::DuplicateDescription,
                ));
            }
        }

        match self.inputs.len() {
            0 => {}
            1 => {
                for var in &self.inputs[0] {
                    var.verify_structure(parent_system, min_serial, directory, errors);
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::DefinitionError(
                    definition_ref,
                    DefinitionParsingError::DuplicateInputs,
                ));
            }
        }

        if self.reads.len() > 1 {
            found_error = true;
            errors.err(ParsingError::DefinitionError(
                definition_ref,
                DefinitionParsingError::DuplicateReads,
            ));
        }

        if self.displays.len() > 1 {
            found_error = true;
            errors.err(ParsingError::DefinitionError(
                definition_ref,
                DefinitionParsingError::DuplicateDisplays,
            ));
        }

        self.verified.set(!found_error);
    }

    fn build_formulas(
        &self,
        self_ref: DefinitionBuilderRef,
        parent_system: &str,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(self.verified.get());
        assert!(unsafe {
            // SAFETY: The only place we get a mutable reference to this is later in this function.
            // So, there are no mutable references because the only place that can happen hasn't
            // happened yet.
            self.type_signature.get().as_ref().unwrap().is_none()
        });

        let local_index = {
            let mut tmp = directory.get_local(parent_system);
            tmp.add_vars(&self.inputs(), errors, |var_ref, e| {
                ParsingError::DefinitionError(
                    self_ref,
                    DefinitionParsingError::VariableError(var_ref, e),
                )
            });
            tmp
        };

        self.expanded()
            .build(&local_index, directory, &self.inputs(), errors);

        let input_types = self.inputs().iter().map(|var| var.type_signature.clone());

        unsafe {
            // SAFETY: This is the only place we write to this field, so we don't need to worry
            // about multiple mutable references. Furthermore, the only time we pass around shared
            // references is with Self::type_signature(). However, if type_signature() successfully
            // returned the value, it would mean we have already called this function previously
            // and the assertion at the beginning of this function would fail. This prevents
            // aliasing.
            let type_signature = self.type_signature.get().as_mut().unwrap();
            *type_signature = Some(
                self.expanded()
                    .type_signature(directory, self.inputs())
                    .clone()
                    .add_inputs(input_types.rev()),
            );
        }
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&self) -> &ParagraphBuilder {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&self) -> &[TextBuilder] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }

    fn inputs(&self) -> &[VariableBuilder] {
        assert!(self.verified.get());
        &self.inputs[0]
    }

    fn arity(&self) -> usize {
        assert!(self.verified.get());
        self.inputs[0].len()
    }

    fn type_signature(&self) -> &TypeSignatureBuilder {
        unsafe {
            // SAFETY: The only time we mutable change this reference is in Self::build_formulas().
            // If we call this function without running build_formulas() first, the unwrap panics.
            // If we call build_formulas() twice, e.g. while this immutable borrow is still alive,
            // it's designed to panic before the mutable borrow.
            let type_signature = self.type_signature.get().as_ref().unwrap();
            type_signature.as_ref().unwrap()
        }
    }

    fn read(&self) -> Option<ReadBuilder> {
        assert!(self.verified.get());
        self.reads.get(0).copied()
    }

    fn display(&self) -> Display {
        assert!(self.verified.get());

        if let Some(display) = self.displays.get(0) {
            display.clone()
        } else {
            if let Some(read) = self.reads.get(0) {
                read.to_display()
            } else {
                todo!()
            }
        }
    }

    fn expanded(&self) -> &DisplayFormulaBuilder {
        &self.expansions[0]
    }
}

pub struct DefinitionBuilder {
    id: String,
    system_id: String,
    href: String,
    serial: BlockLocation,

    entries: DefinitionBuilderEntries,

    self_ref: Option<DefinitionBuilderRef>,
    system_ref: Cell<Option<SystemBuilderRef>>,
}

impl DefinitionBuilder {
    pub fn from_pest(pair: Pair<Rule>, serial: BlockLocation, href: &str) -> DefinitionBuilder {
        assert_eq!(pair.as_rule(), Rule::definition_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();
        let href = format!("{}#{}_{}", href, system_id, id);

        let entries = DefinitionBuilderEntries::from_pest(inner);

        DefinitionBuilder {
            id,
            system_id,
            href,
            serial,

            entries,

            self_ref: None,
            system_ref: Cell::new(None),
        }
    }

    pub fn set_self_ref(&mut self, self_ref: DefinitionBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(self_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        assert!(self.system_ref.get().is_none());
        let self_ref = self.self_ref.unwrap();

        self.system_ref
            .set(directory.search_system(&self.system_id));
        // TODO: This check is already done when adding system children to the index. Remove it.
        if self.system_ref.get().is_none() {
            errors.err(ParsingError::DefinitionError(
                self_ref,
                DefinitionParsingError::ParentNotFound,
            ));
        }

        self.entries
            .verify_structure(&self.system_id, self_ref, self.serial, directory, errors);
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        let tagline = self.entries.tagline().bib_refs();
        let description = self
            .entries
            .description()
            .iter()
            .flat_map(TextBuilder::bib_refs);

        Box::new(tagline.chain(description))
    }

    pub fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        self.entries.tagline().set_local_bib_refs(index);

        for text in self.entries.description() {
            text.set_local_bib_refs(index);
        }
    }

    pub fn build_formulas(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        self.entries
            .build_formulas(self.self_ref.unwrap(), &self.system_id, directory, errors);
    }

    pub fn finish(&self) -> DefinitionBlock {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();
        let href = self.href.clone();
        let system = self.system_ref.get().unwrap().finish();
        let tagline = self.entries.tagline().finish();
        let description = self
            .entries
            .description()
            .iter()
            .map(TextBuilder::finish)
            .collect();

        let type_signature = self.entries.type_signature().finish();
        let inputs = self
            .entries
            .inputs()
            .iter()
            .map(VariableBuilder::finish)
            .collect();
        let display = self.entries.display();
        let expanded = self.entries.expanded().finish();

        DefinitionBlock::new(
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
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    pub fn type_signature(&self) -> &TypeSignatureBuilder {
        self.entries.type_signature()
    }

    pub fn read_signature(&self) -> Option<ReadSignature> {
        self.entries.read().map(|read| {
            ReadSignature::new(
                read,
                self.entries
                    .inputs()
                    .iter()
                    .map(|var| var.type_signature.clone())
                    .collect(),
            )
        })
    }

    pub fn as_readable(&self) -> Readable {
        Readable::definition(self.self_ref.unwrap(), self.entries.arity())
    }
}

pub struct VariableBuilder {
    id: String,
    type_signature: TypeSignatureBuilder,
}

impl VariableBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> VariableBuilder {
        assert_eq!(pair.as_rule(), Rule::var_declaration);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let type_signature = TypeSignatureBuilder::from_pest(inner.next().unwrap());

        VariableBuilder { id, type_signature }
    }

    pub fn verify_structure(
        &self,
        parent_system: &str,
        min_serial: BlockLocation,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        self.type_signature
            .verify_structure(parent_system, min_serial, directory, errors);
    }

    pub fn finish(&self) -> VariableBlock {
        let id = self.id.clone();
        let type_signature = self.type_signature.finish();

        VariableBlock::new(id, type_signature)
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

pub struct FormulaSymbolBuilder {
    id: String,
}

impl FormulaSymbolBuilder {
    fn from_pest(pair: Pair<Rule>) -> FormulaSymbolBuilder {
        assert_eq!(pair.as_rule(), Rule::ident);

        let id = pair.as_str().to_owned();

        FormulaSymbolBuilder { id }
    }
}

pub struct FormulaVariableBuilder {
    id: String,

    var_ref: Cell<Option<VariableBuilderRef>>,
}

impl FormulaVariableBuilder {
    fn from_pest(pair: Pair<Rule>) -> FormulaVariableBuilder {
        assert_eq!(pair.as_rule(), Rule::var);

        let pair = pair.into_inner().next().unwrap();
        let id = pair.as_str().to_owned();

        FormulaVariableBuilder {
            id,

            var_ref: Cell::new(None),
        }
    }

    fn build(&self, local_index: &LocalIndex, errors: &mut ParsingErrorContext) {
        assert!(self.var_ref.get().is_none());

        self.var_ref.set(local_index.search_variable(&self.id));

        if self.var_ref.get().is_none() {
            todo!()
        }
    }

    fn finish(&self) -> FormulaBlock {
        FormulaBlock::Variable(self.var_ref.get().unwrap().finish())
    }

    fn type_signature<'a>(&'a self, vars: &'a [VariableBuilder]) -> &'a TypeSignatureBuilder {
        &vars[self.var_ref.get().unwrap().get()].type_signature
    }
}

pub struct FormulaPrefixBuilder {
    operator: ReadOperator,
    inner: Box<FormulaBuilder>,

    operator_ref: Cell<Option<Readable>>,
}

impl FormulaPrefixBuilder {
    fn from_pest(pair: Pair<Rule>, inner: FormulaBuilder) -> FormulaPrefixBuilder {
        FormulaPrefixBuilder {
            operator: ReadOperator::from_pest(pair),
            inner: Box::new(inner),

            operator_ref: Cell::new(None),
        }
    }

    fn build(
        &self,
        local_index: &LocalIndex,
        directory: &BuilderDirectory,
        vars: &[VariableBuilder],
        errors: &mut ParsingErrorContext,
    ) {
        self.inner.build(local_index, directory, vars, errors);

        let inner_type = self.inner.type_signature(directory, vars);
        let read_signature = ReadSignature::new(
            ReadBuilder {
                style: ReadStyle::Prefix,
                operator: self.operator,
            },
            vec![inner_type.clone()],
        );

        self.operator_ref
            .set(local_index.search_operator(&read_signature));

        if self.operator_ref.get().is_none() {
            todo!()
        }
    }

    fn finish(&self) -> FormulaBlock {
        let inner = self.inner.finish();

        match self.operator_ref.get().unwrap().kind() {
            ReadableKind::Symbol(symbol_ref) => {
                FormulaBlock::SymbolApplication(symbol_ref.finish(), vec![inner])
            }

            ReadableKind::Definition(_) => todo!(),
        }
    }

    fn type_signature<'a>(&'a self, directory: &'a BuilderDirectory) -> &'a TypeSignatureBuilder {
        let readable = self.operator_ref.get().unwrap();
        match readable.kind() {
            ReadableKind::Symbol(symbol_ref) => directory[symbol_ref].type_signature().applied(),
            ReadableKind::Definition(_) => todo!(),
        }
    }
}

pub struct FormulaInfixBuilder {
    operator: ReadOperator,
    lhs: Box<FormulaBuilder>,
    rhs: Box<FormulaBuilder>,

    operator_ref: Cell<Option<Readable>>,
}

impl FormulaInfixBuilder {
    fn from_op(
        operator: ReadOperator,
        lhs: FormulaBuilder,
        rhs: FormulaBuilder,
    ) -> FormulaInfixBuilder {
        FormulaInfixBuilder {
            operator,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),

            operator_ref: Cell::new(None),
        }
    }

    fn build(
        &self,
        local_index: &LocalIndex,
        directory: &BuilderDirectory,
        vars: &[VariableBuilder],
        errors: &mut ParsingErrorContext,
    ) {
        self.lhs.build(local_index, directory, vars, errors);
        self.rhs.build(local_index, directory, vars, errors);

        let lhs_type = self.lhs.type_signature(directory, vars);
        let rhs_type = self.rhs.type_signature(directory, vars);
        let read_signature = ReadSignature::new(
            ReadBuilder {
                style: ReadStyle::Infix,
                operator: self.operator,
            },
            vec![lhs_type.clone(), rhs_type.clone()],
        );

        self.operator_ref
            .set(local_index.search_operator(&read_signature));

        if self.operator_ref.get().is_none() {
            todo!()
        }
    }

    fn finish(&self) -> FormulaBlock {
        let lhs = self.lhs.finish();
        let rhs = self.rhs.finish();

        match self.operator_ref.get().unwrap().kind() {
            ReadableKind::Symbol(symbol_ref) => {
                FormulaBlock::SymbolApplication(symbol_ref.finish(), vec![lhs, rhs])
            }

            ReadableKind::Definition(definition_ref) => {
                FormulaBlock::DefinitionApplication(definition_ref.finish(), vec![lhs, rhs])
            }
        }
    }

    fn type_signature<'a>(&'a self, directory: &'a BuilderDirectory) -> &'a TypeSignatureBuilder {
        let readable = self.operator_ref.get().unwrap();
        match readable.kind() {
            ReadableKind::Symbol(symbol_ref) => {
                directory[symbol_ref].type_signature().applied().applied()
            }

            ReadableKind::Definition(definition_ref) => directory[definition_ref]
                .type_signature()
                .applied()
                .applied(),
        }
    }
}

enum FormulaBuilder {
    Symbol(FormulaSymbolBuilder),
    Variable(FormulaVariableBuilder),

    Prefix(FormulaPrefixBuilder),
    Infix(FormulaInfixBuilder),
}

impl FormulaBuilder {
    fn primary(pair: Pair<Rule>) -> FormulaBuilder {
        match pair.as_rule() {
            Rule::ident => FormulaBuilder::Symbol(FormulaSymbolBuilder::from_pest(pair)),
            Rule::var => FormulaBuilder::Variable(FormulaVariableBuilder::from_pest(pair)),

            Rule::primary_paren => FormulaBuilder::from_pest(pair.into_inner().next().unwrap()),

            _ => unreachable!(),
        }
    }

    fn prec_climb(pairs: &mut Pairs<Rule>, curr_prec: usize) -> FormulaBuilder {
        let prefix_list = pairs.next().unwrap().into_inner();

        let mut primary =
            prefix_list
                .rev()
                .fold(Self::primary(pairs.next().unwrap()), |primary, prefix| {
                    FormulaBuilder::Prefix(FormulaPrefixBuilder::from_pest(prefix, primary))
                });

        // Google "Precedence Climbing".
        while let Some(pair) = pairs.peek() {
            let infix = ReadOperator::from_pest(pair);

            if infix.prec() < curr_prec {
                break;
            }
            pairs.next();

            let next_prec = if infix.is_left_assoc() {
                infix.prec() + 1
            } else {
                infix.prec()
            };

            let rhs = Self::prec_climb(pairs, next_prec);
            primary = FormulaBuilder::Infix(FormulaInfixBuilder::from_op(infix, primary, rhs));
        }

        primary
    }

    fn from_pest(pair: Pair<Rule>) -> FormulaBuilder {
        assert_eq!(pair.as_rule(), Rule::formula);

        Self::prec_climb(&mut pair.into_inner(), 0)
    }

    fn build(
        &self,
        local_index: &LocalIndex,
        directory: &BuilderDirectory,
        vars: &[VariableBuilder],
        errors: &mut ParsingErrorContext,
    ) {
        match self {
            Self::Symbol(builder) => todo!(),
            Self::Variable(builder) => builder.build(local_index, errors),

            Self::Prefix(builder) => builder.build(local_index, directory, vars, errors),
            Self::Infix(builder) => builder.build(local_index, directory, vars, errors),
        }
    }

    fn finish(&self) -> FormulaBlock {
        match self {
            Self::Symbol(builder) => todo!(),
            Self::Variable(builder) => builder.finish(),

            Self::Prefix(builder) => builder.finish(),
            Self::Infix(builder) => builder.finish(),
        }
    }

    fn type_signature<'a>(
        &'a self,
        directory: &'a BuilderDirectory,
        vars: &'a [VariableBuilder],
    ) -> &'a TypeSignatureBuilder {
        match self {
            Self::Symbol(builder) => todo!(),
            Self::Variable(builder) => builder.type_signature(vars),

            Self::Prefix(builder) => builder.type_signature(directory),
            Self::Infix(builder) => builder.type_signature(directory),
        }
    }
}

pub struct DisplayFormulaBuilder {
    display: MathBuilder,
    contents: FormulaBuilder,
}

impl DisplayFormulaBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> DisplayFormulaBuilder {
        let display = MathBuilder::from_pest_formula(pair.clone());
        let contents = FormulaBuilder::from_pest(pair);

        DisplayFormulaBuilder { display, contents }
    }

    pub fn build(
        &self,
        local_index: &LocalIndex,
        directory: &BuilderDirectory,
        vars: &[VariableBuilder],
        errors: &mut ParsingErrorContext,
    ) {
        self.contents.build(local_index, directory, vars, errors);
    }

    pub fn type_signature<'a>(
        &'a self,
        directory: &'a BuilderDirectory,
        vars: &'a [VariableBuilder],
    ) -> &'a TypeSignatureBuilder {
        self.contents.type_signature(directory, vars)
    }

    pub fn finish(&self) -> DisplayFormulaBlock {
        let display = self.display.finish();
        let contents = self.contents.finish();

        DisplayFormulaBlock::new(display, contents)
    }
}
