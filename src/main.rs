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

use std::env;

use ckproof::builders::ManifestBuilder;
use ckproof::document::Document;

const RET_BUILDER_ERR: i32 = 1;
const RET_CHECKER_ERR: i32 = 2;

fn get_document(path: &str) -> Result<Document, i32> {
    let builder = ManifestBuilder::from_lib(path);

    builder.build().map_err(|errors| {
        errors.eprint();

        RET_BUILDER_ERR
    })
}

// Using std::process::exit doesn't call destructors. Wrapping the main function like this makes
// sure they're called before the process exits.
fn main_real() -> Result<(), i32> {
    let args: Vec<String> = env::args().collect();

    let document = get_document(&args[1])?;
    document.crosslink();
    document.check().map_err(|errors| {
        errors.eprint();

        RET_CHECKER_ERR
    })?;

    let rendered = document.render();
    let out_file = std::fs::File::create(&args[2]).unwrap();
    serde_json::to_writer_pretty(out_file, &rendered).unwrap();

    Ok(())
}

fn main() {
    if let Err(code) = main_real() {
        std::process::exit(code);
    }
}
