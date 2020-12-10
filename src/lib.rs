// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with this program.  If not, see
// <https://www.gnu.org/licenses/>.

#![deny(clippy::all)]

pub mod builders;
pub mod deduction;
pub mod document;
pub mod rendered;

fn map_ident(ident: &str) -> String {
    match ident {
        "alpha" => "\u{03B1}".to_owned(),
        "beta" => "\u{03B2}".to_owned(),
        "gamma" => "\u{03B3}".to_owned(),
        "delta" => "\u{03B4}".to_owned(),
        "epsilon" => "\u{03B5}".to_owned(),
        "zeta" => "\u{03B6}".to_owned(),
        "eta" => "\u{03B7}".to_owned(),
        "theta" => "\u{03B8}".to_owned(),
        "iota" => "\u{03B9}".to_owned(),
        "kappa" => "\u{03BA}".to_owned(),
        "lambda" => "\u{03BB}".to_owned(),
        "mu" => "\u{03BC}".to_owned(),
        "nu" => "\u{03BD}".to_owned(),
        "xi" => "\u{03BE}".to_owned(),
        "omicron" => "\u{03BF}".to_owned(),
        "pi" => "\u{03C0}".to_owned(),
        "rho" => "\u{03C1}".to_owned(),
        "sigma" => "\u{03C3}".to_owned(),
        "tau" => "\u{03C4}".to_owned(),
        "upsilon" => "\u{03C5}".to_owned(),
        "phi" => "\u{03C6}".to_owned(),
        "chi" => "\u{03C7}".to_owned(),
        "psi" => "\u{03C8}".to_owned(),
        "omega" => "\u{03C9}".to_owned(),

        s => s.to_owned(),
    }
}
