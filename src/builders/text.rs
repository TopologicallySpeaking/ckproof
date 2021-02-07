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

use std::cell::{Cell, RefCell};

use pest::iterators::{Pair, Pairs};
use url::Url;

use crate::map_ident;

use crate::document::directory::BlockReference;
use crate::document::text::{
    BareElement, BareText, Citation, DisplayMathBlock, HeadingBlock, HeadingLevel, Hyperlink,
    MathBlock, MathElement, Mla, MlaContainer, Paragraph, ParagraphElement, QuoteBlock, QuoteValue,
    SubHeadingBlock, Sublist, SublistItem, TableBlock, TableBlockRow, Text, TextBlock, TodoBlock,
    Unformatted, UnformattedElement,
};

use super::directory::{
    BibliographyBuilderRef, BuilderDirectory, LocalBibliographyBuilderIndex,
    LocalBibliographyBuilderRef, ProofBuilderStepRef, QuoteBuilderRef, SystemBuilderChild,
    SystemBuilderRef, TableBuilderRef, TagIndex, TextBlockBuilderRef, TodoBuilderRef,
};
use super::errors::{
    MlaContainerParsingError, MlaParsingError, ParagraphElementParsingError, ParagraphParsingError,
    ParsingError, ParsingErrorContext, QuoteParsingError, TableParsingError, TextParsingError,
    TodoParsingError,
};
use super::Rule;

fn map_operator(operator: Rule) -> String {
    match operator {
        Rule::operator_plus => "+".to_owned(),
        Rule::operator_minus => "-".to_owned(),
        Rule::operator_asterisk => "*".to_owned(),
        Rule::operator_slash => "/".to_owned(),

        Rule::operator_eq => "=".to_owned(),

        Rule::operator_negation => "\u{00AC}".to_owned(),
        Rule::operator_implies => "\u{21D2}".to_owned(),
        Rule::operator_and => "\u{2227}".to_owned(),

        Rule::operator_bang => "!".to_owned(),

        _ => unreachable!(),
    }
}

impl BareElement {
    fn from_pest(pair: Pair<Rule>, whitespace_rule: Rule) -> BareElement {
        match pair.as_rule() {
            rule if rule == whitespace_rule => Self::Whitespace,

            Rule::open_bracket => Self::OpenBracket,
            Rule::close_bracket => Self::CloseBracket,

            Rule::amp => Self::Ampersand,
            Rule::apos => Self::Apostrophe,
            Rule::ldquo => Self::LeftDoubleQuote,
            Rule::rdquo => Self::RightDoubleQuote,
            Rule::lsquo => Self::LeftSingleQuote,
            Rule::rsquo => Self::RightSingleQuote,
            Rule::ellipsis => Self::Ellipsis,
            Rule::word => Self::Word(pair.as_str().to_owned()),

            _ => unreachable!("{:#?}", pair),
        }
    }
}

impl BareText {
    fn from_pest(pair: Pair<Rule>) -> BareText {
        assert_eq!(pair.as_rule(), Rule::bare_text);

        let elements = pair
            .into_inner()
            .map(|pair| BareElement::from_pest(pair, Rule::bare_whitespace))
            .collect();

        BareText::new(elements)
    }
}

struct CitationBuilder {
    bib_key: String,

    bib_ref: Cell<Option<BibliographyBuilderRef>>,
    local_bib_ref: Cell<Option<LocalBibliographyBuilderRef>>,
}

impl CitationBuilder {
    fn from_pest(pair: Pair<Rule>) -> CitationBuilder {
        assert_eq!(pair.as_rule(), Rule::citation);

        let bib_key = pair.into_inner().next().unwrap().as_str().to_owned();

        CitationBuilder {
            bib_key,

            bib_ref: Cell::new(None),
            local_bib_ref: Cell::new(None),
        }
    }

    fn verify_structure<F>(
        &self,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        assert!(self.bib_ref.get().is_none());
        self.bib_ref.set(directory.search_bib_key(&self.bib_key));

        if self.bib_ref.get().is_none() {
            errors.err(generate_error(
                ParagraphElementParsingError::CitationKeyNotFound,
            ));
        }
    }

    fn bib_refs(&self) -> BibliographyBuilderRef {
        self.bib_ref.get().unwrap()
    }

    fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        assert!(self.local_bib_ref.get().is_none());

        let local_bib_ref = index[self.bib_ref.get().unwrap()];
        self.local_bib_ref.set(Some(local_bib_ref));
    }

    fn finish(&self) -> Citation {
        let local_bib_ref = self.local_bib_ref.get().unwrap().finish();

        Citation::new(local_bib_ref)
    }
}

struct HyperlinkBuilder {
    url: String,
    contents: BareText,

    url_parsed: RefCell<Option<Url>>,
}

impl HyperlinkBuilder {
    fn from_pest(pair: Pair<Rule>) -> HyperlinkBuilder {
        assert_eq!(pair.as_rule(), Rule::hyperlink);

        let mut inner = pair.into_inner();
        let url = inner.next().unwrap().as_str().to_owned();
        let contents = BareText::from_pest(inner.next().unwrap());

        HyperlinkBuilder {
            url,
            contents,

            url_parsed: RefCell::new(None),
        }
    }

    fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        match Url::parse(&self.url) {
            Ok(url_parsed) => {
                let mut cell = self.url_parsed.borrow_mut();
                *cell = Some(url_parsed);
            }

            Err(e) => errors.err(e),
        }
    }

    fn finish(&self) -> Hyperlink {
        let url = self.url_parsed.borrow().clone().unwrap();
        let contents = self.contents.clone();

        Hyperlink::new(url, contents)
    }
}

enum UnformattedBuilderElement {
    Hyperlink(HyperlinkBuilder),
    BareElement(BareElement),
}

impl UnformattedBuilderElement {
    fn from_pest(pair: Pair<Rule>, whitespace_rule: Rule) -> UnformattedBuilderElement {
        match pair.as_rule() {
            Rule::hyperlink => Self::Hyperlink(HyperlinkBuilder::from_pest(pair)),

            _ => Self::BareElement(BareElement::from_pest(pair, whitespace_rule)),
        }
    }

    fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        match self {
            Self::Hyperlink(hyperlink) => hyperlink.verify_structure(errors),

            _ => {}
        }
    }

    fn finish(&self) -> UnformattedElement {
        match self {
            Self::Hyperlink(hyperlink) => UnformattedElement::Hyperlink(hyperlink.finish()),
            Self::BareElement(text) => UnformattedElement::BareElement(text.clone()),
        }
    }
}

struct UnformattedBuilder {
    elements: Vec<UnformattedBuilderElement>,
}

impl UnformattedBuilder {
    fn from_pest(pair: Pair<Rule>) -> UnformattedBuilder {
        assert_eq!(pair.as_rule(), Rule::unformatted);

        let elements = pair
            .into_inner()
            .map(|pair| UnformattedBuilderElement::from_pest(pair, Rule::oneline_whitespace))
            .collect();

        UnformattedBuilder { elements }
    }

    fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        for element in &self.elements {
            element.verify_structure(errors);
        }
    }

    fn finish(&self) -> Unformatted {
        let elements = self
            .elements
            .iter()
            .map(UnformattedBuilderElement::finish)
            .collect();

        Unformatted::new(elements)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MlaContainerRef(usize);

pub struct MlaContainerBuilder {
    container_titles: Vec<UnformattedBuilder>,
    other_contributors: Vec<UnformattedBuilder>,
    versions: Vec<UnformattedBuilder>,
    numbers: Vec<UnformattedBuilder>,
    publishers: Vec<UnformattedBuilder>,
    publication_dates: Vec<UnformattedBuilder>,
    locations: Vec<UnformattedBuilder>,

    self_ref: MlaContainerRef,
    verified: Cell<bool>,
}

impl MlaContainerBuilder {
    fn from_pest(pairs: Pairs<Rule>, self_ref: MlaContainerRef) -> MlaContainerBuilder {
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
                    let container_title =
                        UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    container_titles.push(container_title);
                }

                Rule::mla_other_contributors => {
                    let other_contributor =
                        UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    other_contributors.push(other_contributor);
                }

                Rule::mla_version => {
                    let version = UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    versions.push(version);
                }

                Rule::mla_number => {
                    let number = UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    numbers.push(number);
                }

                Rule::mla_publisher => {
                    let publisher =
                        UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    publishers.push(publisher);
                }

                Rule::mla_publication_date => {
                    let publication_date =
                        UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    publication_dates.push(publication_date);
                }

                Rule::mla_location => {
                    let location = UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

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

            self_ref,
            verified: Cell::new(false),
        }
    }

    fn verify_structure<F>(&self, errors: &mut ParsingErrorContext, generate_error: F)
    where
        F: Fn(MlaParsingError) -> ParsingError,
    {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.container_titles.len() {
            0 => {}
            1 => self.container_titles[0].verify_structure(errors),
            _ => {
                found_error = true;

                errors.err(generate_error(MlaParsingError::ContainerError(
                    self.self_ref,
                    MlaContainerParsingError::DuplicateTitle,
                )))
            }
        }

        match self.other_contributors.len() {
            0 => {}
            1 => self.other_contributors[0].verify_structure(errors),
            _ => {
                found_error = true;

                errors.err(generate_error(MlaParsingError::ContainerError(
                    self.self_ref,
                    MlaContainerParsingError::DuplicateOtherContributors,
                )))
            }
        }

        match self.versions.len() {
            0 => {}
            1 => self.versions[0].verify_structure(errors),
            _ => {
                found_error = true;

                errors.err(generate_error(MlaParsingError::ContainerError(
                    self.self_ref,
                    MlaContainerParsingError::DuplicateVersion,
                )))
            }
        }

        match self.numbers.len() {
            0 => {}
            1 => self.numbers[0].verify_structure(errors),
            _ => {
                found_error = true;

                errors.err(generate_error(MlaParsingError::ContainerError(
                    self.self_ref,
                    MlaContainerParsingError::DuplicateNumber,
                )))
            }
        }

        match self.publishers.len() {
            0 => {}
            1 => self.publishers[0].verify_structure(errors),
            _ => {
                found_error = true;

                errors.err(generate_error(MlaParsingError::ContainerError(
                    self.self_ref,
                    MlaContainerParsingError::DuplicatePublisher,
                )))
            }
        }

        match self.publication_dates.len() {
            0 => {}
            1 => self.publication_dates[0].verify_structure(errors),
            _ => {
                found_error = true;

                errors.err(generate_error(MlaParsingError::ContainerError(
                    self.self_ref,
                    MlaContainerParsingError::DuplicatePublicationDate,
                )))
            }
        }

        match self.locations.len() {
            0 => {}
            1 => self.locations[0].verify_structure(errors),
            _ => {
                found_error = true;

                errors.err(generate_error(MlaParsingError::ContainerError(
                    self.self_ref,
                    MlaContainerParsingError::DuplicateLocation,
                )))
            }
        }

        self.verified.set(!found_error);
    }

    fn finish(&self) -> MlaContainer {
        assert!(self.verified.get());

        let container_title = self.container_titles.get(0).map(UnformattedBuilder::finish);
        let other_contributors = self
            .other_contributors
            .get(0)
            .map(UnformattedBuilder::finish);
        let version = self.versions.get(0).map(UnformattedBuilder::finish);
        let number = self.numbers.get(0).map(UnformattedBuilder::finish);
        let publisher = self.publishers.get(0).map(UnformattedBuilder::finish);
        let publication_date = self
            .publication_dates
            .get(0)
            .map(UnformattedBuilder::finish);
        let location = self.locations.get(0).map(UnformattedBuilder::finish);

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

pub struct MlaBuilderEntries {
    authors: Vec<UnformattedBuilder>,
    titles: Vec<UnformattedBuilder>,
    containers: Vec<MlaContainerBuilder>,

    verified: Cell<bool>,
}

impl MlaBuilderEntries {
    pub fn from_pest(pairs: Pairs<Rule>) -> MlaBuilderEntries {
        let mut authors = Vec::with_capacity(1);
        let mut titles = Vec::with_capacity(1);
        let mut containers = Vec::new();

        for pair in pairs {
            match pair.as_rule() {
                Rule::mla_authors => authors.push(UnformattedBuilder::from_pest(
                    pair.into_inner().next().unwrap(),
                )),
                Rule::mla_title => titles.push(UnformattedBuilder::from_pest(
                    pair.into_inner().next().unwrap(),
                )),
                Rule::mla_container => {
                    let container_ref = MlaContainerRef(containers.len());
                    containers.push(MlaContainerBuilder::from_pest(
                        pair.into_inner(),
                        container_ref,
                    ))
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

    pub fn verify_structure<F>(&self, errors: &mut ParsingErrorContext, generate_error: F)
    where
        F: Fn(MlaParsingError) -> ParsingError,
    {
        assert!(!self.verified.get());
        let found_error = false;

        match self.authors.len() {
            0 => {}
            1 => self.authors[0].verify_structure(errors),
            _ => errors.err(generate_error(MlaParsingError::DuplicateName)),
        }

        match self.titles.len() {
            0 => errors.err(generate_error(MlaParsingError::MissingTitle)),
            1 => self.titles[0].verify_structure(errors),
            _ => errors.err(generate_error(MlaParsingError::DuplicateTitle)),
        }

        for container in &self.containers {
            container.verify_structure(errors, |e| generate_error(e));
        }

        self.verified.set(!found_error);
    }

    pub fn finish(&self) -> Mla {
        let author = self.author().map(UnformattedBuilder::finish);
        let title = self.title().finish();
        let containers = self
            .containers
            .iter()
            .map(MlaContainerBuilder::finish)
            .collect();

        Mla::new(author, title, containers)
    }

    fn author(&self) -> Option<&UnformattedBuilder> {
        assert!(self.verified.get());
        self.authors.get(0)
    }

    fn title(&self) -> &UnformattedBuilder {
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

    fn verify_structure<F>(&self, errors: &mut ParsingErrorContext, generate_error: F)
    where
        F: Fn(MlaParsingError) -> ParsingError,
    {
        self.entries.verify_structure(errors, generate_error);
    }

    fn finish(&self) -> Mla {
        self.entries.finish()
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
        let var_id = map_ident(var_inner.as_str()).to_owned();
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
    Number(String),
}

impl MathBuilderElement {
    fn from_pest(pair: Pair<Rule>) -> MathBuilderElement {
        match pair.as_rule() {
            Rule::math_row => Self::Fenced(MathBuilder::from_pest(pair)),

            Rule::display_operator => {
                Self::Operator(map_operator(pair.into_inner().next().unwrap().as_rule()))
            }
            Rule::ident => Self::Symbol(map_ident(pair.as_str()).to_owned()),
            Rule::var => {
                Self::Variable(map_ident(pair.into_inner().next().unwrap().as_str()).to_owned())
            }
            Rule::integer => Self::Number(pair.as_str().to_owned()),

            _ => unreachable!(),
        }
    }

    fn from_pest_formula(pair: Pair<Rule>) -> MathBuilderElement {
        match pair.as_rule() {
            Rule::primary_paren => Self::Fenced(MathBuilder::from_pest_formula(
                pair.into_inner().next().unwrap(),
            )),

            Rule::read_operator => {
                Self::Operator(map_operator(pair.into_inner().next().unwrap().as_rule()).to_owned())
            }
            Rule::ident => Self::Symbol(map_ident(pair.as_str()).to_owned()),
            Rule::var => {
                Self::Variable(map_ident(pair.into_inner().next().unwrap().as_str()).to_owned())
            }

            _ => unreachable!("{:#?}", pair),
        }
    }

    fn finish(&self) -> MathElement {
        match self {
            Self::Fenced(builder) => MathElement::Fenced(builder.finish()),

            Self::Operator(op) => MathElement::Operator(op.clone()),
            Self::Symbol(s) => MathElement::Symbol(s.clone()),
            Self::Variable(v) => MathElement::Variable(v.clone()),
            Self::Number(n) => MathElement::Number(n.clone()),
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

    pub fn from_pest_formula(pair: Pair<Rule>) -> MathBuilder {
        assert_eq!(pair.as_rule(), Rule::formula);

        let elements = pair
            .into_inner()
            .flat_map(|pair| match pair.as_rule() {
                Rule::prefix_list => {
                    Box::new(pair.into_inner().map(MathBuilderElement::from_pest_formula))
                        as Box<dyn Iterator<Item = MathBuilderElement>>
                }

                _ => Box::new(Some(MathBuilderElement::from_pest_formula(pair)).into_iter())
                    as Box<dyn Iterator<Item = MathBuilderElement>>,
            })
            .collect();

        MathBuilder { elements }
    }

    pub fn finish(&self) -> MathBlock {
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

    pub fn verify_structure<F>(
        &self,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(TextParsingError) -> ParsingError,
    {
        match self {
            Self::Mla(mla) => {
                mla.verify_structure(errors, |e| generate_error(TextParsingError::MlaError(e)))
            }
            Self::Paragraph(paragraph) => paragraph.verify_structure(directory, errors, |e| {
                generate_error(TextParsingError::ParagraphError(e))
            }),

            Self::Sublist(_) | Self::DisplayMath(_) => {}
        }
    }

    pub fn verify_structure_with_tags<F>(
        &self,
        directory: &BuilderDirectory,
        tags: &TagIndex,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(TextParsingError) -> ParsingError,
    {
        match self {
            Self::Mla(mla) => {
                mla.verify_structure(errors, |e| generate_error(TextParsingError::MlaError(e)))
            }
            Self::Paragraph(paragraph) => {
                paragraph.verify_structure_with_tags(directory, tags, errors, |e| {
                    generate_error(TextParsingError::ParagraphError(e))
                })
            }

            Self::Sublist(_) | Self::DisplayMath(_) => {}
        }
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        let ret = match self {
            Self::Paragraph(paragraph) => Some(paragraph.bib_refs()),
            _ => None,
        };

        Box::new(ret.into_iter().flatten())
    }

    pub fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        if let Self::Paragraph(paragraph) = self {
            paragraph.set_local_bib_refs(index);
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

    fn verify_structure<F>(
        &self,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        self.system_ref.set(directory.search_system(&self.id));

        if self.system_ref.get().is_none() {
            errors.err(generate_error(
                ParagraphElementParsingError::SystemReferenceIdNotFound,
            ));
        }
    }

    fn finish(&self) -> BlockReference {
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

    fn verify_structure<F>(
        &self,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        self.child_ref
            .set(directory.search_system_child(&self.system_id, &self.child_id));

        if self.child_ref.get().is_none() {
            errors.err(generate_error(
                ParagraphElementParsingError::SystemChildReferenceIdNotFound,
            ));
        }
    }

    fn finish(&self) -> BlockReference {
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

    fn verify_structure<F>(
        &self,
        tags: &TagIndex,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        self.step_ref.set(tags.search(&self.tag));

        if self.step_ref.get().is_none() {
            errors.err(generate_error(
                ParagraphElementParsingError::TagReferenceNotFound,
            ));
        }
    }

    fn finish(&self) -> BlockReference {
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

    fn verify_structure<F>(
        &self,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        match self {
            Self::System(r) => r.verify_structure(directory, errors, generate_error),
            Self::SystemChild(r) => r.verify_structure(directory, errors, generate_error),
            Self::Tag(_) => unreachable!(),
        }
    }

    fn verify_structure_with_tags<F>(
        &self,
        directory: &BuilderDirectory,
        tags: &TagIndex,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        match self {
            Self::System(r) => r.verify_structure(directory, errors, generate_error),
            Self::SystemChild(r) => r.verify_structure(directory, errors, generate_error),
            Self::Tag(r) => r.verify_structure(tags, errors, generate_error),
        }
    }

    fn finish(&self) -> BlockReference {
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
    Citation(CitationBuilder),

    UnicornVomitBegin,
    UnicornVomitEnd,
    EmBegin,
    EmEnd,

    Unformatted(UnformattedBuilderElement),
}

impl ParagraphBuilderElement {
    fn from_pest(pair: Pair<Rule>, whitespace_rule: Rule) -> ParagraphBuilderElement {
        match pair.as_rule() {
            Rule::text_reference => Self::Reference(ReferenceBuilder::from_pest(pair)),
            Rule::math_row => Self::InlineMath(MathBuilder::from_pest(pair)),
            Rule::citation => Self::Citation(CitationBuilder::from_pest(pair)),

            Rule::unicorn_vomit_begin => Self::UnicornVomitBegin,
            Rule::unicorn_vomit_end => Self::UnicornVomitEnd,
            Rule::em_begin => Self::EmBegin,
            Rule::em_end => Self::EmEnd,

            _ => Self::Unformatted(UnformattedBuilderElement::from_pest(pair, whitespace_rule)),
        }
    }

    fn verify_structure<F>(
        &self,
        directory: &BuilderDirectory,
        state: &mut ParagraphFormattingState,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        match self {
            Self::Reference(r) => r.verify_structure(directory, errors, generate_error),
            Self::InlineMath(_) => {}
            Self::Citation(citation) => {
                citation.verify_structure(directory, errors, generate_error)
            }

            Self::UnicornVomitBegin => state.unicorn_begin(errors, generate_error),
            Self::UnicornVomitEnd => state.unicorn_end(errors, generate_error),
            Self::EmBegin => state.em_begin(errors, generate_error),
            Self::EmEnd => state.em_end(errors, generate_error),

            Self::Unformatted(builder) => builder.verify_structure(errors),
        }
    }

    fn verify_structure_with_tags<F>(
        &self,
        directory: &BuilderDirectory,
        tags: &TagIndex,
        state: &mut ParagraphFormattingState,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        match self {
            Self::Reference(r) => {
                r.verify_structure_with_tags(directory, tags, errors, generate_error)
            }
            Self::InlineMath(_) => {}
            Self::Citation(citation) => {
                citation.verify_structure(directory, errors, generate_error)
            }

            Self::UnicornVomitBegin => state.unicorn_begin(errors, generate_error),
            Self::UnicornVomitEnd => state.unicorn_end(errors, generate_error),
            Self::EmBegin => state.em_begin(errors, generate_error),
            Self::EmEnd => state.em_end(errors, generate_error),

            Self::Unformatted(builder) => builder.verify_structure(errors),
        }
    }

    fn bib_refs(&self) -> Option<BibliographyBuilderRef> {
        match self {
            Self::Citation(citation) => Some(citation.bib_refs()),
            _ => None,
        }
    }

    fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        if let Self::Citation(citation) = self {
            citation.set_local_bib_refs(index)
        }
    }

    fn finish(&self) -> ParagraphElement {
        match self {
            Self::Reference(r) => ParagraphElement::Reference(r.finish()),
            Self::InlineMath(math) => ParagraphElement::InlineMath(math.finish()),
            Self::Citation(citation) => ParagraphElement::Citation(citation.finish()),

            Self::UnicornVomitBegin => ParagraphElement::UnicornVomitBegin,
            Self::UnicornVomitEnd => ParagraphElement::UnicornVomitEnd,
            Self::EmBegin => ParagraphElement::EmBegin,
            Self::EmEnd => ParagraphElement::EmEnd,

            Self::Unformatted(unformatted) => ParagraphElement::Unformatted(unformatted.finish()),
        }
    }
}

enum ParagraphFormattingState {
    None,
    Unicorn,
    Em,
}

impl ParagraphFormattingState {
    fn unicorn_begin<F>(&mut self, errors: &mut ParsingErrorContext, generate_error: F)
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        match self {
            Self::None => *self = Self::Unicorn,

            _ => errors.err(generate_error(
                ParagraphElementParsingError::UnexpectedUnicornVomitBegin,
            )),
        }
    }

    fn unicorn_end<F>(&mut self, errors: &mut ParsingErrorContext, generate_error: F)
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        match self {
            Self::Unicorn => *self = Self::None,

            _ => errors.err(generate_error(
                ParagraphElementParsingError::UnexpectedUnicornVomitEnd,
            )),
        }
    }

    fn em_begin<F>(&mut self, errors: &mut ParsingErrorContext, generate_error: F)
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        match self {
            Self::None => *self = Self::Em,

            _ => errors.err(generate_error(
                ParagraphElementParsingError::UnexpectedEmBegin,
            )),
        }
    }

    fn em_end<F>(&mut self, errors: &mut ParsingErrorContext, generate_error: F)
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError,
    {
        match self {
            Self::Em => *self = Self::None,

            _ => errors.err(generate_error(
                ParagraphElementParsingError::UnexpectedEmEnd,
            )),
        }
    }

    fn verify<F>(self, errors: &mut ParsingErrorContext, generate_error: F) -> bool
    where
        F: Fn(ParagraphParsingError) -> ParsingError,
    {
        match self {
            Self::None => true,

            Self::Unicorn => {
                errors.err(generate_error(ParagraphParsingError::UnclosedUnicornVomit));
                false
            }

            Self::Em => {
                errors.err(generate_error(ParagraphParsingError::UnclosedEm));
                false
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ParagraphBuilderElementRef(usize);

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

    pub fn verify_structure<F>(
        &self,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ParagraphParsingError) -> ParsingError,
    {
        assert!(!self.verified.get());

        let mut state = ParagraphFormattingState::None;

        for (i, element) in self.elements.iter().enumerate() {
            element.verify_structure(directory, &mut state, errors, |e| {
                generate_error(ParagraphParsingError::ElementError(
                    ParagraphBuilderElementRef(i),
                    e,
                ))
            });
        }

        if state.verify(errors, generate_error) {
            self.verified.set(true);
        }
    }

    pub fn verify_structure_with_tags<F>(
        &self,
        directory: &BuilderDirectory,
        tags: &TagIndex,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn(ParagraphParsingError) -> ParsingError,
    {
        assert!(!self.verified.get());

        let mut state = ParagraphFormattingState::None;

        for (i, element) in self.elements.iter().enumerate() {
            element.verify_structure_with_tags(directory, tags, &mut state, errors, |e| {
                generate_error(ParagraphParsingError::ElementError(
                    ParagraphBuilderElementRef(i),
                    e,
                ))
            });
        }

        if state.verify(errors, generate_error) {
            self.verified.set(true);
        }
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        Box::new(
            self.elements
                .iter()
                .flat_map(ParagraphBuilderElement::bib_refs),
        )
    }

    pub fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        for element in &self.elements {
            element.set_local_bib_refs(index)
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

#[derive(Clone, Copy, Debug)]
enum CellLocation {
    Head,
    Body,
    Foot,
}

#[derive(Clone, Copy, Debug)]
pub struct TableBuilderCellRef(CellLocation, usize, usize);

struct TableBuilderRow {
    cells: Vec<ParagraphBuilder>,
}

impl TableBuilderRow {
    fn from_pest(pair: Pair<Rule>) -> TableBuilderRow {
        assert_eq!(pair.as_rule(), Rule::table_row);

        let cells = pair.into_inner().map(ParagraphBuilder::from_pest).collect();

        TableBuilderRow { cells }
    }

    fn verify_structure(
        &self,
        table_ref: TableBuilderRef,
        location: CellLocation,
        row_index: usize,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
    ) {
        for (cell_index, cell) in self.cells.iter().enumerate() {
            cell.verify_structure(directory, errors, |e| {
                ParsingError::TableError(
                    table_ref,
                    TableParsingError::CellParsingError(
                        TableBuilderCellRef(location, row_index, cell_index),
                        e,
                    ),
                )
            });
        }
    }

    fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        Box::new(self.cells.iter().flat_map(ParagraphBuilder::bib_refs))
    }

    fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        for cell in &self.cells {
            cell.set_local_bib_refs(index)
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

    self_ref: Option<TableBuilderRef>,
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

            self_ref: None,
        }
    }

    pub fn set_self_ref(&mut self, self_ref: TableBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(self_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        if let Some(head) = &self.head {
            for (row_index, row) in head.iter().enumerate() {
                row.verify_structure(
                    self.self_ref.unwrap(),
                    CellLocation::Head,
                    row_index,
                    directory,
                    errors,
                );
            }
        }
        if let Some(body) = &self.body {
            for (row_index, row) in body.iter().enumerate() {
                row.verify_structure(
                    self.self_ref.unwrap(),
                    CellLocation::Body,
                    row_index,
                    directory,
                    errors,
                );
            }
        }
        if let Some(foot) = &self.foot {
            for (row_index, row) in foot.iter().enumerate() {
                row.verify_structure(
                    self.self_ref.unwrap(),
                    CellLocation::Foot,
                    row_index,
                    directory,
                    errors,
                );
            }
        }

        if let Some(paragraph) = self.caption.as_ref() {
            paragraph.verify_structure(directory, errors, |e| {
                ParsingError::TableError(
                    self.self_ref.unwrap(),
                    TableParsingError::CaptionParsingError(e),
                )
            });
        }
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        let head = self.head.iter().flatten();
        let body = self.body.iter().flatten();
        let foot = self.foot.iter().flatten();
        let rows = head.chain(body).chain(foot);

        let row_refs = rows.flat_map(TableBuilderRow::bib_refs);
        let caption_refs = self.caption.iter().flat_map(ParagraphBuilder::bib_refs);

        Box::new(row_refs.chain(caption_refs))
    }

    pub fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        let head = self.head.iter().flatten();
        let body = self.body.iter().flatten();
        let foot = self.foot.iter().flatten();
        let rows = head.chain(body).chain(foot);

        for row in rows {
            row.set_local_bib_refs(index);
        }

        if let Some(paragraph) = self.caption.as_ref() {
            paragraph.set_local_bib_refs(index);
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

struct QuoteValueBuilder {
    bib_key: String,
    quote: UnformattedBuilder,

    bib_ref: Cell<Option<BibliographyBuilderRef>>,
    local_bib_ref: Cell<Option<LocalBibliographyBuilderRef>>,
}

impl QuoteValueBuilder {
    fn from_pest(pair: Pair<Rule>) -> QuoteValueBuilder {
        assert!(pair.as_rule() == Rule::quote_value || pair.as_rule() == Rule::quote_original);

        let mut inner = pair.into_inner();
        let bib_key = inner.next().unwrap().as_str().to_owned();
        let quote = UnformattedBuilder::from_pest(inner.next().unwrap());

        QuoteValueBuilder {
            bib_key,
            quote,

            bib_ref: Cell::new(None),
            local_bib_ref: Cell::new(None),
        }
    }

    fn verify_structure<F>(
        &self,
        directory: &BuilderDirectory,
        errors: &mut ParsingErrorContext,
        generate_error: F,
    ) where
        F: Fn() -> ParsingError,
    {
        assert!(self.bib_ref.get().is_none());

        self.bib_ref.set(directory.search_bib_key(&self.bib_key));
        if self.bib_ref.get().is_none() {
            errors.err(generate_error());
        }
    }

    fn bib_ref(&self) -> BibliographyBuilderRef {
        self.bib_ref.get().unwrap()
    }

    fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        assert!(self.local_bib_ref.get().is_none());

        let local_bib_ref = index[self.bib_ref.get().unwrap()];
        self.local_bib_ref.set(Some(local_bib_ref))
    }

    fn finish(&self) -> QuoteValue {
        let quote = self.quote.finish();
        let local_bib_ref = self.local_bib_ref.get().unwrap().finish();

        QuoteValue::new(quote, local_bib_ref)
    }
}

pub struct QuoteBuilder {
    original: Option<QuoteValueBuilder>,
    value: QuoteValueBuilder,

    self_ref: Option<QuoteBuilderRef>,
}

impl QuoteBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> QuoteBuilder {
        assert_eq!(pair.as_rule(), Rule::quote_block);

        let mut inner = pair.into_inner();
        let mut curr = inner.next().unwrap();

        let original = if curr.as_rule() == Rule::quote_original {
            let original = curr;
            curr = inner.next().unwrap();

            Some(QuoteValueBuilder::from_pest(original))
        } else {
            None
        };

        let value = QuoteValueBuilder::from_pest(curr);

        QuoteBuilder {
            original,
            value,

            self_ref: None,
        }
    }

    pub fn set_self_ref(&mut self, self_ref: QuoteBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(self_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        self.original.as_ref().map(|original| {
            original.verify_structure(directory, errors, || {
                ParsingError::QuoteError(
                    self.self_ref.unwrap(),
                    QuoteParsingError::OriginalKeyNotFound,
                )
            })
        });
        self.value.verify_structure(directory, errors, || {
            ParsingError::QuoteError(self.self_ref.unwrap(), QuoteParsingError::ValueKeyNotFound)
        });
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        let original_ref = self.original.iter().map(QuoteValueBuilder::bib_ref);
        let value_ref = self.value.bib_ref();

        Box::new(original_ref.chain(Some(value_ref)))
    }

    pub fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        if let Some(original) = &self.original {
            original.set_local_bib_refs(index);
        }

        self.value.set_local_bib_refs(index);
    }

    pub fn finish(&self) -> QuoteBlock {
        let original = self.original.as_ref().map(QuoteValueBuilder::finish);
        let value = self.value.finish();

        QuoteBlock::new(original, value)
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

struct SubHeadingBuilder {
    level: HeadingLevel,
    contents: Vec<UnformattedBuilderElement>,
}

impl SubHeadingBuilder {
    fn from_pest(pair: Pair<Rule>) -> SubHeadingBuilder {
        assert_eq!(pair.as_rule(), Rule::subheading);

        let mut inner = pair.into_inner();
        let level = HeadingLevel::from_pest(inner.next().unwrap());
        let contents = inner
            .map(|pair| UnformattedBuilderElement::from_pest(pair, Rule::heading_whitespace))
            .collect();

        SubHeadingBuilder { level, contents }
    }

    fn finish(&self) -> SubHeadingBlock {
        let level = self.level;
        let contents = self
            .contents
            .iter()
            .map(UnformattedBuilderElement::finish)
            .collect();

        SubHeadingBlock::new(level, contents)
    }
}

pub struct HeadingBuilder {
    subheadings: Vec<SubHeadingBuilder>,
}

impl HeadingBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> HeadingBuilder {
        assert_eq!(pair.as_rule(), Rule::heading_block);

        let subheadings = pair
            .into_inner()
            .map(SubHeadingBuilder::from_pest)
            .collect();

        HeadingBuilder { subheadings }
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        Box::new(std::iter::empty())
    }

    pub fn finish(&self) -> HeadingBlock {
        let subheadings = self
            .subheadings
            .iter()
            .map(SubHeadingBuilder::finish)
            .collect();

        HeadingBlock::new(subheadings)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TodoBuilderElementRef(usize);

pub struct TodoBuilder {
    elements: Vec<TextBuilder>,

    self_ref: Option<TodoBuilderRef>,
}

impl TodoBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> TodoBuilder {
        let elements = pair.into_inner().map(TextBuilder::from_pest).collect();

        TodoBuilder {
            elements,

            self_ref: None,
        }
    }

    pub fn set_self_ref(&mut self, self_ref: TodoBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(self_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        for (i, element) in self.elements.iter().enumerate() {
            element.verify_structure(directory, errors, |e| {
                ParsingError::TodoError(
                    self.self_ref.unwrap(),
                    TodoParsingError::TextError(TodoBuilderElementRef(i), e),
                )
            });
        }
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        Box::new(self.elements.iter().flat_map(TextBuilder::bib_refs))
    }

    pub fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        for element in &self.elements {
            element.set_local_bib_refs(index);
        }
    }

    pub fn finish(&self) -> TodoBlock {
        let elements = self.elements.iter().map(TextBuilder::finish).collect();

        TodoBlock::new(elements)
    }
}

pub struct TextBlockBuilder {
    text: TextBuilder,

    self_ref: Option<TextBlockBuilderRef>,
}

impl TextBlockBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> TextBlockBuilder {
        assert_eq!(pair.as_rule(), Rule::text_block);

        let text = TextBuilder::from_pest(pair);

        TextBlockBuilder {
            text,

            self_ref: None,
        }
    }

    pub fn set_self_ref(&mut self, self_ref: TextBlockBuilderRef) {
        assert!(self.self_ref.is_none());
        self.self_ref = Some(self_ref);
    }

    pub fn verify_structure(&self, directory: &BuilderDirectory, errors: &mut ParsingErrorContext) {
        self.text.verify_structure(directory, errors, |e| {
            ParsingError::TextBlockError(self.self_ref.unwrap(), e)
        });
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = BibliographyBuilderRef> + '_> {
        self.text.bib_refs()
    }

    pub fn set_local_bib_refs(&self, index: &LocalBibliographyBuilderIndex) {
        self.text.set_local_bib_refs(index)
    }

    pub fn finish(&self) -> TextBlock {
        let text = self.text.finish();

        TextBlock::new(text)
    }
}
