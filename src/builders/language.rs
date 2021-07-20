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

use std::cell::{Cell, RefCell};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::lazy::OnceCell;
use std::path::Path;

use pest::iterators::{Pair, Pairs};

use crate::document::language::{
    DefinitionBlock, Display, DisplayFormulaBlock, DisplayStyle, FormulaBlock, SymbolBlock,
    TypeBlock, TypeSignatureBlock, VariableBlock, VariableBlockRef,
};
use crate::document::structure::{
    BlockLocation, DefinitionBlockRef, SymbolBlockRef, SystemBlockRef, TypeBlockRef,
};

use super::bibliography::BibliographyBuilderEntry;
use super::errors::{
    DefinitionParsingError, FormulaParsingError, ParsingError, ParsingErrorContext,
    ReadableParsingError, SymbolParsingError, TypeParsingError, TypeSignatureParsingError,
    VariableParsingError,
};
use super::index::{BuilderIndex, LocalBuilderIndex};
use super::system::{DeductableBuilder, SystemBuilder};
use super::text::{MathBuilder, ParagraphBuilder, TextBuilder};
use super::Rule;

struct TypeBuilderEntries<'a> {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder<'a>>,
    descriptions: Vec<Vec<TextBuilder<'a>>>,

    verified: Cell<bool>,
}

impl<'a> TypeBuilderEntries<'a> {
    fn from_pest(path: &Path, pairs: Pairs<Rule>) -> Self {
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
                    let tagline =
                        ParagraphBuilder::from_pest(path, pair.into_inner().next().unwrap());

                    taglines.push(tagline);
                }
                Rule::block_description => {
                    let description = pair
                        .into_inner()
                        .map(|pair| TextBuilder::from_pest(path, pair))
                        .collect();

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
        &'a self,
        type_ref: &'a TypeBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.names.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::TypeError(
                    type_ref,
                    TypeParsingError::MissingName,
                ));
            }

            1 => {}

            _ => {
                found_error = true;
                errors.err(ParsingError::TypeError(
                    type_ref,
                    TypeParsingError::DuplicateName,
                ));
            }
        }

        match self.taglines.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::TypeError(
                    type_ref,
                    TypeParsingError::MissingTagline,
                ));
            }

            1 => {
                let success = self.taglines[0].verify_structure(index, errors, |e| {
                    ParsingError::TypeError(type_ref, TypeParsingError::TaglineParsingError(e))
                });

                if !success {
                    found_error = true;
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::TypeError(
                    type_ref,
                    TypeParsingError::DuplicateTagline,
                ));
            }
        }

        match self.descriptions.len() {
            0 => {}
            1 => {
                for text in &self.descriptions[0] {
                    let success = text.verify_structure(index, errors, |e| {
                        ParsingError::TypeError(
                            type_ref,
                            TypeParsingError::DescriptionParsingError(text, e),
                        )
                    });

                    if !success {
                        found_error = true;
                    }
                }
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::TypeError(
                    type_ref,
                    TypeParsingError::DuplicateDescription,
                ));
            }
        }

        self.verified.set(!found_error);
    }

    fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        let tagline_refs = self.tagline().bib_refs();
        let description_refs = self.description().iter().flat_map(TextBuilder::bib_refs);

        Box::new(tagline_refs.chain(description_refs))
    }

    fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.tagline().set_local_bib_refs(index);
        for text in self.description() {
            text.set_local_bib_refs(index);
        }
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&self) -> &ParagraphBuilder<'a> {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&self) -> &[TextBuilder<'a>] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }
}

pub struct TypeBuilder<'a> {
    id: String,
    system_id: String,
    location: BlockLocation,

    system_ref: OnceCell<&'a SystemBuilder<'a>>,
    entries: TypeBuilderEntries<'a>,
}

impl<'a> TypeBuilder<'a> {
    pub fn from_pest(path: &Path, pair: Pair<Rule>, location: BlockLocation) -> Self {
        assert_eq!(pair.as_rule(), Rule::type_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();
        let entries = TypeBuilderEntries::from_pest(path, inner);

        TypeBuilder {
            id,
            system_id,
            location,

            system_ref: OnceCell::new(),
            entries,
        }
    }

    pub fn set_system_ref(&self, system_ref: &'a SystemBuilder<'a>) {
        self.system_ref.set(system_ref).unwrap();
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.entries.verify_structure(self, index, errors);
    }

    pub fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        self.entries.bib_refs()
    }

    pub fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.entries.set_local_bib_refs(index);
    }

    pub fn finish<'b>(&self) -> TypeBlock<'b> {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();

        let system_location = self.system_ref.get().unwrap().location();
        let system_ref = SystemBlockRef::new(system_location);

        let tagline = self.entries.tagline().finish();
        let description = self
            .entries
            .description()
            .iter()
            .map(TextBuilder::finish)
            .collect();

        TypeBlock::new(id, name, system_ref, tagline, description)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    pub fn serial(&self) -> usize {
        self.location.serial()
    }

    pub fn location(&self) -> BlockLocation {
        self.location
    }
}

impl<'a> std::fmt::Debug for TypeBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Type").field(&self.id).finish()
    }
}

#[derive(Clone, Debug)]
pub struct TypeSignatureBuilderGround<'a> {
    id: String,

    type_ref: OnceCell<&'a TypeBuilder<'a>>,
}

impl<'a> TypeSignatureBuilderGround<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::ident);

        let id = pair.as_str().to_owned();

        TypeSignatureBuilderGround {
            id,

            type_ref: OnceCell::new(),
        }
    }

    fn verify_structure<F>(
        &'a self,
        parent_system: &str,
        max_serial: usize,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(TypeSignatureParsingError<'a>) -> ParsingError<'a>,
    {
        if let Some(child) = index.search_system_child(parent_system, &self.id) {
            if let Some(ty) = child.ty() {
                if ty.serial() < max_serial {
                    self.type_ref.set(ty).unwrap();

                    true
                } else {
                    errors.err(generate_error(TypeSignatureParsingError::ForwardReference(
                        self,
                    )));

                    false
                }
            } else {
                errors.err(generate_error(
                    TypeSignatureParsingError::SystemChildWrongKind(self),
                ));

                false
            }
        } else {
            errors.err(generate_error(TypeSignatureParsingError::TypeIdNotFound(
                self,
            )));

            false
        }
    }

    fn finish<'b>(&self) -> TypeSignatureBlock<'b> {
        let type_location = self.type_ref.get().unwrap().location();
        let type_ref = TypeBlockRef::new(type_location);

        TypeSignatureBlock::Ground(type_ref)
    }
}

impl<'a> PartialEq for TypeSignatureBuilderGround<'a> {
    fn eq(&self, other: &Self) -> bool {
        let self_ref = *self.type_ref.get().unwrap();
        let other_ref = *other.type_ref.get().unwrap();

        std::ptr::eq(self_ref, other_ref)
    }
}
impl<'a> Eq for TypeSignatureBuilderGround<'a> {}

impl<'a> Hash for TypeSignatureBuilderGround<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let self_ref = *self.type_ref.get().unwrap();

        (self_ref as *const TypeBuilder).hash(state);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeSignatureBuilder<'a> {
    Ground(TypeSignatureBuilderGround<'a>),
    Compound(Box<TypeSignatureBuilder<'a>>, Box<TypeSignatureBuilder<'a>>),
}

impl<'a> TypeSignatureBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        let items: Vec<_> = pair
            .into_inner()
            .map(|pair| match pair.as_rule() {
                Rule::ident => {
                    TypeSignatureBuilder::Ground(TypeSignatureBuilderGround::from_pest(pair))
                }
                Rule::type_signature => TypeSignatureBuilder::from_pest(pair),

                _ => unreachable!(),
            })
            .collect();

        items
            .into_iter()
            .rev()
            .reduce(|tail, prev| TypeSignatureBuilder::Compound(Box::new(prev), Box::new(tail)))
            .unwrap()
    }

    fn extend<I>(self, new_inputs: I) -> Self
    where
        I: DoubleEndedIterator<Item = TypeSignatureBuilder<'a>>,
    {
        new_inputs.rev().fold(self, |tail, prev| {
            TypeSignatureBuilder::Compound(Box::new(prev), Box::new(tail))
        })
    }

    fn verify_structure<F>(
        &'a self,
        parent_system: &str,
        max_serial: usize,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(TypeSignatureParsingError<'a>) -> ParsingError<'a> + Copy,
    {
        match self {
            Self::Ground(ground) => {
                ground.verify_structure(parent_system, max_serial, index, errors, generate_error)
            }

            Self::Compound(input, output) => {
                let input_success = input.verify_structure(
                    parent_system,
                    max_serial,
                    index,
                    errors,
                    generate_error,
                );
                let output_success = output.verify_structure(
                    parent_system,
                    max_serial,
                    index,
                    errors,
                    generate_error,
                );

                input_success && output_success
            }
        }
    }

    fn finish<'b>(&self) -> TypeSignatureBlock<'b> {
        match self {
            Self::Ground(ground) => ground.finish(),

            Self::Compound(input, output) => {
                TypeSignatureBlock::Compound(Box::new(input.finish()), Box::new(output.finish()))
            }
        }
    }

    fn inputs(&'a self) -> TypeSignatureBuilderInputs {
        TypeSignatureBuilderInputs { curr: self }
    }

    fn applied(&'a self) -> &TypeSignatureBuilder {
        match self {
            Self::Ground(_) => panic!("Tried to apply an input to a ground type"),
            Self::Compound(_, right) => right,
        }
    }
}

struct TypeSignatureBuilderInputs<'a> {
    curr: &'a TypeSignatureBuilder<'a>,
}

impl<'a> Iterator for TypeSignatureBuilderInputs<'a> {
    type Item = &'a TypeSignatureBuilder<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.curr {
            TypeSignatureBuilder::Ground(_) => None,
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ReadOperator {
    Negation,
    Implies,
    Equivalent,
    And,
    Or,

    Asterisk,
    Slash,
    Plus,
    Minus,
}

impl ReadOperator {
    fn from_pest(pair: Pair<Rule>) -> ReadOperator {
        assert_eq!(pair.as_rule(), Rule::read_operator);

        // TODO: operator_lt, operator_eq, and operator_gt
        match pair.into_inner().next().unwrap().as_rule() {
            Rule::operator_negation => Self::Negation,
            Rule::operator_implies => Self::Implies,
            Rule::operator_equiv => Self::Equivalent,
            Rule::operator_and => Self::And,
            Rule::operator_or => Self::Or,

            Rule::operator_asterisk => Self::Asterisk,
            Rule::operator_slash => Self::Slash,
            Rule::operator_plus => Self::Plus,
            Rule::operator_minus => Self::Minus,

            _ => unreachable!(),
        }
    }

    fn precedence(self) -> usize {
        match self {
            Self::Negation => todo!(),

            Self::Equivalent => 0,
            Self::Implies => 1,
            Self::And => 2,
            Self::Or => 3,

            Self::Asterisk | Self::Slash => 4,
            Self::Plus | Self::Minus => 5,
        }
    }

    fn is_left_associative(self) -> bool {
        match self {
            Self::Negation => todo!(),

            Self::Asterisk | Self::Slash | Self::Plus | Self::Minus => true,

            Self::Equivalent | Self::Implies | Self::And | Self::Or => false,
        }
    }

    fn to_display(&self) -> &str {
        match self {
            Self::Negation => "\u{00AC}",
            Self::Implies => "\u{2192}",
            Self::Equivalent => "\u{21D4}",
            Self::And => "\u{2227}",
            Self::Or => "\u{2228}",

            Self::Asterisk => "\u{22C5}",
            Self::Slash => "/",
            Self::Plus => "+",
            Self::Minus => "-",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct ReadBuilder {
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
        let operator = self.operator.to_display().to_owned();

        Display::new(style, operator)
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct ReadSignature<'a> {
    read: ReadBuilder,
    inputs: Vec<&'a TypeSignatureBuilder<'a>>,
}

impl Display {
    fn from_pest(pair: Pair<Rule>) -> Display {
        assert_eq!(pair.as_rule(), Rule::display);

        todo!()
    }
}

#[derive(Default)]
struct PropertyList<'a> {
    reflexive: OnceCell<DeductableBuilder<'a>>,
    symmetric: OnceCell<DeductableBuilder<'a>>,
    transitive: OnceCell<DeductableBuilder<'a>>,

    function: RefCell<HashMap<ReadableBuilder<'a>, DeductableBuilder<'a>>>,
}

impl<'a> PropertyList<'a> {
    fn new() -> Self {
        PropertyList::default()
    }

    fn set_reflexive(
        &self,
        readable_ref: ReadableBuilder<'a>,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        if let Err(deductable_ref) = self.reflexive.set(deductable_ref) {
            errors.err(ParsingError::ReadableError(
                readable_ref,
                ReadableParsingError::DuplicateReflexive(deductable_ref),
            ));
        }
    }

    fn get_reflexive(&'a self) -> Option<DeductableBuilder> {
        self.reflexive.get().copied()
    }

    fn set_symmetric(
        &self,
        readable_ref: ReadableBuilder<'a>,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        if let Err(deductable_ref) = self.symmetric.set(deductable_ref) {
            errors.err(ParsingError::ReadableError(
                readable_ref,
                ReadableParsingError::DuplicateSymmetric(deductable_ref),
            ));
        }
    }

    fn get_symmetric(&'a self) -> Option<DeductableBuilder> {
        self.symmetric.get().copied()
    }

    fn set_transitive(
        &self,
        readable_ref: ReadableBuilder<'a>,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        if let Err(deductable_ref) = self.transitive.set(deductable_ref) {
            errors.err(ParsingError::ReadableError(
                readable_ref,
                ReadableParsingError::DuplicateTransitive(deductable_ref),
            ));
        }
    }

    // TODO: Allow inputs to use different preorders.
    fn set_function(
        &self,
        readable_ref: ReadableBuilder<'a>,
        deductable_ref: DeductableBuilder<'a>,
        relation: ReadableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        let mut function = self.function.borrow_mut();
        match function.entry(relation) {
            Entry::Occupied(_) => {
                errors.err(ParsingError::ReadableError(
                    readable_ref,
                    ReadableParsingError::DuplicateFunction(relation, deductable_ref),
                ));
            }

            Entry::Vacant(slot) => {
                slot.insert(deductable_ref);
            }
        }
    }

    fn get_function(&'a self, relation: ReadableBuilder<'a>) -> Option<DeductableBuilder> {
        self.function.borrow().get(&relation).copied()
    }

    fn is_reflexive(&self) -> bool {
        self.reflexive.get().is_some()
    }

    fn is_symmetric(&self) -> bool {
        self.symmetric.get().is_some()
    }

    fn is_transitive(&self) -> bool {
        self.transitive.get().is_some()
    }

    fn is_preorder(&self) -> bool {
        self.is_reflexive() && self.is_transitive()
    }
}

struct SymbolBuilderEntries<'a> {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder<'a>>,
    descriptions: Vec<Vec<TextBuilder<'a>>>,
    type_signatures: Vec<TypeSignatureBuilder<'a>>,
    reads: Vec<ReadBuilder>,
    displays: Vec<Display>,

    verified: Cell<bool>,
}

impl<'a> SymbolBuilderEntries<'a> {
    fn from_pest(path: &Path, pairs: Pairs<Rule>) -> Self {
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
                    let tagline =
                        ParagraphBuilder::from_pest(path, pair.into_inner().next().unwrap());

                    taglines.push(tagline);
                }

                Rule::block_description => {
                    let description = pair
                        .into_inner()
                        .map(|pair| TextBuilder::from_pest(path, pair))
                        .collect();

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

            verified: Cell::new(false),
        }
    }

    fn verify_structure(
        &'a self,
        symbol_ref: &'a SymbolBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
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

            1 => {
                let success = self.taglines[0].verify_structure(index, errors, |e| {
                    ParsingError::SymbolError(
                        symbol_ref,
                        SymbolParsingError::TaglineParsingError(e),
                    )
                });

                if !success {
                    found_error = true;
                }
            }

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
                for text in &self.descriptions[0] {
                    let success = text.verify_structure(index, errors, |e| {
                        ParsingError::SymbolError(
                            symbol_ref,
                            SymbolParsingError::DescriptionParsingError(text, e),
                        )
                    });

                    if !success {
                        found_error = true;
                    }
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

            1 => {
                let success = self.type_signatures[0].verify_structure(
                    &symbol_ref.system_id,
                    symbol_ref.serial(),
                    index,
                    errors,
                    |e| {
                        ParsingError::SymbolError(
                            symbol_ref,
                            SymbolParsingError::TypeSignatureError(e),
                        )
                    },
                );

                if !success {
                    found_error = true;
                }
            }

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
    }

    fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        let tagline = self.tagline().bib_refs();
        let description = self.description().iter().flat_map(TextBuilder::bib_refs);

        Box::new(tagline.chain(description))
    }

    fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.tagline().set_local_bib_refs(index);
        for text in self.description() {
            text.set_local_bib_refs(index);
        }
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&self) -> &ParagraphBuilder<'a> {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&self) -> &[TextBuilder<'a>] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }

    fn type_signature(&self) -> &TypeSignatureBuilder<'a> {
        assert!(self.verified.get());
        &self.type_signatures[0]
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

pub struct SymbolBuilder<'a> {
    id: String,
    system_id: String,
    location: BlockLocation,

    system_ref: OnceCell<&'a SystemBuilder<'a>>,
    entries: SymbolBuilderEntries<'a>,

    properties: PropertyList<'a>,

    // TODO: Remove.
    href: OnceCell<String>,
}

impl<'a> SymbolBuilder<'a> {
    pub fn from_pest(path: &Path, pair: Pair<Rule>, location: BlockLocation) -> Self {
        assert_eq!(pair.as_rule(), Rule::symbol_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();

        let entries = SymbolBuilderEntries::from_pest(path, inner);

        SymbolBuilder {
            id,
            system_id,
            location,

            system_ref: OnceCell::new(),
            entries,

            properties: PropertyList::new(),

            href: OnceCell::new(),
        }
    }

    pub fn set_system_ref(&self, system_ref: &'a SystemBuilder<'a>) {
        self.system_ref.set(system_ref).unwrap();
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.entries.verify_structure(self, index, errors);
    }

    pub fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        self.entries.bib_refs()
    }

    pub fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.entries.set_local_bib_refs(index);
    }

    pub fn set_reflexive(
        &'a self,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.properties
            .set_reflexive(ReadableBuilder::Symbol(self), deductable_ref, errors);
    }

    pub fn get_reflexive(&'a self) -> Option<DeductableBuilder> {
        self.properties.get_reflexive()
    }

    pub fn set_symmetric(
        &'a self,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.properties
            .set_symmetric(ReadableBuilder::Symbol(self), deductable_ref, errors);
    }

    pub fn get_symmetric(&'a self) -> Option<DeductableBuilder> {
        self.properties.get_symmetric()
    }

    pub fn set_transitive(
        &'a self,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.properties
            .set_transitive(ReadableBuilder::Symbol(self), deductable_ref, errors);
    }

    pub fn set_function(
        &'a self,
        deductable_ref: DeductableBuilder<'a>,
        relation: ReadableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.properties.set_function(
            ReadableBuilder::Symbol(self),
            deductable_ref,
            relation,
            errors,
        );
    }

    pub fn get_function(&'a self, relation: ReadableBuilder<'a>) -> Option<DeductableBuilder> {
        self.properties.get_function(relation)
    }

    pub fn is_reflexive(&self) -> bool {
        self.properties.is_reflexive()
    }

    pub fn is_symmetric(&self) -> bool {
        self.properties.is_symmetric()
    }

    pub fn is_transitive(&self) -> bool {
        self.properties.is_transitive()
    }

    pub fn is_preorder(&self) -> bool {
        self.properties.is_preorder()
    }

    // TODO: Remove.
    pub fn set_href(&self, book_id: &str, chapter_id: &str, page_id: &str) {
        let href = format!(
            "/{}/{}/{}#{}_{}",
            book_id, chapter_id, page_id, &self.system_id, &self.id
        );
        self.href.set(href).unwrap();
    }

    pub fn finish<'b>(&self) -> SymbolBlock<'b> {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();

        let system_location = self.system_ref.get().unwrap().location();
        let system_ref = SystemBlockRef::new(system_location);

        let tagline = self.entries.tagline().finish();
        let description = self
            .entries
            .description()
            .iter()
            .map(TextBuilder::finish)
            .collect();

        let type_signature = self.entries.type_signature().finish();
        let display = self.entries.display();

        let href = self.href.get().unwrap().clone();

        SymbolBlock::new(
            id,
            name,
            system_ref,
            tagline,
            description,
            type_signature,
            display,
            href,
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    pub fn type_signature(&'a self) -> &TypeSignatureBuilder {
        self.entries.type_signature()
    }

    pub fn read_signature(&'a self) -> Option<ReadSignature> {
        self.entries.read().map(|read| ReadSignature {
            read,
            inputs: self.entries.type_signature().inputs().collect(),
        })
    }

    pub fn serial(&self) -> usize {
        self.location.serial()
    }

    pub fn location(&self) -> BlockLocation {
        self.location
    }
}

impl<'a> PartialEq for SymbolBuilder<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<'a> Eq for SymbolBuilder<'a> {}

impl<'a> Hash for SymbolBuilder<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.system_id.hash(state);
        self.id.hash(state);
    }
}

impl<'a> std::fmt::Debug for SymbolBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Symbol").field(&self.id).finish()
    }
}

struct DefinitionBuilderEntries<'a> {
    names: Vec<String>,
    taglines: Vec<ParagraphBuilder<'a>>,
    descriptions: Vec<Vec<TextBuilder<'a>>>,
    inputs: Vec<Vec<VariableBuilder<'a>>>,
    reads: Vec<ReadBuilder>,
    displays: Vec<Display>,
    expansions: Vec<DisplayFormulaBuilder<'a>>,

    verified: Cell<bool>,
}

impl<'a> DefinitionBuilderEntries<'a> {
    pub fn from_pest(path: &Path, pairs: Pairs<Rule>) -> Self {
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
                    let tagline =
                        ParagraphBuilder::from_pest(path, pair.into_inner().next().unwrap());

                    taglines.push(tagline);
                }

                Rule::block_description => {
                    let description = pair
                        .into_inner()
                        .map(|pair| TextBuilder::from_pest(path, pair))
                        .collect();

                    descriptions.push(description);
                }

                Rule::block_inputs => {
                    let input = pair
                        .into_inner()
                        .enumerate()
                        .map(|(index, pair)| VariableBuilder::from_pest(pair, index))
                        .collect();

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
        }
    }

    fn verify_structure(
        &'a self,
        definition_ref: &'a DefinitionBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
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

            1 => {
                let success = self.taglines[0].verify_structure(index, errors, |e| {
                    ParsingError::DefinitionError(
                        definition_ref,
                        DefinitionParsingError::TaglineParsingError(e),
                    )
                });

                if !success {
                    found_error = true;
                }
            }

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
                for text in &self.descriptions[0] {
                    let success = text.verify_structure(index, errors, |e| {
                        ParsingError::DefinitionError(
                            definition_ref,
                            DefinitionParsingError::DescriptionParsingError(text, e),
                        )
                    });

                    if !success {
                        found_error = true;
                    }
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
                    var.verify_structure(
                        &definition_ref.system_id,
                        definition_ref.serial(),
                        index,
                        errors,
                        |e| {
                            ParsingError::DefinitionError(
                                definition_ref,
                                DefinitionParsingError::VariableError(var, e),
                            )
                        },
                    );
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

        match self.expansions.len() {
            0 => {
                found_error = true;
                errors.err(ParsingError::DefinitionError(
                    definition_ref,
                    DefinitionParsingError::MissingExpansion,
                ));
            }

            1 => {
                self.expansions[0].verify_structure(errors);
            }

            _ => {
                found_error = true;
                errors.err(ParsingError::DefinitionError(
                    definition_ref,
                    DefinitionParsingError::DuplicateExpansion,
                ));
            }
        }

        self.verified.set(!found_error);
    }

    fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        let tagline = self.tagline().bib_refs();
        let description = self.description().iter().flat_map(TextBuilder::bib_refs);

        Box::new(tagline.chain(description))
    }

    fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.tagline().set_local_bib_refs(index);

        for text in self.description() {
            text.set_local_bib_refs(index);
        }
    }

    fn build_formulas(
        &'a self,
        definition_ref: &'a DefinitionBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        assert!(self.verified.get());

        let local_index = index.get_local(definition_ref.system_id(), self.inputs());
        let success = self.expanded().build(&local_index, errors, |formula, e| {
            ParsingError::DefinitionError(
                definition_ref,
                DefinitionParsingError::FormulaError(formula, e),
            )
        });

        if !success {
            return;
        }

        let input_signatures = self.inputs().iter().map(|var| var.type_signature().clone());
        definition_ref
            .type_signature
            .set(
                self.expanded()
                    .type_signature()
                    .clone()
                    .extend(input_signatures),
            )
            .unwrap()
    }

    fn name(&self) -> &str {
        assert!(self.verified.get());
        &self.names[0]
    }

    fn tagline(&self) -> &ParagraphBuilder<'a> {
        assert!(self.verified.get());
        &self.taglines[0]
    }

    fn description(&self) -> &[TextBuilder<'a>] {
        assert!(self.verified.get());
        if self.descriptions.is_empty() {
            &[]
        } else {
            &self.descriptions[0]
        }
    }

    fn inputs(&self) -> &[VariableBuilder<'a>] {
        assert!(self.verified.get());
        &self.inputs[0]
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

    fn expanded(&self) -> &DisplayFormulaBuilder<'a> {
        assert!(self.verified.get());
        &self.expansions[0]
    }
}

pub struct DefinitionBuilder<'a> {
    id: String,
    system_id: String,
    location: BlockLocation,

    system_ref: OnceCell<&'a SystemBuilder<'a>>,
    type_signature: OnceCell<TypeSignatureBuilder<'a>>,

    entries: DefinitionBuilderEntries<'a>,

    properties: PropertyList<'a>,

    // TODO: Remove.
    href: OnceCell<String>,
}

impl<'a> DefinitionBuilder<'a> {
    pub fn from_pest(path: &Path, pair: Pair<Rule>, location: BlockLocation) -> Self {
        assert_eq!(pair.as_rule(), Rule::definition_block);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let system_id = inner.next().unwrap().as_str().to_owned();

        let entries = DefinitionBuilderEntries::from_pest(path, inner);

        DefinitionBuilder {
            id,
            system_id,
            location,

            system_ref: OnceCell::new(),
            type_signature: OnceCell::new(),

            entries,

            properties: PropertyList::new(),

            href: OnceCell::new(),
        }
    }

    pub fn set_system_ref(&self, system_ref: &'a SystemBuilder<'a>) {
        self.system_ref.set(system_ref).unwrap();
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.entries.verify_structure(self, index, errors);
    }

    pub fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        self.entries.bib_refs()
    }

    pub fn set_local_bib_refs(&'a self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.entries.set_local_bib_refs(index);
    }

    pub fn build_formulas(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.entries.build_formulas(self, index, errors);
    }

    pub fn set_reflexive(
        &'a self,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.properties
            .set_reflexive(ReadableBuilder::Definition(self), deductable_ref, errors);
    }

    pub fn get_reflexive(&'a self) -> Option<DeductableBuilder> {
        self.properties.get_reflexive()
    }

    pub fn set_symmetric(
        &'a self,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.properties
            .set_symmetric(ReadableBuilder::Definition(self), deductable_ref, errors);
    }

    pub fn get_symmetric(&'a self) -> Option<DeductableBuilder> {
        self.properties.get_symmetric()
    }

    pub fn set_transitive(
        &'a self,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.properties
            .set_transitive(ReadableBuilder::Definition(self), deductable_ref, errors);
    }

    pub fn set_function(
        &'a self,
        deductable_ref: DeductableBuilder<'a>,
        relation: ReadableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.properties.set_function(
            ReadableBuilder::Definition(self),
            deductable_ref,
            relation,
            errors,
        );
    }

    pub fn get_function(&'a self, relation: ReadableBuilder<'a>) -> Option<DeductableBuilder> {
        self.properties.get_function(relation)
    }

    pub fn is_reflexive(&self) -> bool {
        self.properties.is_reflexive()
    }

    pub fn is_symmetric(&self) -> bool {
        self.properties.is_symmetric()
    }

    pub fn is_transitive(&self) -> bool {
        self.properties.is_transitive()
    }

    pub fn is_preorder(&self) -> bool {
        self.properties.is_preorder()
    }

    // TODO: Remove.
    pub fn set_href(&self, book_id: &str, chapter_id: &str, page_id: &str) {
        let href = format!(
            "/{}/{}/{}#{}_{}",
            book_id, chapter_id, page_id, &self.system_id, &self.id
        );
        self.href.set(href).unwrap();
    }

    pub fn finish<'b>(&self) -> DefinitionBlock<'b> {
        let id = self.id.clone();
        let name = self.entries.name().to_owned();

        let system_location = self.system_ref.get().unwrap().location();
        let system_ref = SystemBlockRef::new(system_location);

        let tagline = self.entries.tagline().finish();
        let description = self
            .entries
            .description()
            .iter()
            .map(TextBuilder::finish)
            .collect();

        let type_signature = self.type_signature.get().unwrap().finish();
        let display = self.entries.display();
        let inputs = self
            .entries
            .inputs()
            .iter()
            .map(VariableBuilder::finish)
            .collect();
        let expanded = self.entries.expanded().finish();

        let href = self.href.get().unwrap().clone();

        DefinitionBlock::new(
            id,
            name,
            system_ref,
            tagline,
            description,
            display,
            inputs,
            type_signature,
            expanded,
            href,
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    pub fn type_signature(&'a self) -> &TypeSignatureBuilder {
        &self.type_signature.get().unwrap()
    }

    pub fn read_signature(&'a self) -> Option<ReadSignature> {
        self.entries.read().map(|read| ReadSignature {
            read,
            inputs: self
                .entries
                .inputs()
                .iter()
                .map(|var| var.type_signature())
                .collect(),
        })
    }

    pub fn serial(&self) -> usize {
        self.location.serial()
    }

    pub fn location(&self) -> BlockLocation {
        self.location
    }
}

impl<'a> PartialEq for DefinitionBuilder<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<'a> Eq for DefinitionBuilder<'a> {}

impl<'a> Hash for DefinitionBuilder<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.system_id.hash(state);
        self.id.hash(state);
    }
}

impl<'a> std::fmt::Debug for DefinitionBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Definition").field(&self.id).finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReadableBuilder<'a> {
    Symbol(&'a SymbolBuilder<'a>),
    Definition(&'a DefinitionBuilder<'a>),
}

impl<'a> ReadableBuilder<'a> {
    pub fn id(&self) -> &str {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.id(),
            Self::Definition(definition_ref) => definition_ref.id(),
        }
    }

    pub fn system_id(&self) -> &str {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.system_id(),
            Self::Definition(definition_ref) => definition_ref.system_id(),
        }
    }

    fn type_signature(&'a self) -> &TypeSignatureBuilder {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.type_signature(),
            Self::Definition(definition_ref) => definition_ref.type_signature(),
        }
    }

    pub fn set_reflexive(
        &self,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.set_reflexive(deductable_ref, errors),
            Self::Definition(definition_ref) => {
                definition_ref.set_reflexive(deductable_ref, errors)
            }
        }
    }

    pub fn get_reflexive(self) -> Option<DeductableBuilder<'a>> {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.get_reflexive(),
            Self::Definition(definition_ref) => definition_ref.get_reflexive(),
        }
    }

    pub fn set_symmetric(
        &self,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.set_symmetric(deductable_ref, errors),
            Self::Definition(definition_ref) => {
                definition_ref.set_symmetric(deductable_ref, errors)
            }
        }
    }

    pub fn get_symmetric(self) -> Option<DeductableBuilder<'a>> {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.get_symmetric(),
            Self::Definition(definition_ref) => definition_ref.get_symmetric(),
        }
    }

    pub fn set_transitive(
        &self,
        deductable_ref: DeductableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.set_transitive(deductable_ref, errors),
            Self::Definition(definition_ref) => {
                definition_ref.set_transitive(deductable_ref, errors)
            }
        }
    }

    pub fn set_function(
        &self,
        deductable_ref: DeductableBuilder<'a>,
        relation: ReadableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.set_function(deductable_ref, relation, errors),
            Self::Definition(definition_ref) => {
                definition_ref.set_function(deductable_ref, relation, errors)
            }
        }
    }

    pub fn get_function(self, relation: ReadableBuilder<'a>) -> Option<DeductableBuilder> {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.get_function(relation),
            Self::Definition(definition_ref) => definition_ref.get_function(relation),
        }
    }

    pub fn is_preorder(&self) -> bool {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.is_preorder(),
            Self::Definition(definition_ref) => definition_ref.is_preorder(),
        }
    }

    pub fn is_symmetric(&self) -> bool {
        match self {
            Self::Symbol(symbol_ref) => symbol_ref.is_symmetric(),
            Self::Definition(definition_ref) => definition_ref.is_symmetric(),
        }
    }
}

#[derive(Debug)]
pub struct VariableBuilder<'a> {
    id: String,
    index: usize,

    type_signature: TypeSignatureBuilder<'a>,
}

impl<'a> VariableBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>, index: usize) -> Self {
        assert_eq!(pair.as_rule(), Rule::var_declaration);

        let mut inner = pair.into_inner();
        let id = inner.next().unwrap().as_str().to_owned();
        let type_signature = TypeSignatureBuilder::from_pest(inner.next().unwrap());

        VariableBuilder {
            id,
            index,

            type_signature,
        }
    }

    pub fn verify_structure<F>(
        &'a self,
        parent_system: &str,
        max_serial: usize,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(VariableParsingError<'a>) -> ParsingError<'a>,
    {
        self.type_signature
            .verify_structure(parent_system, max_serial, index, errors, |e| {
                generate_error(VariableParsingError::TypeSignatureError(e))
            })
    }

    pub fn finish<'b>(&self) -> VariableBlock<'b> {
        let id = self.id.clone();
        let type_signature = self.type_signature.finish();

        VariableBlock::new(id, type_signature)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn type_signature(&'a self) -> &TypeSignatureBuilder {
        &self.type_signature
    }
}

impl<'a> PartialEq for VariableBuilder<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<'a> Eq for VariableBuilder<'a> {}

#[derive(Clone, Debug)]
pub struct FormulaSymbolBuilder<'a> {
    id: String,

    symbol_ref: OnceCell<&'a SymbolBuilder<'a>>,
}

impl<'a> FormulaSymbolBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::ident);

        let id = pair.as_str().to_owned();

        FormulaSymbolBuilder {
            id,

            symbol_ref: OnceCell::new(),
        }
    }
}

impl<'a> PartialEq for FormulaSymbolBuilder<'a> {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}
impl<'a> Eq for FormulaSymbolBuilder<'a> {}

#[derive(Clone, Debug)]
pub struct FormulaVariableBuilder<'a> {
    id: String,

    var_ref: OnceCell<&'a VariableBuilder<'a>>,
}

impl<'a> FormulaVariableBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::var);

        let pair = pair.into_inner().next().unwrap();
        let id = pair.as_str().to_owned();

        FormulaVariableBuilder {
            id,

            var_ref: OnceCell::new(),
        }
    }

    fn build<F>(
        &'a self,
        formula_ref: &'a FormulaBuilder<'a>,
        local_index: &LocalBuilderIndex<'a, '_>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(&'a FormulaBuilder<'a>, FormulaParsingError) -> ParsingError<'a>,
    {
        assert!(self.var_ref.get().is_none());

        match local_index.search_variable(&self.id) {
            Some(var) => {
                self.var_ref.set(var).unwrap();
                true
            }

            None => {
                errors.err(generate_error(
                    formula_ref,
                    FormulaParsingError::VariableIdNotFound,
                ));
                false
            }
        }
    }

    fn finish<'b>(&self) -> FormulaBlock<'b> {
        let index = self.var_ref.get().unwrap().index();
        let var_ref = VariableBlockRef::new(index);

        FormulaBlock::Variable(var_ref)
    }

    fn type_signature(&'a self) -> &TypeSignatureBuilder {
        self.var_ref.get().unwrap().type_signature()
    }
}

impl<'a> PartialEq for FormulaVariableBuilder<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.var_ref.get().unwrap() == other.var_ref.get().unwrap()
    }
}
impl<'a> Eq for FormulaVariableBuilder<'a> {}

#[derive(Clone, Debug)]
pub struct FormulaPrefixBuilder<'a> {
    operator: ReadOperator,
    inner: Box<FormulaBuilder<'a>>,

    operator_ref: OnceCell<ReadableBuilder<'a>>,
}

impl<'a> FormulaPrefixBuilder<'a> {
    fn from_pest(pair: Pair<Rule>, inner: FormulaBuilder<'a>) -> Self {
        FormulaPrefixBuilder {
            operator: ReadOperator::from_pest(pair),
            inner: Box::new(inner),

            operator_ref: OnceCell::new(),
        }
    }

    fn build<F>(
        &'a self,
        formula_ref: &'a FormulaBuilder<'a>,
        local_index: &LocalBuilderIndex<'a, '_>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(&'a FormulaBuilder<'a>, FormulaParsingError) -> ParsingError<'a> + Copy,
    {
        if !self.inner.build(local_index, errors, generate_error) {
            return false;
        };

        let inner_type = self.inner.type_signature();
        let read_signature = ReadSignature {
            read: ReadBuilder {
                style: ReadStyle::Prefix,
                operator: self.operator,
            },
            inputs: vec![inner_type],
        };

        match local_index.search_operator(&read_signature) {
            Some(operator_ref) => {
                self.operator_ref.set(operator_ref).unwrap();
                true
            }

            None => {
                errors.err(generate_error(
                    formula_ref,
                    FormulaParsingError::OperatorNotFound,
                ));
                false
            }
        }
    }

    fn finish<'b>(&self) -> FormulaBlock<'b> {
        let inner = self.inner.finish();

        match self.operator_ref.get().unwrap() {
            ReadableBuilder::Symbol(symbol) => {
                let symbol_location = symbol.location();
                let symbol_ref = SymbolBlockRef::new(symbol_location);

                FormulaBlock::Application(
                    Box::new(FormulaBlock::Symbol(symbol_ref)),
                    Box::new(inner),
                )
            }

            ReadableBuilder::Definition(definition) => {
                let definition_location = definition.location();
                let definition_ref = DefinitionBlockRef::new(definition_location);

                FormulaBlock::Definition(definition_ref, vec![inner])
            }
        }
    }

    fn type_signature(&'a self) -> &TypeSignatureBuilder {
        self.operator_ref.get().unwrap().type_signature().applied()
    }

    fn application(
        &'a self,
    ) -> Option<(
        ReadableBuilder,
        Box<dyn ExactSizeIterator<Item = &FormulaBuilder> + '_>,
    )> {
        let readable = *self.operator_ref.get().unwrap();
        let inputs = Box::new(std::iter::once(self.inner.as_ref()));

        Some((readable, inputs))
    }
}

impl<'a> PartialEq for FormulaPrefixBuilder<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.operator_ref.get().unwrap() == other.operator_ref.get().unwrap()
            && self.inner == other.inner
    }
}
impl<'a> Eq for FormulaPrefixBuilder<'a> {}

#[derive(Clone, Debug)]
pub struct FormulaInfixBuilder<'a> {
    operator: ReadOperator,
    lhs: Box<FormulaBuilder<'a>>,
    rhs: Box<FormulaBuilder<'a>>,

    operator_ref: OnceCell<ReadableBuilder<'a>>,
}

impl<'a> FormulaInfixBuilder<'a> {
    fn from_op(operator: ReadOperator, lhs: FormulaBuilder<'a>, rhs: FormulaBuilder<'a>) -> Self {
        FormulaInfixBuilder {
            operator,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),

            operator_ref: OnceCell::new(),
        }
    }

    fn build<F>(
        &'a self,
        formula_ref: &'a FormulaBuilder<'a>,
        local_index: &LocalBuilderIndex<'a, '_>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(&'a FormulaBuilder<'a>, FormulaParsingError) -> ParsingError<'a> + Copy,
    {
        if !self.lhs.build(local_index, errors, generate_error) {
            return false;
        }
        if !self.rhs.build(local_index, errors, generate_error) {
            return false;
        }

        let lhs_type = self.lhs.type_signature();
        let rhs_type = self.rhs.type_signature();
        let read_signature = ReadSignature {
            read: ReadBuilder {
                style: ReadStyle::Infix,
                operator: self.operator,
            },
            inputs: vec![lhs_type, rhs_type],
        };

        match local_index.search_operator(&read_signature) {
            Some(operator_ref) => {
                self.operator_ref.set(operator_ref).unwrap();
                true
            }

            None => {
                errors.err(generate_error(
                    formula_ref,
                    FormulaParsingError::OperatorNotFound,
                ));
                false
            }
        }
    }

    fn finish<'b>(&self) -> FormulaBlock<'b> {
        let lhs = self.lhs.finish();
        let rhs = self.rhs.finish();

        match self.operator_ref.get().unwrap() {
            ReadableBuilder::Symbol(symbol) => {
                let symbol_location = symbol.location();
                let symbol_ref = SymbolBlockRef::new(symbol_location);

                FormulaBlock::Application(
                    Box::new(FormulaBlock::Application(
                        Box::new(FormulaBlock::Symbol(symbol_ref)),
                        Box::new(lhs),
                    )),
                    Box::new(rhs),
                )
            }

            ReadableBuilder::Definition(definition) => {
                let definition_location = definition.location();
                let definition_ref = DefinitionBlockRef::new(definition_location);

                FormulaBlock::Definition(definition_ref, vec![lhs, rhs])
            }
        }
    }

    fn type_signature(&'a self) -> &TypeSignatureBuilder {
        self.operator_ref
            .get()
            .unwrap()
            .type_signature()
            .applied()
            .applied()
    }

    fn binary(&'a self) -> Option<(ReadableBuilder, &FormulaBuilder, &FormulaBuilder)> {
        let readable = *self.operator_ref.get().unwrap();
        let left = &self.lhs;
        let right = &self.rhs;

        Some((readable, left, right))
    }

    fn application(
        &'a self,
    ) -> Option<(
        ReadableBuilder,
        Box<dyn ExactSizeIterator<Item = &FormulaBuilder> + '_>,
    )> {
        let readable = *self.operator_ref.get().unwrap();
        let inputs = Box::new(std::array::IntoIter::new([
            self.lhs.as_ref(),
            self.rhs.as_ref(),
        ]));

        Some((readable, inputs))
    }
}

impl<'a> PartialEq for FormulaInfixBuilder<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.operator_ref.get().unwrap() == other.operator_ref.get().unwrap()
            && self.lhs == other.lhs
            && self.rhs == other.rhs
    }
}
impl<'a> Eq for FormulaInfixBuilder<'a> {}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FormulaReadableApplicationBuilder<'a> {
    readable: ReadableBuilder<'a>,
    inputs: Vec<FormulaBuilder<'a>>,
}

impl<'a> FormulaReadableApplicationBuilder<'a> {
    pub fn new(readable: ReadableBuilder<'a>, inputs: Vec<FormulaBuilder<'a>>) -> Self {
        FormulaReadableApplicationBuilder { readable, inputs }
    }

    pub fn test(&self, other: &FormulaBuilder<'a>) -> bool {
        match other {
            FormulaBuilder::Symbol(_) => todo!(),
            FormulaBuilder::Variable(_) => false,

            FormulaBuilder::Prefix(_) => todo!(),
            FormulaBuilder::Infix(formula) => {
                self.inputs.len() == 2
                    && &self.readable == formula.operator_ref.get().unwrap()
                    && &self.inputs[0] == formula.lhs.as_ref()
                    && &self.inputs[1] == formula.rhs.as_ref()
            }

            FormulaBuilder::ReadableApplication(app) => self == app,
        }
    }

    fn finish<'b>(&self) -> FormulaBlock<'b> {
        match self.readable {
            ReadableBuilder::Symbol(symbol) => {
                let symbol_location = symbol.location();
                let symbol_ref = SymbolBlockRef::new(symbol_location);

                self.inputs
                    .iter()
                    .fold(FormulaBlock::Symbol(symbol_ref), |curr, input| {
                        FormulaBlock::Application(Box::new(curr), Box::new(input.finish()))
                    })
            }

            ReadableBuilder::Definition(definition) => {
                let definition_location = definition.location();
                let definition_ref = DefinitionBlockRef::new(definition_location);

                FormulaBlock::Definition(
                    definition_ref,
                    self.inputs.iter().map(FormulaBuilder::finish).collect(),
                )
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum FormulaBuilder<'a> {
    Symbol(FormulaSymbolBuilder<'a>),
    Variable(FormulaVariableBuilder<'a>),

    Prefix(FormulaPrefixBuilder<'a>),
    Infix(FormulaInfixBuilder<'a>),

    ReadableApplication(FormulaReadableApplicationBuilder<'a>),
}

impl<'a> FormulaBuilder<'a> {
    fn primary(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::ident => FormulaBuilder::Symbol(FormulaSymbolBuilder::from_pest(pair)),
            Rule::var => FormulaBuilder::Variable(FormulaVariableBuilder::from_pest(pair)),

            Rule::primary_paren => FormulaBuilder::from_pest(pair.into_inner().next().unwrap()),

            _ => unreachable!(),
        }
    }

    fn prec_climb(pairs: &mut Pairs<Rule>, curr_prec: usize) -> Self {
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
            let precedence = infix.precedence();

            if precedence < curr_prec {
                break;
            }
            pairs.next();

            let next_prec = if infix.is_left_associative() {
                precedence + 1
            } else {
                precedence
            };

            let rhs = Self::prec_climb(pairs, next_prec);
            primary = FormulaBuilder::Infix(FormulaInfixBuilder::from_op(infix, primary, rhs));
        }

        primary
    }

    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::formula);

        Self::prec_climb(&mut pair.into_inner(), 0)
    }

    fn build<F>(
        &'a self,
        local_index: &LocalBuilderIndex<'a, '_>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(&'a FormulaBuilder<'a>, FormulaParsingError) -> ParsingError<'a> + Copy,
    {
        match self {
            Self::Symbol(_) => todo!(),
            Self::Variable(formula) => formula.build(self, local_index, errors, generate_error),

            Self::Prefix(formula) => formula.build(self, local_index, errors, generate_error),
            Self::Infix(formula) => formula.build(self, local_index, errors, generate_error),

            Self::ReadableApplication(_) => unreachable!(),
        }
    }

    fn type_signature(&'a self) -> &TypeSignatureBuilder {
        match self {
            Self::Symbol(_) => todo!(),
            Self::Variable(formula) => formula.type_signature(),

            Self::Prefix(formula) => formula.type_signature(),
            Self::Infix(formula) => formula.type_signature(),

            Self::ReadableApplication(formula) => todo!(),
        }
    }

    pub fn variable(&'a self) -> Option<&VariableBuilder> {
        match self {
            Self::Variable(formula) => Some(formula.var_ref.get().unwrap()),

            _ => None,
        }
    }

    pub fn binary(&'a self) -> Option<(ReadableBuilder, &FormulaBuilder, &FormulaBuilder)> {
        match self {
            Self::Infix(formula) => formula.binary(),

            _ => todo!(),
        }
    }

    pub fn simple_binary(
        &'a self,
    ) -> Option<(ReadableBuilder, &VariableBuilder, &VariableBuilder)> {
        self.binary().and_then(|(readable_ref, left, right)| {
            Some((readable_ref, left.variable()?, right.variable()?))
        })
    }

    pub fn application(
        &'a self,
    ) -> Option<(
        ReadableBuilder,
        impl ExactSizeIterator<Item = &FormulaBuilder>,
    )> {
        match self {
            Self::Prefix(formula) => formula.application(),
            Self::Infix(formula) => formula.application(),

            _ => todo!(),
        }
    }

    pub fn finish<'b>(&self) -> FormulaBlock<'b> {
        match self {
            Self::Symbol(_) => todo!(),
            Self::Variable(formula) => formula.finish(),

            Self::Prefix(formula) => formula.finish(),
            Self::Infix(formula) => formula.finish(),

            Self::ReadableApplication(formula) => formula.finish(),
        }
    }
}

impl<'a> PartialEq for FormulaBuilder<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Symbol(self_symbol), Self::Symbol(other_symbol)) => self_symbol == other_symbol,
            (Self::Variable(self_variable), Self::Variable(other_variable)) => {
                self_variable == other_variable
            }

            (Self::Prefix(self_prefix), Self::Prefix(other_prefix)) => self_prefix == other_prefix,
            (Self::Infix(self_infix), Self::Infix(other_infix)) => self_infix == other_infix,

            (Self::ReadableApplication(self_app), _) => self_app.test(other),
            (_, Self::ReadableApplication(other_app)) => other_app.test(self),

            _ => false,
        }
    }
}
impl<'a> Eq for FormulaBuilder<'a> {}

#[derive(Debug)]
pub struct DisplayFormulaBuilder<'a> {
    display: MathBuilder,
    formula: FormulaBuilder<'a>,
}

impl<'a> DisplayFormulaBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>) -> Self {
        let display = MathBuilder::from_pest_formula(pair.clone());
        let formula = FormulaBuilder::from_pest(pair);

        DisplayFormulaBuilder { display, formula }
    }

    pub fn verify_structure(&'a self, errors: &mut ParsingErrorContext<'a>) {
        self.display.verify_structure(errors, |_| unreachable!());
    }

    pub fn build<F>(
        &'a self,
        local_index: &LocalBuilderIndex<'a, '_>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(&'a FormulaBuilder<'a>, FormulaParsingError) -> ParsingError<'a> + Copy,
    {
        self.formula.build(local_index, errors, generate_error)
    }

    pub fn finish<'b>(&self) -> DisplayFormulaBlock<'b> {
        let display = self.display.finish();
        let formula = self.formula.finish();

        DisplayFormulaBlock::new(display, formula)
    }

    pub fn formula(&'a self) -> &FormulaBuilder {
        &self.formula
    }

    pub fn display(&self) -> &MathBuilder {
        &self.display
    }

    pub fn type_signature(&'a self) -> &TypeSignatureBuilder {
        self.formula.type_signature()
    }

    pub fn binary(&'a self) -> Option<(ReadableBuilder, &FormulaBuilder, &FormulaBuilder)> {
        self.formula.binary()
    }

    pub fn simple_binary(
        &'a self,
    ) -> Option<(ReadableBuilder, &VariableBuilder, &VariableBuilder)> {
        self.formula.simple_binary()
    }

    pub fn application(
        &'a self,
    ) -> Option<(
        ReadableBuilder,
        impl ExactSizeIterator<Item = &FormulaBuilder>,
    )> {
        self.formula.application()
    }
}
