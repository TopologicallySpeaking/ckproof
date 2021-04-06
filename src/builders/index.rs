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

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use super::bibliography::BibliographyBuilderEntry;
use super::errors::{
    BibliographyParsingError, ParsingError, ParsingErrorContext, ReadableParsingError,
    SystemChildParsingError, SystemParsingError,
};
use super::language::{ReadSignature, ReadableBuilder, VariableBuilder};
use super::system::{SystemBuilder, SystemBuilderChild};

struct SystemBuilderIndex<'a> {
    system_ref: &'a SystemBuilder<'a>,

    children: HashMap<&'a str, SystemBuilderChild<'a>>,
    operators: HashMap<ReadSignature<'a>, ReadableBuilder<'a>>,
}

impl<'a> SystemBuilderIndex<'a> {
    fn new(system_ref: &'a SystemBuilder<'a>) -> Self {
        SystemBuilderIndex {
            system_ref,

            children: HashMap::new(),
            operators: HashMap::new(),
        }
    }

    fn add_child(
        &mut self,
        child_ref: SystemBuilderChild<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self.children.entry(child_ref.id()) {
            Entry::Occupied(old_child_ref) => {
                errors.err(ParsingError::SystemChildError(
                    child_ref,
                    SystemChildParsingError::IdAlreadyTaken(*old_child_ref.get()),
                ));
            }

            Entry::Vacant(slot) => {
                slot.insert(child_ref);
            }
        }
    }

    fn search_child(&self, child_id: &str) -> Option<SystemBuilderChild<'a>> {
        self.children.get(child_id).copied()
    }

    fn add_operator(
        &mut self,
        read_signature: ReadSignature<'a>,
        readable: ReadableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self.operators.entry(read_signature) {
            Entry::Occupied(old_readable) => {
                errors.err(ParsingError::ReadableError(
                    readable,
                    ReadableParsingError::IdAlreadyTaken(*old_readable.get()),
                ));
            }

            Entry::Vacant(slot) => {
                slot.insert(readable);
            }
        }
    }

    pub fn search_operator(
        &self,
        read_signature: &ReadSignature<'a>,
    ) -> Option<ReadableBuilder<'a>> {
        self.operators.get(read_signature).copied()
    }
}

#[derive(Default)]
pub struct BuilderIndex<'a> {
    systems: HashMap<&'a str, SystemBuilderIndex<'a>>,
    bib_refs: HashMap<&'a str, &'a BibliographyBuilderEntry>,
}

impl<'a> BuilderIndex<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_system(
        &mut self,
        system_ref: &'a SystemBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self.systems.entry(system_ref.id()) {
            Entry::Occupied(old_index) => {
                errors.err(ParsingError::SystemError(
                    system_ref,
                    SystemParsingError::IdAlreadyTaken((*old_index.get()).system_ref),
                ));

                return;
            }

            Entry::Vacant(slot) => {
                slot.insert(SystemBuilderIndex::new(system_ref));
            }
        }
    }

    pub fn search_system(&self, system_id: &str) -> Option<&'a SystemBuilder<'a>> {
        self.systems
            .get(system_id)
            .map(|system_index| system_index.system_ref)
    }

    pub fn add_system_child(
        &mut self,
        child_ref: SystemBuilderChild<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self.systems.get_mut(child_ref.system_id()) {
            Some(index) => {
                child_ref.set_system_ref(index.system_ref);
                index.add_child(child_ref, errors);
            }
            None => errors.err(ParsingError::SystemChildError(
                child_ref,
                SystemChildParsingError::ParentNotFound,
            )),
        }
    }

    pub fn search_system_child(
        &self,
        system_id: &str,
        child_id: &str,
    ) -> Option<SystemBuilderChild<'a>> {
        self.systems
            .get(system_id)
            .and_then(|system_index| system_index.search_child(child_id))
    }

    pub fn add_bib_ref(
        &mut self,
        bib_ref: &'a BibliographyBuilderEntry,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        match self.bib_refs.entry(bib_ref.id()) {
            Entry::Occupied(old_ref) => errors.err(ParsingError::BibliographyError(
                bib_ref,
                BibliographyParsingError::KeyAlreadyTaken(*old_ref.get()),
            )),

            Entry::Vacant(slot) => {
                slot.insert(bib_ref);
            }
        }
    }

    pub fn search_bib_ref(&self, bib_key: &str) -> Option<&'a BibliographyBuilderEntry> {
        self.bib_refs.get(bib_key).copied()
    }

    pub fn add_operator(
        &mut self,
        read_signature: ReadSignature<'a>,
        readable: ReadableBuilder<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.systems
            .get_mut(readable.system_id())
            .unwrap()
            .add_operator(read_signature, readable, errors);
    }

    pub fn get_local<'b>(
        &'b self,
        system_id: &str,
        vars: &'a [VariableBuilder<'a>],
    ) -> LocalBuilderIndex<'a, 'b> {
        let parent_system = self.systems.get(system_id).unwrap();
        let vars = vars.iter().map(|var| (var.id(), var)).collect();

        LocalBuilderIndex {
            parent_system,

            vars,
        }
    }
}

pub struct LocalBuilderIndex<'a, 'b> {
    parent_system: &'b SystemBuilderIndex<'a>,

    vars: HashMap<&'a str, &'a VariableBuilder<'a>>,
}

impl<'a, 'b> LocalBuilderIndex<'a, 'b> {
    pub fn search_variable(&'b self, id: &str) -> Option<&'a VariableBuilder<'a>> {
        self.vars.get(id).copied()
    }

    pub fn search_operator(
        &self,
        read_signature: &ReadSignature<'a>,
    ) -> Option<ReadableBuilder<'a>> {
        self.parent_system.search_operator(read_signature)
    }
}
