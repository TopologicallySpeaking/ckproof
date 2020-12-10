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

use std::cell::Cell;

use pest::iterators::{Pair, Pairs};

use crate::map_ident;

use crate::document::directory::Block;
use crate::document::text::{
    DisplayMathBlock, HeadingBlock, HeadingLevel, MathBlock, MathElement, Mla, MlaContainer,
    Paragraph, ParagraphElement, SubHeadingBlock, Sublist, SublistItem, TableBlock, TableBlockRow,
    Text, TextBlock, TodoBlock, Unformatted, UnformattedElement,
};

use super::directory::{
    BuilderDirectory, ProofBuilderStepRef, SystemBuilderChild, SystemBuilderRef, TagIndex,
};
use super::errors::ParsingErrorContext;
use super::Rule;

fn map_operator(operator: Rule) -> String {
    match operator {
        Rule::operator_negation => "\u{00AC}".to_owned(),
        Rule::operator_implies => "\u{21D2}".to_owned(),

        Rule::operator_bang => "!".to_owned(),

        _ => unreachable!(),
    }
}

impl UnformattedElement {
    fn from_pest(pair: Pair<Rule>, whitespace_rule: Rule) -> UnformattedElement {
        match pair.as_rule() {
            rule if rule == whitespace_rule => Self::Whitespace,

            Rule::amp => Self::Ampersand,
            Rule::apos => Self::Apostrophe,
            Rule::ldquo => Self::LeftDoubleQuote,
            Rule::rdquo => Self::RightDoubleQuote,
            Rule::lsquo => Self::LeftSingleQuote,
            Rule::rsquo => Self::RightSingleQuote,
            Rule::ellipsis => Self::Ellipsis,
            Rule::word => Self::Word(pair.as_str().to_owned()),

            _ => unreachable!("{:#?}", pair.as_span().start_pos().line_col()),
        }
    }
}

impl Unformatted {
    fn from_pest(pair: Pair<Rule>) -> Unformatted {
        assert_eq!(pair.as_rule(), Rule::unformatted);

        let elements = pair
            .into_inner()
            .map(|pair| UnformattedElement::from_pest(pair, Rule::oneline_whitespace))
            .collect();

        Unformatted::new(elements)
    }
}

pub struct MlaContainerBuilder {
    container_titles: Vec<Unformatted>,
    other_contributors: Vec<Unformatted>,
    versions: Vec<Unformatted>,
    numbers: Vec<Unformatted>,
    publishers: Vec<Unformatted>,
    publication_dates: Vec<Unformatted>,
    locations: Vec<Unformatted>,

    verified: Cell<bool>,
}

impl MlaContainerBuilder {
    fn from_pest(pairs: Pairs<Rule>) -> MlaContainerBuilder {
        let mut container_titles = Vec::with_capacity(1);
        let mut other_contributors = Vec::with_capacity(1);
        let mut versions = Vec::with_capacity(1);
        let mut numbers = Vec::with_capacity(1);
        let mut publishers = Vec::with_capacity(1);
        let mut publication_dates = Vec::with_capacity(1);
        let mut locations = Vec::with_capacity(1);

        for pair in pairs {
            match pair.as_rule() {
                Rule::mla_container_title => {
                    let container_title = Unformatted::from_pest(pair.into_inner().next().unwrap());

                    container_titles.push(container_title);
                }

                Rule::mla_other_contributors => {
                    let other_contributor =
                        Unformatted::from_pest(pair.into_inner().next().unwrap());

                    other_contributors.push(other_contributor);
                }

                Rule::mla_version => {
                    let version = Unformatted::from_pest(pair.into_inner().next().unwrap());

                    versions.push(version);
                }

                Rule::mla_number => {
                    let number = Unformatted::from_pest(pair.into_inner().next().unwrap());

                    numbers.push(number);
                }

                Rule::mla_publisher => {
                    let publisher = Unformatted::from_pest(pair.into_inner().next().unwrap());

                    publishers.push(publisher);
                }

                Rule::mla_publication_date => {
                    let publication_date =
                        Unformatted::from_pest(pair.into_inner().next().unwrap());

                    publication_dates.push(publication_date);
                }

                Rule::mla_location => {
                    let location = Unformatted::from_pest(pair.into_inner().next().unwrap());

                    locations.push(location);
                }

                _ => unreachable!(),
            }
        }

        MlaContainerBuilder {
            container_titles,
            other_contributors,
            versions,
            numbers,
            publishers,
            publication_dates,
            locations,

            verified: Cell::new(false),
        }
    }

    fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        assert!(!self.verified.get());
        let mut found_error = false;

        if self.container_titles.len() > 1 {
            todo!()
        }

        if self.other_contributors.len() > 1 {
            todo!()
        }

        if self.versions.len() > 1 {
            todo!()
        }

        if self.numbers.len() > 1 {
            todo!()
        }

        if self.publishers.len() > 1 {
            todo!()
        }

        if self.publication_dates.len() > 1 {
            todo!()
        }

        if self.locations.len() > 1 {
            todo!()
        }

        self.verified.set(!found_error);
    }

    fn finish(&self) -> MlaContainer {
        assert!(self.verified.get());

        let container_title = self.container_titles.get(0).cloned();
        let other_contributors = self.other_contributors.get(0).cloned();
        let version = self.versions.get(0).cloned();
        let number = self.numbers.get(0).cloned();
        let publisher = self.publishers.get(0).cloned();
        let publication_date = self.publication_dates.get(0).cloned();
        let location = self.locations.get(0).cloned();

        MlaContainer::new(
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

struct MlaBuilderEntries {
    authors: Vec<Unformatted>,
    titles: Vec<Unformatted>,
    containers: Vec<MlaContainerBuilder>,

    verified: Cell<bool>,
}

impl MlaBuilderEntries {
    fn from_pest(pairs: Pairs<Rule>) -> MlaBuilderEntries {
        let mut authors = Vec::with_capacity(1);
        let mut titles = Vec::with_capacity(1);
        let mut containers = Vec::new();

        for pair in pairs {
            match pair.as_rule() {
                Rule::mla_authors => {
                    authors.push(Unformatted::from_pest(pair.into_inner().next().unwrap()))
                }
                Rule::mla_title => {
                    titles.push(Unformatted::from_pest(pair.into_inner().next().unwrap()))
                }
                Rule::mla_container => {
                    containers.push(MlaContainerBuilder::from_pest(pair.into_inner()))
                }

                _ => unreachable!(),
            }
        }

        MlaBuilderEntries {
            authors,
            titles,
            containers,

            verified: Cell::new(false),
        }
    }

    fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        assert!(!self.verified.get());
        let found_error = false;

        if self.authors.len() > 1 {
            todo!()
        }

        match self.titles.len() {
            0 => todo!(),
            1 => {}
            _ => todo!(),
        }

        for container in &self.containers {
            container.verify_structure(errors);
        }

        self.verified.set(!found_error);
    }

    fn author(&self) -> Option<&Unformatted> {
        assert!(self.verified.get());
        self.authors.get(0)
    }

    fn title(&self) -> &Unformatted {
        assert!(self.verified.get());
        &self.titles[0]
    }
}

pub struct MlaBuilder {
    entries: MlaBuilderEntries,
}

impl MlaBuilder {
    fn from_pest(pair: Pair<Rule>) -> MlaBuilder {
        let entries = MlaBuilderEntries::from_pest(pair.into_inner());

        MlaBuilder { entries }
    }

    fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        self.entries.verify_structure(errors);
    }

    fn finish(&self) -> Mla {
        let author = self.entries.author().cloned();
        let title = self.entries.title().clone();
        let containers = self
            .entries
            .containers
            .iter()
            .map(MlaContainerBuilder::finish)
            .collect();

        Mla::new(author, title, containers)
    }
}

struct SublistBuilderItem {
    var_id: String,
    replacement: MathBuilder,
}

impl SublistBuilderItem {
    fn from_pest(pair: Pair<Rule>) -> SublistBuilderItem {
        assert_eq!(pair.as_rule(), Rule::sublist_item);

        let mut inner = pair.into_inner();
        let var = inner.next().unwrap();
        let var_inner = var.into_inner().next().unwrap();
        let var_id = map_ident(var_inner.as_str());
        let replacement = MathBuilder::from_pest(inner.next().unwrap());

        SublistBuilderItem {
            var_id,
            replacement,
        }
    }

    fn finish(&self) -> SublistItem {
        let var_id = self.var_id.clone();
        let replacement = self.replacement.finish();

        SublistItem::new(var_id, replacement)
    }
}

pub struct SublistBuilder {
    items: Vec<SublistBuilderItem>,
}

impl SublistBuilder {
    fn from_pest(pair: Pair<Rule>) -> SublistBuilder {
        assert_eq!(pair.as_rule(), Rule::sublist);

        let items = pair
            .into_inner()
            .map(SublistBuilderItem::from_pest)
            .collect();

        SublistBuilder { items }
    }

    fn finish(&self) -> Sublist {
        let items = self.items.iter().map(SublistBuilderItem::finish).collect();

        Sublist::new(items)
    }
}

pub enum MathBuilderElement {
    Fenced(MathBuilder),

    Operator(String),
    Symbol(String),
    Variable(String),
}

impl MathBuilderElement {
    fn from_pest(pair: Pair<Rule>) -> MathBuilderElement {
        match pair.as_rule() {
            Rule::math_row => Self::Fenced(MathBuilder::from_pest(pair)),

            Rule::display_operator => {
                Self::Operator(map_operator(pair.into_inner().next().unwrap().as_rule()))
            }
            Rule::ident => Self::Symbol(map_ident(pair.as_str())),
            Rule::var => Self::Variable(map_ident(pair.into_inner().next().unwrap().as_str())),

            _ => unreachable!(),
        }
    }

    fn finish(&self) -> MathElement {
        match self {
            Self::Fenced(builder) => MathElement::Fenced(builder.finish()),

            Self::Operator(op) => MathElement::Operator(op.clone()),
            Self::Symbol(s) => MathElement::Symbol(s.clone()),
            Self::Variable(v) => MathElement::Variable(v.clone()),
        }
    }
}

pub struct MathBuilder {
    elements: Vec<MathBuilderElement>,
}

impl MathBuilder {
    fn from_pest(pair: Pair<Rule>) -> MathBuilder {
        assert_eq!(pair.as_rule(), Rule::math_row);

        let elements = pair
            .into_inner()
            .map(MathBuilderElement::from_pest)
            .collect();

        MathBuilder { elements }
    }

    fn finish(&self) -> MathBlock {
        let elements = self
            .elements
            .iter()
            .map(MathBuilderElement::finish)
            .collect();

        MathBlock::new(elements)
    }
}

pub struct DisplayMathBuilder {
    math: MathBuilder,
    end: String,
}

impl DisplayMathBuilder {
    fn from_pest(pair: Pair<Rule>) -> DisplayMathBuilder {
        assert_eq!(pair.as_rule(), Rule::display_math);

        let mut inner = pair.into_inner();
        let math = MathBuilder::from_pest(inner.next().unwrap());
        let end_container = inner.next().unwrap();
        let end = end_container
            .into_inner()
            .next()
            .unwrap()
            .as_str()
            .to_owned();

        DisplayMathBuilder { math, end }
    }

    fn finish(&self) -> DisplayMathBlock {
        let math = self.math.finish();
        let end = self.end.clone();

        DisplayMathBlock::new(math, end)
    }
}

pub enum TextBuilder {
    Mla(MlaBuilder),
    Sublist(SublistBuilder),
    DisplayMath(DisplayMathBuilder),
    Paragraph(ParagraphBuilder),
}

impl TextBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> TextBuilder {
        assert_eq!(pair.as_rule(), Rule::text_block);
        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::mla => Self::Mla(MlaBuilder::from_pest(pair)),
            Rule::sublist => Self::Sublist(SublistBuilder::from_pest(pair)),
            Rule::display_math => Self::DisplayMath(DisplayMathBuilder::from_pest(pair)),
            Rule::paragraph => Self::Paragraph(ParagraphBuilder::from_pest(pair)),

            _ => unreachable!(),
        }
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        match self {
            Self::Mla(mla) => mla.verify_structure(errors),
            Self::Paragraph(paragraph) => paragraph.verify_structure(directory, errors),

            Self::Sublist(_) | Self::DisplayMath(_) => {}
        }
    }

    pub fn verify_structure_with_tags(
        &self,
        directory: &BuilderDirectory,
        tags: &TagIndex,
        errors: &mut ParsingErrorContext,
    ) {
        match self {
            Self::Mla(mla) => mla.verify_structure(errors),
            Self::Paragraph(paragraph) => {
                paragraph.verify_structure_with_tags(directory, tags, errors)
            }

            Self::Sublist(_) | Self::DisplayMath(_) => {}
        }
    }

    pub fn finish(&self) -> Text {
        match self {
            Self::Mla(mla) => Text::Mla(mla.finish()),
            Self::Sublist(sublist) => Text::Sublist(sublist.finish()),
            Self::DisplayMath(display_math) => Text::DisplayMath(display_math.finish()),
            Self::Paragraph(paragraph) => Text::Paragraph(paragraph.finish()),
        }
    }
}

struct SystemReferenceBuilder {
    id: String,

    system_ref: Cell<Option<SystemBuilderRef>>,
}

impl SystemReferenceBuilder {
    fn from_pest(pair: Pair<Rule>) -> SystemReferenceBuilder {
        assert_eq!(pair.as_rule(), Rule::ident);

        let id = pair.as_str().to_owned();

        SystemReferenceBuilder {
            id,

            system_ref: Cell::new(None),
        }
    }

    fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        self.system_ref.set(directory.search_system(&self.id));

        if self.system_ref.get().is_none() {
            todo!()
        }
    }

    fn finish(&self) -> Block {
        self.system_ref.get().unwrap().finish().into()
    }
}

struct SystemChildReferenceBuilder {
    system_id: String,
    child_id: String,

    child_ref: Cell<Option<SystemBuilderChild>>,
}

impl SystemChildReferenceBuilder {
    fn from_pest(pair: Pair<Rule>) -> SystemChildReferenceBuilder {
        assert_eq!(pair.as_rule(), Rule::fqid);

        let mut inner = pair.into_inner();
        let system_id = inner.next().unwrap().as_str().to_owned();
        let child_id = inner.next().unwrap().as_str().to_owned();

        SystemChildReferenceBuilder {
            system_id,
            child_id,

            child_ref: Cell::new(None),
        }
    }

    fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        self.child_ref
            .set(directory.search_system_child(&self.system_id, &self.child_id));

        if self.child_ref.get().is_none() {
            todo!()
        }
    }

    fn finish(&self) -> Block {
        self.child_ref.get().unwrap().finish()
    }
}

struct TagReferenceBuilder {
    tag: String,

    step_ref: Cell<Option<ProofBuilderStepRef>>,
}

impl TagReferenceBuilder {
    fn from_pest(pair: Pair<Rule>) -> TagReferenceBuilder {
        assert_eq!(pair.as_rule(), Rule::tag);

        let tag = pair.into_inner().next().unwrap().as_str().to_owned();

        TagReferenceBuilder {
            tag,

            step_ref: Cell::new(None),
        }
    }

    fn verify_structure(&self, tags: &TagIndex, errors: &mut ParsingErrorContext) {
        self.step_ref.set(tags.search(&self.tag));

        if self.step_ref.get().is_none() {
            todo!()
        }
    }

    fn finish(&self) -> Block {
        self.step_ref.get().unwrap().finish().into()
    }
}

enum ReferenceBuilder {
    System(SystemReferenceBuilder),
    SystemChild(SystemChildReferenceBuilder),
    Tag(TagReferenceBuilder),
}

impl ReferenceBuilder {
    fn from_pest(pair: Pair<Rule>) -> ReferenceBuilder {
        assert_eq!(pair.as_rule(), Rule::text_reference);
        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::ident => Self::System(SystemReferenceBuilder::from_pest(pair)),
            Rule::fqid => Self::SystemChild(SystemChildReferenceBuilder::from_pest(pair)),
            Rule::tag => Self::Tag(TagReferenceBuilder::from_pest(pair)),

            _ => unreachable!(),
        }
    }

    fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        match self {
            Self::System(r) => r.verify_structure(directory, errors),
            Self::SystemChild(r) => r.verify_structure(directory, errors),
            Self::Tag(_) => unreachable!(),
        }
    }

    fn verify_structure_with_tags(
        &self,
        directory: &BuilderDirectory,
        tags: &TagIndex,
        errors: &mut ParsingErrorContext,
    ) {
        match self {
            Self::System(r) => r.verify_structure(directory, errors),
            Self::SystemChild(r) => r.verify_structure(directory, errors),
            Self::Tag(r) => r.verify_structure(tags, errors),
        }
    }

    fn finish(&self) -> Block {
        match self {
            Self::System(r) => r.finish(),
            Self::SystemChild(r) => r.finish(),
            Self::Tag(r) => r.finish(),
        }
    }
}

enum ParagraphBuilderElement {
    Reference(ReferenceBuilder),
    InlineMath(MathBuilder),

    UnicornVomitBegin,
    UnicornVomitEnd,
    EmBegin,
    EmEnd,

    Unformatted(UnformattedElement),
}

impl ParagraphBuilderElement {
    fn from_pest(pair: Pair<Rule>, whitespace_rule: Rule) -> ParagraphBuilderElement {
        match pair.as_rule() {
            Rule::text_reference => Self::Reference(ReferenceBuilder::from_pest(pair)),
            Rule::math_row => Self::InlineMath(MathBuilder::from_pest(pair)),

            Rule::unicorn_vomit_begin => Self::UnicornVomitBegin,
            Rule::unicorn_vomit_end => Self::UnicornVomitEnd,
            Rule::em_begin => Self::EmBegin,
            Rule::em_end => Self::EmEnd,

            _ => Self::Unformatted(UnformattedElement::from_pest(pair, whitespace_rule)),
        }
    }

    fn verify_structure(
        &self,
        directory: &BuilderDirectory,
        state: &mut ParagraphFormattingState,
        errors: &mut ParsingErrorContext,
    ) {
        match self {
            Self::Reference(r) => r.verify_structure(directory, errors),

            Self::UnicornVomitBegin => state.unicorn_begin(errors),
            Self::UnicornVomitEnd => state.unicorn_end(errors),
            Self::EmBegin => state.em_begin(errors),
            Self::EmEnd => state.em_end(errors),

            _ => {}
        }
    }

    fn verify_structure_with_tags(
        &self,
        directory: &BuilderDirectory,
        tags: &TagIndex,
        state: &mut ParagraphFormattingState,
        errors: &mut ParsingErrorContext,
    ) {
        match self {
            Self::Reference(r) => r.verify_structure_with_tags(directory, tags, errors),

            Self::UnicornVomitBegin => state.unicorn_begin(errors),
            Self::UnicornVomitEnd => state.unicorn_end(errors),
            Self::EmBegin => state.em_begin(errors),
            Self::EmEnd => state.em_end(errors),

            _ => {}
        }
    }

    fn finish(&self) -> ParagraphElement {
        match self {
            Self::Reference(r) => ParagraphElement::Reference(r.finish()),
            Self::InlineMath(math) => ParagraphElement::InlineMath(math.finish()),

            Self::UnicornVomitBegin => ParagraphElement::UnicornVomitBegin,
            Self::UnicornVomitEnd => ParagraphElement::UnicornVomitEnd,
            Self::EmBegin => ParagraphElement::EmBegin,
            Self::EmEnd => ParagraphElement::EmEnd,

            Self::Unformatted(unformatted) => ParagraphElement::Unformatted(unformatted.clone()),
        }
    }
}

enum ParagraphFormattingState {
    None,
    Unicorn,
    Em,
}

impl ParagraphFormattingState {
    fn unicorn_begin(&mut self, errors: &mut ParsingErrorContext) {
        match self {
            Self::None => *self = Self::Unicorn,

            _ => todo!(),
        }
    }

    fn unicorn_end(&mut self, errors: &mut ParsingErrorContext) {
        match self {
            Self::Unicorn => *self = Self::None,

            _ => todo!(),
        }
    }

    fn em_begin(&mut self, errors: &mut ParsingErrorContext) {
        match self {
            Self::None => *self = Self::Em,

            _ => todo!(),
        }
    }

    fn em_end(&mut self, errors: &mut ParsingErrorContext) {
        match self {
            Self::Em => *self = Self::None,

            _ => todo!(),
        }
    }
}

pub struct ParagraphBuilder {
    elements: Vec<ParagraphBuilderElement>,

    verified: Cell<bool>,
}

impl ParagraphBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> ParagraphBuilder {
        let whitespace_rule = match pair.as_rule() {
            Rule::paragraph => Rule::text_whitespace,
            Rule::oneline => Rule::oneline_whitespace,

            _ => unreachable!(),
        };

        let elements = pair
            .into_inner()
            .map(|pair| ParagraphBuilderElement::from_pest(pair, whitespace_rule))
            .collect();

        ParagraphBuilder {
            elements,

            verified: Cell::new(false),
        }
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        assert!(!self.verified.get());

        let mut state = ParagraphFormattingState::None;

        for element in &self.elements {
            element.verify_structure(directory, &mut state, errors);
        }

        match state {
            ParagraphFormattingState::None => self.verified.set(true),

            _ => todo!(),
        }
    }

    pub fn verify_structure_with_tags(
        &self,
        directory: &BuilderDirectory,
        tags: &TagIndex,
        errors: &mut ParsingErrorContext,
    ) {
        assert!(!self.verified.get());

        let mut state = ParagraphFormattingState::None;

        for element in &self.elements {
            element.verify_structure_with_tags(directory, tags, &mut state, errors);
        }

        match state {
            ParagraphFormattingState::None => self.verified.set(true),

            _ => todo!(),
        }
    }

    pub fn finish(&self) -> Paragraph {
        assert!(self.verified.get());
        let elements = self
            .elements
            .iter()
            .map(ParagraphBuilderElement::finish)
            .collect();

        Paragraph::new(elements)
    }
}

struct TableBuilderRow {
    cells: Vec<ParagraphBuilder>,
}

impl TableBuilderRow {
    fn from_pest(pair: Pair<Rule>) -> TableBuilderRow {
        assert_eq!(pair.as_rule(), Rule::table_row);

        let cells = pair.into_inner().map(ParagraphBuilder::from_pest).collect();

        TableBuilderRow { cells }
    }

    fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        for cell in &self.cells {
            cell.verify_structure(directory, errors);
        }
    }

    fn finish(&self) -> TableBlockRow {
        let cells = self.cells.iter().map(ParagraphBuilder::finish).collect();

        TableBlockRow::new(cells)
    }
}

pub struct TableBuilder {
    head: Option<Vec<TableBuilderRow>>,
    body: Option<Vec<TableBuilderRow>>,
    foot: Option<Vec<TableBuilderRow>>,

    caption: Option<ParagraphBuilder>,
}

impl TableBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> TableBuilder {
        assert_eq!(pair.as_rule(), Rule::table_block);

        let mut head = None;
        let mut body = None;
        let mut foot = None;

        let mut caption = None;

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::table_head => {
                    let rows = pair.into_inner().map(TableBuilderRow::from_pest).collect();

                    head = Some(rows);
                }

                Rule::table_body => {
                    let rows = pair.into_inner().map(TableBuilderRow::from_pest).collect();

                    body = Some(rows);
                }

                Rule::table_foot => {
                    let rows = pair.into_inner().map(TableBuilderRow::from_pest).collect();

                    foot = Some(rows);
                }

                Rule::table_caption => {
                    caption = Some(ParagraphBuilder::from_pest(
                        pair.into_inner().next().unwrap(),
                    ));
                }

                _ => unreachable!(),
            }
        }

        TableBuilder {
            head,
            body,
            foot,

            caption,
        }
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        let head = self.head.iter().flatten();
        let body = self.body.iter().flatten();
        let foot = self.foot.iter().flatten();
        let rows = head.chain(body).chain(foot);

        for row in rows {
            row.verify_structure(directory, errors);
        }

        if let Some(paragraph) = self.caption.as_ref() {
            paragraph.verify_structure(directory, errors);
        }
    }

    pub fn finish(&self) -> TableBlock {
        let head = self
            .head
            .as_ref()
            .map(|rows| rows.iter().map(TableBuilderRow::finish).collect());
        let body = self
            .body
            .as_ref()
            .map(|rows| rows.iter().map(TableBuilderRow::finish).collect());
        let foot = self
            .foot
            .as_ref()
            .map(|rows| rows.iter().map(TableBuilderRow::finish).collect());

        let caption = self.caption.as_ref().map(ParagraphBuilder::finish);

        TableBlock::new(head, body, foot, caption)
    }
}

impl HeadingLevel {
    fn from_pest(pair: Pair<Rule>) -> HeadingLevel {
        match pair.as_rule() {
            Rule::heading_l1 => Self::L1,
            Rule::heading_l2 => Self::L2,
            Rule::heading_l3 => Self::L3,

            _ => unreachable!(),
        }
    }
}

impl SubHeadingBlock {
    fn from_pest(pair: Pair<Rule>) -> SubHeadingBlock {
        assert_eq!(pair.as_rule(), Rule::subheading);

        let mut inner = pair.into_inner();
        let level = HeadingLevel::from_pest(inner.next().unwrap());
        let contents = inner
            .map(|pair| UnformattedElement::from_pest(pair, Rule::heading_whitespace))
            .collect();

        SubHeadingBlock::new(level, contents)
    }
}

pub struct HeadingBuilder {
    subheadings: Vec<SubHeadingBlock>,
}

impl HeadingBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> HeadingBuilder {
        assert_eq!(pair.as_rule(), Rule::heading_block);

        let subheadings = pair.into_inner().map(SubHeadingBlock::from_pest).collect();

        HeadingBuilder { subheadings }
    }

    pub fn finish(&self) -> HeadingBlock {
        let subheadings = self.subheadings.clone();

        HeadingBlock::new(subheadings)
    }
}

pub struct TodoBuilder {
    elements: Vec<TextBuilder>,
}

impl TodoBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> TodoBuilder {
        let elements = pair.into_inner().map(TextBuilder::from_pest).collect();

        TodoBuilder { elements }
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        for element in &self.elements {
            element.verify_structure(directory, errors)
        }
    }

    pub fn finish(&self) -> TodoBlock {
        let elements = self.elements.iter().map(TextBuilder::finish).collect();

        TodoBlock::new(elements)
    }
}

pub struct TextBlockBuilder {
    text: TextBuilder,
}

impl TextBlockBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> TextBlockBuilder {
        assert_eq!(pair.as_rule(), Rule::text_block);

        let text = TextBuilder::from_pest(pair);

        TextBlockBuilder { text }
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        self.text.verify_structure(directory, errors);
    }

    pub fn finish(&self) -> TextBlock {
        let text = self.text.finish();

        TextBlock::new(text)
    }
}
