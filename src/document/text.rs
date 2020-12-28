// Copyright 2020 Alexander Isaacson
//
// This file is part of ckproof.
//
// Ckproof is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// Ckproof is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with ckproof.  If not, see
// <https://www.gnu.org/licenses/>.

use url::Url;

use crate::rendered::{
    DisplayMathRendered, HeadingRendered, MlaContainerRendered, MlaRendered, QuoteRendered,
    SublistItemRendered, TableRendered, TableRenderedRow, TextRendered, TodoRendered,
};

use super::directory::{Block, BlockDirectory};

#[derive(Clone)]
pub enum BareElement {
    Whitespace,
    Ampersand,
    Apostrophe,
    LeftDoubleQuote,
    RightDoubleQuote,
    LeftSingleQuote,
    RightSingleQuote,
    Ellipsis,
    Word(String),
}

impl BareElement {
    fn render(&self) -> &str {
        match self {
            Self::Whitespace => " ",
            Self::Ampersand => "&amp;",
            Self::Apostrophe => "&apos;",
            Self::LeftDoubleQuote => "\u{201C}",
            Self::RightDoubleQuote => "\u{201D}",
            Self::LeftSingleQuote => "\u{2018}",
            Self::RightSingleQuote => "\u{2019}",
            Self::Ellipsis => "\u{2026}",
            Self::Word(w) => &w,
        }
    }
}

#[derive(Clone)]
pub struct BareText {
    elements: Vec<BareElement>,
}

impl BareText {
    pub fn new(elements: Vec<BareElement>) -> BareText {
        BareText { elements }
    }

    fn render(&self) -> String {
        self.elements.iter().map(BareElement::render).collect()
    }
}

pub struct Hyperlink {
    url: Url,
    contents: BareText,
}

impl Hyperlink {
    pub fn new(url: Url, contents: BareText) -> Hyperlink {
        Hyperlink { url, contents }
    }

    fn render(&self) -> String {
        let contents = self.contents.render();

        format!(
            "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
            self.url.as_str(),
            contents
        )
    }
}

pub enum UnformattedElement {
    OpenBracket,
    CloseBracket,

    Whitespace,
    Ampersand,
    Apostrophe,
    LeftDoubleQuote,
    RightDoubleQuote,
    LeftSingleQuote,
    RightSingleQuote,
    Ellipsis,

    Hyperlink(Hyperlink),
    Word(String),
}

impl UnformattedElement {
    fn render(&self) -> String {
        match self {
            Self::OpenBracket => "[".to_owned(),
            Self::CloseBracket => "]".to_owned(),

            Self::Whitespace => " ".to_owned(),
            Self::Ampersand => "&amp;".to_owned(),
            Self::Apostrophe => "&apos;".to_owned(),
            Self::LeftDoubleQuote => "\u{201C}".to_owned(),
            Self::RightDoubleQuote => "\u{201D}".to_owned(),
            Self::LeftSingleQuote => "\u{2018}".to_owned(),
            Self::RightSingleQuote => "\u{2019}".to_owned(),
            Self::Ellipsis => "\u{2026}".to_owned(),

            Self::Hyperlink(hyperlink) => hyperlink.render(),
            Self::Word(w) => w.clone(),
        }
    }
}

pub struct Unformatted {
    elements: Vec<UnformattedElement>,
}

impl Unformatted {
    pub fn new(elements: Vec<UnformattedElement>) -> Unformatted {
        Unformatted { elements }
    }

    fn render(&self) -> String {
        self.elements
            .iter()
            .map(UnformattedElement::render)
            .collect()
    }
}

pub struct MlaContainer {
    container_title: Option<Unformatted>,
    other_contributors: Option<Unformatted>,
    version: Option<Unformatted>,
    number: Option<Unformatted>,
    publisher: Option<Unformatted>,
    publication_date: Option<Unformatted>,
    location: Option<Unformatted>,
}

impl MlaContainer {
    pub fn new(
        container_title: Option<Unformatted>,
        other_contributors: Option<Unformatted>,
        version: Option<Unformatted>,
        number: Option<Unformatted>,
        publisher: Option<Unformatted>,
        publication_date: Option<Unformatted>,
        location: Option<Unformatted>,
    ) -> MlaContainer {
        MlaContainer {
            container_title,
            other_contributors,
            version,
            number,
            publisher,
            publication_date,
            location,
        }
    }

    fn render(&self) -> MlaContainerRendered {
        let container_title = self.container_title.as_ref().map(Unformatted::render);
        let other_contributors = self.other_contributors.as_ref().map(Unformatted::render);
        let version = self.version.as_ref().map(Unformatted::render);
        let number = self.number.as_ref().map(Unformatted::render);
        let publisher = self.publisher.as_ref().map(Unformatted::render);
        let publication_date = self.publication_date.as_ref().map(Unformatted::render);
        let location = self.location.as_ref().map(Unformatted::render);

        MlaContainerRendered::new(
            container_title,
            other_contributors,
            version,
            number,
            publisher,
            publication_date,
            location,
        )
    }
}

pub struct Mla {
    author: Option<Unformatted>,
    title: Unformatted,
    containers: Vec<MlaContainer>,
}

impl Mla {
    pub fn new(
        author: Option<Unformatted>,
        title: Unformatted,
        containers: Vec<MlaContainer>,
    ) -> Mla {
        Mla {
            author,
            title,
            containers,
        }
    }

    fn render(&self) -> MlaRendered {
        let author = self.author.as_ref().map(Unformatted::render);
        let title = self.title.render();
        let containers = self.containers.iter().map(MlaContainer::render).collect();

        MlaRendered::new(author, title, containers)
    }
}

pub struct SublistItem {
    var_id: String,
    replacement: MathBlock,
}

impl SublistItem {
    pub fn new(var_id: String, replacement: MathBlock) -> SublistItem {
        SublistItem {
            var_id,
            replacement,
        }
    }

    fn render(&self) -> SublistItemRendered {
        let var_id = self.var_id.clone();
        let replacement = self.replacement.render();

        SublistItemRendered::new(var_id, replacement)
    }
}

pub struct Sublist {
    items: Vec<SublistItem>,
}

impl Sublist {
    pub fn new(items: Vec<SublistItem>) -> Sublist {
        Sublist { items }
    }

    fn render(&self) -> Vec<SublistItemRendered> {
        self.items.iter().map(SublistItem::render).collect()
    }
}

pub enum MathElement {
    Fenced(MathBlock),

    Operator(String),
    Symbol(String),
    Variable(String),
    Number(String),
}

impl MathElement {
    fn render(&self) -> String {
        match self {
            Self::Fenced(math) => format!(
                "<mo class=\"paren\">(</mo>{}<mo class=\"paren\">)</mo>",
                math.render()
            ),

            Self::Operator(op) => format!("<mo>{}</mo>", op),
            Self::Symbol(s) => format!("<mi>{}</mi>", s),
            Self::Variable(v) => format!("<mo class=\"var\">&apos;</mo><mi>{}</mi>", v),
            Self::Number(n) => format!("<mn>{}</mn>", n),
        }
    }
}

pub struct MathBlock {
    elements: Vec<MathElement>,
}

impl MathBlock {
    pub fn new(elements: Vec<MathElement>) -> MathBlock {
        MathBlock { elements }
    }

    fn render(&self) -> String {
        Some("<mrow>".to_owned())
            .into_iter()
            .chain(self.elements.iter().map(MathElement::render))
            .chain(Some("</mrow>".to_owned()))
            .collect()
    }
}

pub struct DisplayMathBlock {
    math: MathBlock,
    end: String,
}

impl DisplayMathBlock {
    pub fn new(math: MathBlock, end: String) -> DisplayMathBlock {
        DisplayMathBlock { math, end }
    }

    fn render(&self) -> DisplayMathRendered {
        let math = self.math.render();
        let end = self.end.clone();

        DisplayMathRendered::new(math, end)
    }
}

pub enum ParagraphElement {
    Reference(Block),
    InlineMath(MathBlock),

    UnicornVomitBegin,
    UnicornVomitEnd,
    EmBegin,
    EmEnd,

    Unformatted(UnformattedElement),
}

impl ParagraphElement {
    fn render(&self, directory: &BlockDirectory) -> String {
        match self {
            Self::Reference(block) => block.render_ref(directory),
            Self::InlineMath(math) => format!("<math>{}</math>", math.render()),

            Self::UnicornVomitBegin => "<span class=\"unicorn\">\u{1F661}".to_owned(),
            Self::UnicornVomitEnd => "\u{1F662}</span>".to_owned(),
            Self::EmBegin => "<em>".to_owned(),
            Self::EmEnd => "</em>".to_owned(),

            Self::Unformatted(element) => element.render().to_owned(),
        }
    }
}

pub struct Paragraph {
    elements: Vec<ParagraphElement>,
}

impl Paragraph {
    pub fn render(&self, directory: &BlockDirectory) -> String {
        self.elements
            .iter()
            .map(|element| element.render(directory))
            .collect()
    }
}

impl Paragraph {
    pub fn new(elements: Vec<ParagraphElement>) -> Paragraph {
        Paragraph { elements }
    }
}

pub enum Text {
    Mla(Mla),
    Sublist(Sublist),
    DisplayMath(DisplayMathBlock),
    Paragraph(Paragraph),
}

impl Text {
    pub fn render(&self, directory: &BlockDirectory) -> TextRendered {
        match self {
            Self::Mla(mla) => TextRendered::Mla(mla.render()),
            Self::Sublist(sublist) => TextRendered::Sublist(sublist.render()),
            Self::DisplayMath(display_math) => TextRendered::DisplayMath(display_math.render()),
            Self::Paragraph(paragraph) => TextRendered::Paragraph(paragraph.render(directory)),
        }
    }
}

pub struct TableBlockRow {
    cells: Vec<Paragraph>,
}

impl TableBlockRow {
    pub fn new(cells: Vec<Paragraph>) -> TableBlockRow {
        TableBlockRow { cells }
    }

    fn render(&self, directory: &BlockDirectory) -> TableRenderedRow {
        let cells = self
            .cells
            .iter()
            .map(|paragraph| paragraph.render(directory))
            .collect();

        TableRenderedRow::new(cells)
    }
}

pub struct TableBlock {
    head: Option<Vec<TableBlockRow>>,
    body: Option<Vec<TableBlockRow>>,
    foot: Option<Vec<TableBlockRow>>,

    caption: Option<Paragraph>,
}

impl TableBlock {
    pub fn new(
        head: Option<Vec<TableBlockRow>>,
        body: Option<Vec<TableBlockRow>>,
        foot: Option<Vec<TableBlockRow>>,
        caption: Option<Paragraph>,
    ) -> TableBlock {
        TableBlock {
            head,
            body,
            foot,

            caption,
        }
    }

    pub fn render(&self, directory: &BlockDirectory) -> TableRendered {
        let head = self
            .head
            .as_ref()
            .map(|rows| rows.iter().map(|row| row.render(directory)).collect());
        let body = self
            .body
            .as_ref()
            .map(|rows| rows.iter().map(|row| row.render(directory)).collect());
        let foot = self
            .foot
            .as_ref()
            .map(|rows| rows.iter().map(|row| row.render(directory)).collect());

        let caption = self
            .caption
            .as_ref()
            .map(|paragraph| paragraph.render(directory));

        TableRendered::new(head, body, foot, caption)
    }
}

pub struct QuoteBlock {
    original: Option<Unformatted>,
    value: Unformatted,
}

impl QuoteBlock {
    pub fn new(original: Option<Unformatted>, value: Unformatted) -> QuoteBlock {
        QuoteBlock { original, value }
    }

    pub fn render(&self) -> QuoteRendered {
        let original = self.original.as_ref().map(Unformatted::render);
        let value = self.value.render();

        QuoteRendered::new(original, value)
    }
}

#[derive(Clone, Copy)]
pub enum HeadingLevel {
    L1,
    L2,
    L3,
}

impl HeadingLevel {
    fn render(&self) -> usize {
        match self {
            Self::L1 => 1,
            Self::L2 => 2,
            Self::L3 => 3,
        }
    }
}

pub struct SubHeadingBlock {
    level: HeadingLevel,
    contents: Vec<UnformattedElement>,
}

impl SubHeadingBlock {
    pub fn new(level: HeadingLevel, contents: Vec<UnformattedElement>) -> SubHeadingBlock {
        SubHeadingBlock { level, contents }
    }

    fn render(&self) -> HeadingRendered {
        let level = self.level.render();
        let content = self
            .contents
            .iter()
            .map(UnformattedElement::render)
            .collect();

        HeadingRendered::new(level, content)
    }
}

pub struct HeadingBlock {
    subheadings: Vec<SubHeadingBlock>,
}

impl HeadingBlock {
    pub fn new(subheadings: Vec<SubHeadingBlock>) -> HeadingBlock {
        HeadingBlock { subheadings }
    }

    pub fn render(&self) -> Vec<HeadingRendered> {
        self.subheadings
            .iter()
            .map(SubHeadingBlock::render)
            .collect()
    }
}

pub struct TodoBlock {
    elements: Vec<Text>,
}

impl TodoBlock {
    pub fn new(elements: Vec<Text>) -> TodoBlock {
        TodoBlock { elements }
    }

    pub fn render(&self, directory: &BlockDirectory) -> TodoRendered {
        let elements = self
            .elements
            .iter()
            .map(|element| element.render(directory))
            .collect();

        TodoRendered::new(elements)
    }
}

pub struct TextBlock {
    text: Text,
}

impl TextBlock {
    pub fn new(text: Text) -> TextBlock {
        TextBlock { text }
    }

    pub fn render(&self, directory: &BlockDirectory) -> TextRendered {
        self.text.render(directory)
    }
}
