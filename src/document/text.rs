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

use url::Url;

use crate::rendered::{
    DisplayMathRendered, HeadingRendered, ListRendered, MlaContainerRendered, MlaRendered,
    QuoteRendered, QuoteValueRendered, SublistItemRendered, TableRendered, TableRenderedRow,
    TextRendered, TodoRendered,
};

use super::structure::BlockRef;
use super::system::{ProofBlock, ProofBlockStepRef};
use super::Document;

#[derive(Clone, Debug)]
pub enum BareElement {
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

    Word(String),
}

impl BareElement {
    // TODO: Remove.
    pub fn render(&self) -> &str {
        match self {
            Self::OpenBracket => "[",
            Self::CloseBracket => "]",

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

#[derive(Clone, Debug)]
pub struct BareText {
    elements: Vec<BareElement>,
}

impl BareText {
    pub fn new(elements: Vec<BareElement>) -> Self {
        BareText { elements }
    }

    // TODO: Remove.
    pub fn render(&self) -> String {
        self.elements.iter().map(BareElement::render).collect()
    }
}

pub struct Hyperlink {
    url: Url,
    contents: BareText,
}

impl Hyperlink {
    // TODO: Remove.
    fn render(&self) -> String {
        let contents = self.contents.render();

        format!(
            "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
            self.url.as_str(),
            contents
        )
    }
}

impl Hyperlink {
    pub fn new(url: Url, contents: BareText) -> Self {
        Hyperlink { url, contents }
    }
}

pub enum UnformattedElement {
    Hyperlink(Hyperlink),
    BareElement(BareElement),
}

impl UnformattedElement {
    // TODO: Remove.
    fn render(&self) -> String {
        match self {
            Self::Hyperlink(hyperlink) => hyperlink.render(),
            Self::BareElement(element) => element.render().to_owned(),
        }
    }
}

pub struct Unformatted {
    elements: Vec<UnformattedElement>,
}

impl Unformatted {
    pub fn new(elements: Vec<UnformattedElement>) -> Self {
        Unformatted { elements }
    }

    // TODO: Remove.
    fn render(&self) -> String {
        self.elements
            .iter()
            .map(UnformattedElement::render)
            .collect()
    }
}

pub struct RawCitationContainer {
    container_title: Option<Unformatted>,
    other_contributors: Option<Unformatted>,
    version: Option<Unformatted>,
    number: Option<Unformatted>,
    publisher: Option<Unformatted>,
    publication_date: Option<Unformatted>,
    location: Option<Unformatted>,
}

impl RawCitationContainer {
    pub fn new(
        container_title: Option<Unformatted>,
        other_contributors: Option<Unformatted>,
        version: Option<Unformatted>,
        number: Option<Unformatted>,
        publisher: Option<Unformatted>,
        publication_date: Option<Unformatted>,
        location: Option<Unformatted>,
    ) -> Self {
        RawCitationContainer {
            container_title,
            other_contributors,
            version,
            number,
            publisher,
            publication_date,
            location,
        }
    }

    // TODO: Remove.
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

pub struct RawCitation {
    author: Option<Unformatted>,
    title: Unformatted,
    containers: Vec<RawCitationContainer>,
}

impl RawCitation {
    pub fn new(
        author: Option<Unformatted>,
        title: Unformatted,
        containers: Vec<RawCitationContainer>,
    ) -> Self {
        RawCitation {
            author,
            title,
            containers,
        }
    }

    // TODO: Remove.
    pub fn render(&self) -> MlaRendered {
        let author = self.author.as_ref().map(Unformatted::render);
        let title = self.title.render();
        let containers = self
            .containers
            .iter()
            .map(RawCitationContainer::render)
            .collect();

        MlaRendered::new(author, title, containers)
    }
}

impl std::fmt::Debug for RawCitation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub struct SublistItem {
    var_id: String,
    replacement: MathBlock,
}

impl SublistItem {
    pub fn new(var_id: String, replacement: MathBlock) -> Self {
        SublistItem {
            var_id,
            replacement,
        }
    }

    // TODO: Remove.
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
    pub fn new(items: Vec<SublistItem>) -> Self {
        Sublist { items }
    }

    // TODO: Remove.
    fn render(&self) -> Vec<SublistItemRendered> {
        self.items.iter().map(SublistItem::render).collect()
    }
}

pub enum MathElement {
    Fenced(MathBlock),

    SquareRoot(MathBlock),
    Power(MathBlock, MathBlock),

    Operator(String),
    Separator,
    Symbol(String),
    Variable(String),
    Number(String),
}

impl MathElement {
    // TODO: Remove.
    fn render(&self) -> String {
        match self {
            Self::Fenced(math) => format!(
                "<mo class=\"paren\">(</mo>{}<mo class=\"paren\">)</mo>",
                math.render()
            ),

            Self::SquareRoot(math) => format!("<msqrt>{}</msqrt>", math.render()),
            Self::Power(base, exp) => format!(
                "<msup><mrow>{}</mrow><mrow>{}</mrow></msup>",
                base.render(),
                exp.render()
            ),

            Self::Operator(op) => format!("<mo>{}</mo>", op),
            Self::Separator => "<mo class=\"separator\">,</mo>".to_owned(),
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
    pub fn new(elements: Vec<MathElement>) -> Self {
        MathBlock { elements }
    }

    // TODO: Remove.
    pub fn render(&self) -> String {
        std::iter::once("<mrow>".to_owned())
            .chain(self.elements.iter().map(MathElement::render))
            .chain(std::iter::once("</mrow>".to_owned()))
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

    // TODO: Remove.
    fn render(&self) -> DisplayMathRendered {
        let math = self.math.render();
        let end = self.end.clone();

        DisplayMathRendered::new(math, end)
    }
}

pub enum ParagraphElement<'a> {
    Reference(Option<BareText>, BlockRef<'a>),
    Tag(Option<BareText>, ProofBlockStepRef<'a>),
    InlineMath(MathBlock),
    Citation(usize),

    UnicornVomitBegin,
    UnicornVomitEnd,
    EmBegin,
    EmEnd,

    Unformatted(UnformattedElement),
}

impl<'a> ParagraphElement<'a> {
    fn crosslink(&'a self, document: &'a Document<'a>) {
        match self {
            Self::Reference(_, r) => r.crosslink(document),
            Self::Tag(_, _) => unreachable!(),

            _ => {}
        }
    }

    fn crosslink_proof(&'a self, document: &'a Document<'a>, proof_ref: &'a ProofBlock<'a>) {
        match self {
            Self::Reference(_, r) => r.crosslink(document),
            Self::Tag(_, r) => r.crosslink(proof_ref),

            _ => {}
        }
    }

    // TODO: Remove.
    fn render(&self) -> String {
        match self {
            Self::Reference(text, block_ref) => block_ref.render(text.as_ref()),
            Self::Tag(text, step_ref) => step_ref.render(text.as_ref()),
            Self::InlineMath(math) => format!("<math>{}</math>", math.render()),
            Self::Citation(citation) => format!(
                "<a href=\"#ref{0}\" class=\"reference\">[{0}]</a>",
                citation + 1
            ),

            Self::UnicornVomitBegin => "<span class=\"unicorn\">\u{1F661}".to_owned(),
            Self::UnicornVomitEnd => "\u{1F662}</span>".to_owned(),
            Self::EmBegin => "<em>".to_owned(),
            Self::EmEnd => "</em>".to_owned(),

            Self::Unformatted(element) => element.render().to_owned(),
        }
    }
}

pub struct Paragraph<'a> {
    elements: Vec<ParagraphElement<'a>>,
}

impl<'a> Paragraph<'a> {
    pub fn new(elements: Vec<ParagraphElement<'a>>) -> Self {
        Paragraph { elements }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        for element in &self.elements {
            element.crosslink(document);
        }
    }

    pub fn crosslink_proof(&'a self, document: &'a Document<'a>, proof_ref: &'a ProofBlock<'a>) {
        for element in &self.elements {
            element.crosslink_proof(document, proof_ref);
        }
    }

    // TODO: Remove
    pub fn render(&self) -> String {
        self.elements.iter().map(ParagraphElement::render).collect()
    }
}

pub enum Text<'a> {
    RawCitation(RawCitation),
    Sublist(Sublist),
    DisplayMath(DisplayMathBlock),
    Paragraph(Paragraph<'a>),
}

impl<'a> Text<'a> {
    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        if let Self::Paragraph(paragraph) = self {
            paragraph.crosslink(document);
        }
    }

    pub fn crosslink_proof(&'a self, document: &'a Document<'a>, proof_ref: &'a ProofBlock<'a>) {
        if let Self::Paragraph(paragraph) = self {
            paragraph.crosslink_proof(document, proof_ref);
        }
    }

    // TODO: Remove.
    pub fn render(&self) -> TextRendered {
        match self {
            Self::RawCitation(citation) => TextRendered::Mla(citation.render()),
            Self::Sublist(sublist) => TextRendered::Sublist(sublist.render()),
            Self::DisplayMath(display_math) => TextRendered::DisplayMath(display_math.render()),
            Self::Paragraph(paragraph) => TextRendered::Paragraph(paragraph.render()),
        }
    }
}

pub struct ListBlock<'a> {
    ordered: bool,
    items: Vec<Paragraph<'a>>,
}

impl<'a> ListBlock<'a> {
    pub fn new(ordered: bool, items: Vec<Paragraph<'a>>) -> Self {
        ListBlock { ordered, items }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        for item in &self.items {
            item.crosslink(document);
        }
    }

    // TODO: Remove.
    pub fn render(&self) -> ListRendered {
        let ordered = self.ordered;
        let items = self.items.iter().map(Paragraph::render).collect();

        ListRendered::new(ordered, items)
    }
}

pub struct TableBlockRow<'a> {
    cells: Vec<Paragraph<'a>>,
}

impl<'a> TableBlockRow<'a> {
    pub fn new(cells: Vec<Paragraph<'a>>) -> Self {
        TableBlockRow { cells }
    }

    fn crosslink(&'a self, document: &'a Document<'a>) {
        for cell in &self.cells {
            cell.crosslink(document);
        }
    }

    // TODO: Remove.
    fn render(&self) -> TableRenderedRow {
        let cells = self.cells.iter().map(Paragraph::render).collect();

        TableRenderedRow::new(cells)
    }
}

pub struct TableBlock<'a> {
    head: Option<Vec<TableBlockRow<'a>>>,
    body: Option<Vec<TableBlockRow<'a>>>,
    foot: Option<Vec<TableBlockRow<'a>>>,

    caption: Option<Paragraph<'a>>,
}

impl<'a> TableBlock<'a> {
    pub fn new(
        head: Option<Vec<TableBlockRow<'a>>>,
        body: Option<Vec<TableBlockRow<'a>>>,
        foot: Option<Vec<TableBlockRow<'a>>>,

        caption: Option<Paragraph<'a>>,
    ) -> Self {
        TableBlock {
            head,
            body,
            foot,

            caption,
        }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        let rows = self
            .head
            .iter()
            .chain(&self.body)
            .chain(&self.foot)
            .flatten();

        for row in rows {
            row.crosslink(document);
        }

        if let Some(caption) = &self.caption {
            caption.crosslink(document);
        }
    }

    // TODO: Remove.
    pub fn render(&self) -> TableRendered {
        let head = self
            .head
            .as_ref()
            .map(|rows| rows.iter().map(TableBlockRow::render).collect());
        let body = self
            .body
            .as_ref()
            .map(|rows| rows.iter().map(TableBlockRow::render).collect());
        let foot = self
            .foot
            .as_ref()
            .map(|rows| rows.iter().map(TableBlockRow::render).collect());

        let caption = self.caption.as_ref().map(Paragraph::render);

        TableRendered::new(head, body, foot, caption)
    }
}

pub struct QuoteValue {
    quote: Unformatted,

    local_bib_ref: usize,
}

impl QuoteValue {
    pub fn new(quote: Unformatted, local_bib_ref: usize) -> Self {
        QuoteValue {
            quote,
            local_bib_ref,
        }
    }

    // TODO: Remove.
    fn render(&self) -> QuoteValueRendered {
        let quote = self.quote.render();
        let local_bib_ref = self.local_bib_ref;

        QuoteValueRendered::new(quote, local_bib_ref)
    }
}

pub struct QuoteBlock {
    original: Option<QuoteValue>,
    value: QuoteValue,
}

impl QuoteBlock {
    pub fn new(original: Option<QuoteValue>, value: QuoteValue) -> Self {
        QuoteBlock { original, value }
    }

    // TODO: Remove.
    pub fn render(&self) -> QuoteRendered {
        let original = self.original.as_ref().map(QuoteValue::render);
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
    // TODO: Remove.
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

    // TODO: Remove.
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

    // TODO: Remove.
    pub fn render(&self) -> Vec<HeadingRendered> {
        self.subheadings
            .iter()
            .map(SubHeadingBlock::render)
            .collect()
    }
}

pub struct TodoBlock<'a> {
    elements: Vec<Text<'a>>,
}

impl<'a> TodoBlock<'a> {
    pub fn new(elements: Vec<Text<'a>>) -> Self {
        TodoBlock { elements }
    }

    pub fn crosslink(&'a self, document: &'a Document<'a>) {
        for element in &self.elements {
            element.crosslink(document);
        }
    }

    // TODO: Remove.
    pub fn render(&self) -> TodoRendered {
        let elements = self.elements.iter().map(Text::render).collect();

        TodoRendered::new(elements)
    }
}
