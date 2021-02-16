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
use std::iter::FromIterator;
use std::ops::Index;

use super::directory::{CheckableDirectory, VariableRef};
use super::language::Formula;

#[derive(Debug)]
pub struct Substitution<'a> {
    map: HashMap<VariableRef, &'a Formula>,
}

impl<'a> Substitution<'a> {
    pub fn new(
        template: &'a Formula,
        target: &'a Formula,
        directory: &CheckableDirectory,
    ) -> Option<Self> {
        let mut stack = vec![(template, target)];
        let mut map = HashMap::new();

        while let Some((template, target)) = stack.pop() {
            match template {
                Formula::Symbol(symbol_ref) => {
                    if *symbol_ref != target.symbol()? {
                        return None;
                    }
                }

                Formula::Variable(variable_ref) => {
                    if let Some(old_target) = map.get(variable_ref) {
                        if !Formula::compatible(*old_target, target, directory) {
                            return None;
                        }
                    } else {
                        map.insert(*variable_ref, target);
                    }
                }

                Formula::Definition(definition_ref, inputs) => {
                    let (target_ref, target_inputs) = target.definition()?;

                    if *definition_ref != target_ref {
                        return None;
                    }

                    for stack_item in inputs.iter().zip(target_inputs) {
                        stack.push(stack_item)
                    }
                }

                Formula::Application(template_function, template_input) => {
                    let (target_function, target_input) = target.application()?;

                    stack.push((template_function, target_function));
                    stack.push((template_input, target_input));
                }
            }
        }

        Some(Substitution { map })
    }

    fn merge(&self, other: &Self, directory: &CheckableDirectory) -> Option<Self> {
        for (other_var, other_formula) in other.map.iter() {
            if let Some(self_formula) = self.map.get(other_var) {
                if !Formula::compatible(self_formula, other_formula, directory) {
                    return None;
                }
            }
        }

        let mut map = self.map.clone();
        map.extend(other.map.iter());

        Some(Substitution { map })
    }
}

impl<'a> Index<VariableRef> for Substitution<'a> {
    type Output = Formula;

    fn index(&self, variable_ref: VariableRef) -> &Self::Output {
        self.map[&variable_ref]
    }
}

impl<'a> FromIterator<(VariableRef, &'a Formula)> for Substitution<'a> {
    fn from_iter<I: IntoIterator<Item = (VariableRef, &'a Formula)>>(iter: I) -> Self {
        let map = iter.into_iter().collect();

        Substitution { map }
    }
}

#[derive(Debug)]
pub struct SubstitutionList<'a> {
    subs: Vec<Substitution<'a>>,
}

impl<'a> SubstitutionList<'a> {
    pub fn new(substitution: Substitution<'a>) -> Self {
        SubstitutionList {
            subs: vec![substitution],
        }
    }

    pub fn find<I>(template: &'a Formula, possibilities: I, directory: &CheckableDirectory) -> Self
    where
        I: IntoIterator<Item = &'a Formula>,
    {
        let subs = possibilities
            .into_iter()
            .filter_map(|target| Substitution::new(template, target, directory))
            .collect();

        SubstitutionList { subs }
    }

    pub fn merge(self, other: Self, directory: &CheckableDirectory) -> Self {
        let comparisons = self.subs.iter().flat_map(|self_sub| {
            other
                .subs
                .iter()
                .map(move |other_sub| (self_sub, other_sub))
        });

        let subs = comparisons
            .filter_map(|(self_sub, other_sub)| self_sub.merge(other_sub, directory))
            .collect();

        SubstitutionList { subs }
    }

    pub fn impossible(&self) -> bool {
        self.subs.is_empty()
    }
}
