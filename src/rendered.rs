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

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum DenotedStyle {
    Prefix,
    Infix,
    Suffix,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Denoted {
    style: DenotedStyle,
    example: String,
}

impl Denoted {
    pub fn new(style: DenotedStyle, example: String) -> Denoted {
        Denoted { style, example }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SystemRendered {
    id: String,
    name: String,
    tagline: String,
    description: Vec<TextRendered>,
}

impl SystemRendered {
    pub fn new(
        id: String,
        name: String,
        tagline: String,
        description: Vec<TextRendered>,
    ) -> SystemRendered {
        SystemRendered {
            id,
            name,
            tagline,
            description,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TypeRendered {
    id: String,
    system_id: String,
    name: String,
    system_name: String,
    tagline: String,
    description: Vec<TextRendered>,
}

impl TypeRendered {
    pub fn new(
        id: String,
        system_id: String,
        name: String,
        system_name: String,
        tagline: String,
        description: Vec<TextRendered>,
    ) -> TypeRendered {
        TypeRendered {
            id,
            system_id,
            name,
            system_name,
            tagline,
            description,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SymbolRendered {
    id: String,
    system_id: String,
    name: String,
    system_name: String,
    tagline: String,
    description: Vec<TextRendered>,
    denoted: Option<Denoted>,
    type_signature: String,
}

impl SymbolRendered {
    pub fn new(
        id: String,
        system_id: String,
        name: String,
        system_name: String,
        tagline: String,
        description: Vec<TextRendered>,
        denoted: Option<Denoted>,
        type_signature: String,
    ) -> SymbolRendered {
        SymbolRendered {
            id,
            system_id,
            name,
            system_name,
            tagline,
            description,
            denoted,
            type_signature,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DefinitionRendered {
    id: String,
    system_id: String,
    name: String,
    system_name: String,
    tagline: String,
    description: Vec<TextRendered>,
    denoted: Option<Denoted>,
    type_signature: String,
    expanded: String,
    example: String,
}

impl DefinitionRendered {
    pub fn new(
        id: String,
        system_id: String,
        name: String,
        system_name: String,
        tagline: String,
        description: Vec<TextRendered>,
        denoted: Option<Denoted>,
        type_signature: String,
        expanded: String,
        example: String,
    ) -> DefinitionRendered {
        DefinitionRendered {
            id,
            system_id,
            name,
            system_name,
            tagline,
            description,
            denoted,
            type_signature,
            expanded,
            example,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AxiomRendered {
    id: String,
    system_id: String,
    name: String,
    system_name: String,
    tagline: String,
    description: Vec<TextRendered>,
    premise: Vec<String>,
    assertion: String,
}

impl AxiomRendered {
    pub fn new(
        id: String,
        system_id: String,
        name: String,
        system_name: String,
        tagline: String,
        description: Vec<TextRendered>,
        premise: Vec<String>,
        assertion: String,
    ) -> AxiomRendered {
        AxiomRendered {
            id,
            system_id,
            name,
            system_name,
            tagline,
            description,
            premise,
            assertion,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TheoremRendered {
    id: String,
    system_id: String,
    name: String,
    system_name: String,
    tagline: String,
    description: Vec<TextRendered>,
    premise: Vec<String>,
    assertion: String,
}

impl TheoremRendered {
    pub fn new(
        id: String,
        system_id: String,
        name: String,
        system_name: String,
        tagline: String,
        description: Vec<TextRendered>,
        premise: Vec<String>,
        assertion: String,
    ) -> TheoremRendered {
        TheoremRendered {
            id,
            system_id,
            name,
            system_name,
            tagline,
            description,
            premise,
            assertion,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ProofRenderedJustification {
    SystemChild(String, String),
    Hypothesis(usize),
    Definition,
    Substitution,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProofRenderedStep {
    id: String,
    justification: ProofRenderedJustification,
    formula: String,
    end: String,
    tag: usize,
}

impl ProofRenderedStep {
    pub fn new(
        id: String,
        justification: ProofRenderedJustification,
        formula: String,
        end: String,
        tag: usize,
    ) -> ProofRenderedStep {
        ProofRenderedStep {
            id,
            justification,
            formula,
            end,
            tag,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ProofRenderedElement {
    Text(TextRendered),
    Step(ProofRenderedStep),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProofRendered {
    theorem_name: String,
    elements: Vec<ProofRenderedElement>,
}

impl ProofRendered {
    pub fn new(theorem_name: String, elements: Vec<ProofRenderedElement>) -> ProofRendered {
        ProofRendered {
            theorem_name,
            elements,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TableRenderedRow {
    cells: Vec<String>,
}

impl TableRenderedRow {
    pub fn new(cells: Vec<String>) -> TableRenderedRow {
        TableRenderedRow { cells }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TableRendered {
    head: Option<Vec<TableRenderedRow>>,
    body: Option<Vec<TableRenderedRow>>,
    foot: Option<Vec<TableRenderedRow>>,

    caption: Option<String>,
}

impl TableRendered {
    pub fn new(
        head: Option<Vec<TableRenderedRow>>,
        body: Option<Vec<TableRenderedRow>>,
        foot: Option<Vec<TableRenderedRow>>,
        caption: Option<String>,
    ) -> TableRendered {
        TableRendered {
            head,
            body,
            foot,

            caption,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct QuoteValueRendered {
    quote: String,

    local_bib_ref: usize,
}

impl QuoteValueRendered {
    pub fn new(quote: String, local_bib_ref: usize) -> QuoteValueRendered {
        QuoteValueRendered {
            quote,

            local_bib_ref,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct QuoteRendered {
    original: Option<QuoteValueRendered>,
    value: QuoteValueRendered,
}

impl QuoteRendered {
    pub fn new(original: Option<QuoteValueRendered>, value: QuoteValueRendered) -> QuoteRendered {
        QuoteRendered { original, value }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HeadingRendered {
    level: usize,
    content: String,
}

impl HeadingRendered {
    pub fn new(level: usize, content: String) -> HeadingRendered {
        HeadingRendered { level, content }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TodoRendered {
    elements: Vec<TextRendered>,
}

impl TodoRendered {
    pub fn new(elements: Vec<TextRendered>) -> TodoRendered {
        TodoRendered { elements }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SublistItemRendered {
    var_id: String,
    replacement: String,
}

impl SublistItemRendered {
    pub fn new(var_id: String, replacement: String) -> SublistItemRendered {
        SublistItemRendered {
            var_id,
            replacement,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DisplayMathRendered {
    math: String,
    end: String,
}

impl DisplayMathRendered {
    pub fn new(math: String, end: String) -> DisplayMathRendered {
        DisplayMathRendered { math, end }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MlaContainerRendered {
    container_title: Option<String>,
    other_contributors: Option<String>,
    version: Option<String>,
    number: Option<String>,
    publisher: Option<String>,
    publication_date: Option<String>,
    location: Option<String>,
}

impl MlaContainerRendered {
    pub fn new(
        container_title: Option<String>,
        other_contributors: Option<String>,
        version: Option<String>,
        number: Option<String>,
        publisher: Option<String>,
        publication_date: Option<String>,
        location: Option<String>,
    ) -> MlaContainerRendered {
        MlaContainerRendered {
            container_title,
            other_contributors,
            version,
            number,
            publisher,
            publication_date,
            location,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MlaRendered {
    author: Option<String>,
    title: String,
    containers: Vec<MlaContainerRendered>,
}

impl MlaRendered {
    pub fn new(
        author: Option<String>,
        title: String,
        containers: Vec<MlaContainerRendered>,
    ) -> MlaRendered {
        MlaRendered {
            author,
            title,
            containers,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum TextRendered {
    Mla(MlaRendered),
    Sublist(Vec<SublistItemRendered>),
    DisplayMath(DisplayMathRendered),
    Paragraph(String),
}

#[derive(Deserialize, Serialize, Debug)]
pub enum BlockRendered {
    System(SystemRendered),
    Type(TypeRendered),
    Symbol(SymbolRendered),
    Definition(DefinitionRendered),
    Axiom(AxiomRendered),
    Theorem(TheoremRendered),
    Proof(ProofRendered),
    Table(TableRendered),
    Quote(QuoteRendered),
    Heading(Vec<HeadingRendered>),
    Todo(TodoRendered),
    Text(TextRendered),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PageRendered {
    id: String,
    href: String,
    page_num: usize,
    chapter_num: usize,

    page_name: String,
    chapter_name: String,

    prev_href: String,
    up_href: String,
    next_href: Option<String>,

    blocks: Vec<BlockRendered>,

    local_bibliography: Option<Vec<MlaRendered>>,
}

impl PageRendered {
    pub fn new(
        id: String,
        href: String,
        page_num: usize,
        chapter_num: usize,
        page_name: String,
        chapter_name: String,
        prev_href: String,
        up_href: String,
        next_href: Option<String>,
        blocks: Vec<BlockRendered>,
        local_bibliography: Option<Vec<MlaRendered>>,
    ) -> PageRendered {
        PageRendered {
            id,
            href,
            page_num,
            chapter_num,
            page_name,
            chapter_name,
            prev_href,
            up_href,
            next_href,
            blocks,
            local_bibliography,
        }
    }

    pub fn name(&self) -> &str {
        &self.page_name
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ChapterRendered {
    id: String,
    href: String,
    num: usize,

    name: String,
    tagline: String,

    pages: Vec<PageRendered>,
}

impl ChapterRendered {
    pub fn new(
        id: String,
        href: String,
        num: usize,
        name: String,
        tagline: String,
        pages: Vec<PageRendered>,
    ) -> ChapterRendered {
        ChapterRendered {
            id,
            href,
            num,
            name,
            tagline,
            pages,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BookRendered {
    id: String,
    href: String,
    num: usize,

    name: String,
    tagline: String,

    chapters: Vec<ChapterRendered>,
}

impl BookRendered {
    pub(crate) fn new(
        id: String,
        href: String,
        num: usize,
        name: String,
        tagline: String,
        chapters: Vec<ChapterRendered>,
    ) -> BookRendered {
        BookRendered {
            id,
            href,
            num,

            name,
            tagline,

            chapters,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ManifestRendered {
    books: Vec<BookRendered>,
}

#[derive(Deserialize, Serialize, Debug)]
struct ChapterIndex {
    num: usize,
    pages: HashMap<String, usize>,
}

#[derive(Deserialize, Serialize, Debug)]
struct BookIndex {
    num: usize,
    chapters: HashMap<String, ChapterIndex>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DocumentRendered {
    manifest: ManifestRendered,
    index: HashMap<String, BookIndex>,
}

impl DocumentRendered {
    pub fn new(books: Vec<BookRendered>) -> DocumentRendered {
        let mut index = HashMap::new();
        for (book_num, book) in books.iter().enumerate() {
            let mut chapters = HashMap::new();
            for (chapter_num, chapter) in book.chapters.iter().enumerate() {
                let mut pages = HashMap::new();
                for (page_num, page) in chapter.pages.iter().enumerate() {
                    pages.insert(page.id.clone(), page_num);
                }
                chapters.insert(
                    chapter.id.clone(),
                    ChapterIndex {
                        num: chapter_num,
                        pages,
                    },
                );
            }

            index.insert(
                book.id.clone(),
                BookIndex {
                    num: book_num,
                    chapters,
                },
            );
        }

        DocumentRendered {
            manifest: ManifestRendered { books },
            index,
        }
    }

    pub fn manifest(&self) -> &ManifestRendered {
        &self.manifest
    }

    pub fn get_book(&self, book: &str) -> Option<&BookRendered> {
        let book_index = self.index.get(book)?;
        Some(&self.manifest.books[book_index.num])
    }

    pub fn get_chapter(&self, book: &str, chapter: &str) -> Option<&ChapterRendered> {
        let book_index = self.index.get(book)?;
        let chapter_index = book_index.chapters.get(chapter)?;
        Some(&self.manifest.books[book_index.num].chapters[chapter_index.num])
    }

    pub fn get_page(&self, book: &str, chapter: &str, page: &str) -> Option<&PageRendered> {
        let book_index = self.index.get(book)?;
        let chapter_index = book_index.chapters.get(chapter)?;
        let page_num = *chapter_index.pages.get(page)?;
        Some(&self.manifest.books[book_index.num].chapters[chapter_index.num].pages[page_num])
    }
}
