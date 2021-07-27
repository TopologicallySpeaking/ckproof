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

use crate::eprint;

use super::system::{ProofBlock, ProofBlockStep};

pub enum DocumentCheckingError<'a> {
    AssertionMismatch(&'a ProofBlock<'a>),

    DeductableAssertionNotSubstitutable(&'a ProofBlock<'a>, &'a ProofBlockStep<'a>),
}

impl<'a> DocumentCheckingError<'a> {
    fn eprint_assertion_mismatch(proof: &ProofBlock) {
        let last_step = proof.last_step().unwrap();

        let message = format!(
            "The last step of a proof for `{}` does not match the assertion it's meant to prove.",
            proof.theorem_name()
        );

        eprint(&message, last_step.file_location());
    }

    fn eprint_deductable_assertion_not_substitutable(proof: &ProofBlock, step: &ProofBlockStep) {
        let justification = step.justification().deductable().unwrap();

        let message = format!(
            "A step of a proof for `{}` does not match the assertion of `{}`, the {} meant to justify it.",
            proof.theorem_name(),
            justification.name(),
            justification.kind_str()
        );

        eprint(&message, step.file_location());
    }

    fn eprint(&self) {
        match self {
            Self::AssertionMismatch(proof) => Self::eprint_assertion_mismatch(proof),

            Self::DeductableAssertionNotSubstitutable(proof, step) => {
                Self::eprint_deductable_assertion_not_substitutable(proof, step)
            }
        }
    }
}

#[derive(Default)]
pub struct DocumentCheckingErrorContext<'a> {
    errors: Vec<DocumentCheckingError<'a>>,
}

impl<'a> DocumentCheckingErrorContext<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn err<E: Into<DocumentCheckingError<'a>>>(&mut self, e: E) {
        self.errors.push(e.into());
    }

    pub fn error_found(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn eprint(&self) {
        for error in &self.errors {
            error.eprint();
        }

        eprintln!("Checker exited with errors.");
    }
}
