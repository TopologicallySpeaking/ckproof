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

use ckproof::builders::DocumentBuilder;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut builder = DocumentBuilder::from_lib(&args[1]).unwrap();
    let document = builder.build().unwrap();

    let checkable = document.checkable();
    let checking_errors = checkable.check();
    if checking_errors.error_found() {
        todo!("{:#?}", checking_errors)
    }

    let rendered = document.render();
    let out_file = std::fs::File::create(&args[2]).unwrap();
    serde_json::to_writer_pretty(out_file, &rendered).unwrap();
}
