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
use std::ops::Index;

use super::language::{Formula, Variable};

#[derive(Debug)]
pub struct Substitution<'a> {
    map: HashMap<&'a Variable<'a>, &'a Formula<'a>>,
}

impl<'a> Substitution<'a> {
    pub fn new(template: &Formula<'a>, target: &'a Formula<'a>) -> Option<Self> {
        let mut stack = vec![(template, target)];
        let mut map = HashMap::<&Variable, &Formula>::new();

        while let Some((template, target)) = stack.pop() {
            match template {
                Formula::Symbol(symbol_ref) => {
                    if *symbol_ref != target.symbol()? {
                        return None;
                    }
                }

                Formula::Variable(variable_ref) => match map.entry(*variable_ref) {
                    Entry::Occupied(old_target) => {
                        if !old_target.get().compatible(target) {
                            return None;
                        }
                    }

                    Entry::Vacant(slot) => {
                        slot.insert(target);
                    }
                },

                Formula::Application(template_function, template_input) => {
                    let (target_function, target_input) = target.application()?;

                    stack.push((template_function, target_function));
                    stack.push((template_input, target_input));
                }

                Formula::Definition(definition_ref, inputs) => {
                    let (target_ref, target_inputs) = target.definition()?;

                    if *definition_ref != target_ref {
                        return None;
                    }

                    for next_item in inputs.iter().zip(target_inputs) {
                        stack.push(next_item);
                    }
                }
            }
        }

        Some(Substitution { map })
    }

    fn merge(&self, other: &Self) -> Option<Self> {
        let merge_possible = other.map.iter().all(|(other_var, other_formula)| {
            if let Some(self_formula) = self.map.get(other_var) {
                self_formula.compatible(other_formula)
            } else {
                true
            }
        });

        if merge_possible {
            let mut map = self.map.clone();
            map.extend(&other.map);

            Some(Substitution { map })
        } else {
            None
        }
    }
}

impl<'a> Index<&Variable<'a>> for Substitution<'a> {
    type Output = Formula<'a>;

    fn index(&self, variable_ref: &Variable<'a>) -> &Self::Output {
        self.map[variable_ref]
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

    pub fn find<I>(template: &Formula<'a>, possibilities: I) -> Self
    where
        I: IntoIterator<Item = &'a Formula<'a>>,
    {
        let subs = possibilities
            .into_iter()
            .filter_map(|target| Substitution::new(template, target))
            .collect();

        SubstitutionList { subs }
    }

    pub fn merge(self, other: Self) -> Self {
        // All possible pairs of subs between self and other.
        let comparisons = self.subs.iter().flat_map(|self_sub| {
            other
                .subs
                .iter()
                .map(move |other_sub| (self_sub, other_sub))
        });

        SubstitutionList {
            subs: comparisons
                .filter_map(|(self_sub, other_sub)| self_sub.merge(other_sub))
                .collect(),
        }
    }

    pub fn impossible(&self) -> bool {
        self.subs.is_empty()
    }
}
