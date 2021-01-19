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

use std::cell::Cell;
use std::hash::{Hash, Hasher};

use pest::iterators::{Pair, Pairs};

use crate::document::language::{
    Display, DisplayStyle, FormulaBlock, SymbolBlock, SystemBlock, TypeBlock, TypeSignatureBlock,
    VariableBlock,
};

use super::directory::{
    BibliographyBuilderRef, BuilderDirectory, LocalBibliographyBuilderIndex, LocalIndex,
    ReadSignature, Readable, ReadableKind, SymbolBuilderRef, SystemBuilderRef, TypeBuilderRef,
    VariableBuilderRef,
};
use super::errors::{ParsingError, ParsingErrorContext};
use super::text::{ParagraphBuilder, TextBuilder};
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
                errors.err(ParsingError::SystemMissingName(self_ref));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::SystemDuplicateName(self_ref));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::SystemMissingTagline(self_ref));
            }

            1 => self.taglines[0].verify_structure(directory, errors),

            _ => {
                found_error = true;
                errors.err(ParsingError::SystemDuplicateTagline(self_ref))
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for paragraph in &self.descriptions[0] {
                    paragraph.verify_structure(directory, errors);
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::SystemDuplicateDescription(self_ref));
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
                errors.err(ParsingError::TypeMissingName(self_ref));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::TypeDuplicateName(self_ref));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::TypeMissingTagline(self_ref));
            }

            1 => self.taglines[0].verify_structure(directory, errors),

            _ => {
                found_error = true;
                errors.err(ParsingError::TypeDuplicateTagline(self_ref));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for paragraph in &self.descriptions[0] {
                    paragraph.verify_structure(directory, errors)
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::TypeDuplicateDescription(self_ref));
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
            errors.err(ParsingError::TypeParentNotFound(self_ref));
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

#[derive(Clone)]
pub struct TypeSignatureBuilder {
    inputs: Vec<TypeSignatureBuilder>,
    output: String,
    variable: bool,

    output_ref: Cell<Option<TypeBuilderRef>>,
}

impl TypeSignatureBuilder {
    fn inputs_from_pest(pair: Pair<Rule>) -> Vec<TypeSignatureBuilder> {
        assert_eq!(pair.as_rule(), Rule::type_signature_inputs);

        pair.into_inner()
            .map(|pair| match pair.as_rule() {
                Rule::variable_input_signature => {
                    Self::from_pest(pair.into_inner().next().unwrap(), true)
                }
                Rule::type_signature => Self::from_pest(pair, false),

                _ => unreachable!(),
            })
            .collect()
    }

    fn from_pest(pair: Pair<Rule>, variable: bool) -> TypeSignatureBuilder {
        assert_eq!(pair.as_rule(), Rule::type_signature);

        let mut inner = pair.into_inner();
        let first_pair = inner.next().unwrap();
        let (inputs, output) = match first_pair.as_rule() {
            Rule::type_signature_inputs => (
                Self::inputs_from_pest(first_pair),
                inner.next().unwrap().as_str().to_owned(),
            ),
            Rule::ident => (vec![], first_pair.as_str().to_owned()),

            _ => unreachable!(),
        };

        TypeSignatureBuilder {
            inputs,
            output,
            variable,

            output_ref: Cell::new(None),
        }
    }

    fn verify_structure(
        &self,
        parent_system: &str,
        min_serial: BlockLocation,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        for input in &self.inputs {
            input.verify_structure(parent_system, min_serial, directory, errors);
        }

        if let Some(child) = directory.search_system_child(parent_system, &self.output) {
            let ty = if let Some(ty) = child.ty() {
                ty
            } else {
                todo!()
            };

            if directory[ty].serial() > min_serial {
                todo!()
            }

            self.output_ref.set(Some(ty));
        } else {
            todo!()
        }
    }

    fn finish(&self) -> TypeSignatureBlock {
        let inputs = self
            .inputs
            .iter()
            .map(TypeSignatureBuilder::finish)
            .collect();
        let output = self.output_ref.get().unwrap().finish();
        let variable = self.variable;

        TypeSignatureBlock::new(inputs, output, variable)
    }

    fn applied(&self) -> TypeSignatureBuilder {
        TypeSignatureBuilder {
            inputs: vec![],
            output: self.output.clone(),
            variable: self.variable,

            output_ref: Cell::new(self.output_ref.get()),
        }
    }

    fn arity(&self) -> usize {
        self.inputs.len()
    }

    fn eq_with_var(&self, other: &Self) -> bool {
        if self.output_ref.get().unwrap() != other.output_ref.get().unwrap()
            || self.variable != other.variable
        {
            return false;
        }

        self.inputs
            .iter()
            .zip(&other.inputs)
            .all(|(s, o)| Self::eq_with_var(s, o))
    }

    fn hash_with_var<H: Hasher>(&self, state: &mut H) {
        self.output_ref.get().unwrap().hash(state);
        self.variable.hash(state);

        for input in &self.inputs {
            input.hash(state);
        }
    }
}

impl PartialEq for TypeSignatureBuilder {
    fn eq(&self, other: &Self) -> bool {
        if self.output_ref.get().unwrap() != other.output_ref.get().unwrap() {
            return false;
        }

        self.inputs
            .iter()
            .zip(&other.inputs)
            .all(|(s, o)| Self::eq_with_var(s, o))
    }
}
impl Eq for TypeSignatureBuilder {}

impl Hash for TypeSignatureBuilder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.output_ref.get().unwrap().hash(state);

        for input in &self.inputs {
            input.hash_with_var(state);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum ReadOperator {
    Negation,
    Implies,
}

impl ReadOperator {
    fn from_pest(pair: Pair<Rule>) -> ReadOperator {
        match pair.as_rule() {
            Rule::operator_negation => Self::Negation,
            Rule::operator_implies => Self::Implies,

            _ => unreachable!(),
        }
    }

    fn prec(&self) -> usize {
        match self {
            Self::Negation => todo!(),
            Self::Implies => 0,
        }
    }

    fn is_left_assoc(&self) -> bool {
        match self {
            Self::Negation => todo!(),
            Self::Implies => true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
                        TypeSignatureBuilder::from_pest(pair.into_inner().next().unwrap(), false);

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
                errors.err(ParsingError::SymbolMissingName(symbol_ref));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::SymbolDuplicateName(symbol_ref));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::SymbolMissingTagline(symbol_ref));
            }

            1 => self.taglines[0].verify_structure(directory, errors),

            _ => {
                found_error = true;
                errors.err(ParsingError::SymbolDuplicateTagline(symbol_ref));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for paragraph in &self.descriptions[0] {
                    paragraph.verify_structure(directory, errors);
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::SymbolDuplicateDescription(symbol_ref));
            }
        }

        match self.type_signatures.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::SymbolMissingTypeSignature(symbol_ref));
            }

            1 => self.type_signatures[0].verify_structure(
                parent_system,
                min_serial,
                directory,
                errors,
            ),

            _ => {
                found_error = true;
                errors.err(ParsingError::SymbolDuplicateTypeSignature(symbol_ref));
            }
        }

        if self.reads.len() > 1 {
            found_error = true;
            errors.err(ParsingError::SymbolDuplicateReads(symbol_ref));
        }

        if self.displays.len() > 1 {
            found_error = true;
            errors.err(ParsingError::SymbolDuplicateDisplays(symbol_ref));
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

    fn type_signature(&self) -> &TypeSignatureBuilder {
        assert!(self.verified.get());
        &self.type_signatures[0]
    }

    fn inputs(&self) -> &[TypeSignatureBuilder] {
        assert!(self.verified.get());
        &self.type_signatures[0].inputs
    }

    fn arity(&self) -> usize {
        assert!(self.verified.get());
        self.type_signatures[0].arity()
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
                match read {
                    ReadBuilder {
                        style: ReadStyle::Infix,
                        operator: ReadOperator::Implies,
                    } => Display::new(DisplayStyle::Infix, "\u{21D2}".to_owned()),

                    ReadBuilder {
                        style: ReadStyle::Prefix,
                        operator: ReadOperator::Negation,
                    } => Display::new(DisplayStyle::Prefix, "\u{00AC}".to_owned()),

                    _ => todo!(),
                }
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
            errors.err(ParsingError::SymbolParentNotFound(self_ref));
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
        self.entries
            .read()
            .map(|read| ReadSignature::new(read, self.entries.inputs().to_vec()))
    }

    pub fn as_readable(&self) -> Readable {
        Readable::symbol(self.self_ref.unwrap(), self.entries.arity())
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
        let type_signature = TypeSignatureBuilder::from_pest(inner.next().unwrap(), true);

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

    fn type_signature(&self, vars: &[VariableBuilder]) -> TypeSignatureBuilder {
        vars[self.var_ref.get().unwrap().get()]
            .type_signature
            .clone()
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
            vec![inner_type],
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
        }
    }

    fn type_signature(&self, directory: &BuilderDirectory) -> TypeSignatureBuilder {
        let readable = self.operator_ref.get().unwrap();
        match readable.kind() {
            ReadableKind::Symbol(symbol_ref) => directory[symbol_ref].type_signature().applied(),
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
            vec![lhs_type, rhs_type],
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
        }
    }

    fn type_signature(&self, directory: &BuilderDirectory) -> TypeSignatureBuilder {
        let readable = self.operator_ref.get().unwrap();
        match readable.kind() {
            ReadableKind::Symbol(symbol_ref) => directory[symbol_ref].type_signature().applied(),
        }
    }
}

pub enum FormulaBuilder {
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

    pub fn from_pest(pair: Pair<Rule>) -> FormulaBuilder {
        assert_eq!(pair.as_rule(), Rule::formula);

        Self::prec_climb(&mut pair.into_inner(), 0)
    }

    pub fn build(
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

    pub fn finish(&self) -> FormulaBlock {
        match self {
            Self::Symbol(builder) => todo!(),
            Self::Variable(builder) => builder.finish(),

            Self::Prefix(builder) => builder.finish(),
            Self::Infix(builder) => builder.finish(),
        }
    }

    fn type_signature(
        &self,
        directory: &BuilderDirectory,
        vars: &[VariableBuilder],
    ) -> TypeSignatureBuilder {
        match self {
            Self::Symbol(builder) => todo!(),
            Self::Variable(builder) => builder.type_signature(vars),

            Self::Prefix(builder) => builder.type_signature(directory),
            Self::Infix(builder) => builder.type_signature(directory),
        }
    }
}
