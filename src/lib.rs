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

#![deny(clippy::all)]
#![feature(nll)]
#![feature(once_cell)]

use pest::Span;
use std::path::{Path, PathBuf};

pub mod builders;
pub mod deduction;
pub mod document;
pub mod rendered;

// TODO: Move somewhere more reasonable.
fn map_ident(ident: &str) -> &str {
    match ident {
        "alpha" => "\u{03B1}",
        "beta" => "\u{03B2}",
        "gamma" => "\u{03B3}",
        "delta" => "\u{03B4}",
        "epsilon" => "\u{03B5}",
        "zeta" => "\u{03B6}",
        "eta" => "\u{03B7}",
        "theta" => "\u{03B8}",
        "iota" => "\u{03B9}",
        "kappa" => "\u{03BA}",
        "lambda" => "\u{03BB}",
        "mu" => "\u{03BC}",
        "nu" => "\u{03BD}",
        "xi" => "\u{03BE}",
        "omicron" => "\u{03BF}",
        "pi" => "\u{03C0}",
        "rho" => "\u{03C1}",
        "sigma" => "\u{03C3}",
        "tau" => "\u{03C4}",
        "upsilon" => "\u{03C5}",
        "phi" => "\u{03C6}",
        "chi" => "\u{03C7}",
        "psi" => "\u{03C8}",
        "omega" => "\u{03C9}",

        _ => ident,
    }
}

pub struct FileLocation {
    path: PathBuf,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    preview: Vec<String>,
}

impl FileLocation {
    fn new(path: &Path, span: Span) -> Self {
        let (start_line, start_column) = span.start_pos().line_col();
        let (end_line, end_column) = span.end_pos().line_col();

        let preview = span
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let (before, inside, after) = if start_line == end_line {
                    (
                        &line[..start_column - 1],
                        &line[start_column - 1..end_column - 1],
                        &line[end_column - 1..],
                    )
                } else {
                    if i == 0 {
                        (&line[..start_column - 1], &line[start_column - 1..], "")
                    } else if i == end_line - start_line {
                        (
                            "",
                            &line[start_column - 1..end_column - 1],
                            &line[end_column - 1..],
                        )
                    } else {
                        ("", line, "")
                    }
                };

                format!("{}\u{1B}[91;4m{}\u{1B}[0m{}", before, inside, after)
            })
            .collect();

        FileLocation {
            path: path.to_owned(),
            start_line,
            start_column,
            end_line,
            end_column,
            preview,
        }
    }
}

fn eprint(message: &str, file_location: &FileLocation) {
    eprintln!("\u{1B}[91;1mError\u{1B}[97;1m: {}\u{1B}[0m", message);
    eprintln!(
        "    \u{1B}[94m-->\u{1B}[0m {}:{}:{}",
        file_location.path.to_str().unwrap(),
        file_location.start_line,
        file_location.start_column
    );
    eprintln!("\u{1B}[94m     |\u{1B}[0m");
    for (i, line) in file_location.preview.iter().enumerate() {
        eprint!(
            "\u{1B}[94m{:>4} |\u{1B}[0m {}",
            i + file_location.start_line,
            line
        );
    }
    eprintln!("\u{1B}[94m     |\u{1B}[0m");
    eprintln!();
}
