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

use std::cell::Cell;
use std::collections::HashMap;
use std::lazy::OnceCell;

use pest::iterators::{Pair, Pairs};
use url::Url;

use crate::document::directory::{
    BlockReference, HeadingBlockRef, LocalBibliographyRef, QuoteBlockRef, TableBlockRef,
    TextBlockRef, TodoBlockRef,
};
use crate::document::text::{
    BareElement, BareText, Citation, DisplayMathBlock, HeadingBlock, HeadingLevel, Hyperlink,
    MathBlock, MathElement, Mla, MlaContainer, Paragraph, ParagraphElement, QuoteBlock, QuoteValue,
    SubHeadingBlock, Sublist, SublistItem, TableBlock, TableBlockRow, Text, TextBlock, TodoBlock,
    Unformatted, UnformattedElement,
};
use crate::map_ident;

use super::bibliography::BibliographyBuilderEntry;
use super::errors::{
    ParagraphElementParsingError, ParagraphParsingError, ParsingError, ParsingErrorContext,
    QuoteParsingError, QuoteValueParsingError, RawCitationContainerParsingError,
    RawCitationParsingError, TableParsingError, TextParsingError,
};
use super::index::BuilderIndex;
use super::system::{ProofBuilderStep, SystemBuilder, SystemBuilderChild};
use super::Rule;

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

#[derive(Debug)]
struct CitationBuilder<'a> {
    bib_key: String,

    bib_ref: OnceCell<&'a BibliographyBuilderEntry>,
    local_bib_index: OnceCell<usize>,
}

impl<'a> CitationBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::citation);

        let bib_key = pair.into_inner().next().unwrap().as_str().to_owned();

        CitationBuilder {
            bib_key,

            bib_ref: OnceCell::new(),
            local_bib_index: OnceCell::new(),
        }
    }

    fn verify_structure<F>(
        &self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match index.search_bib_ref(&self.bib_key) {
            Some(bib_ref) => {
                self.bib_ref.set(bib_ref).unwrap();
                true
            }

            None => {
                errors.err(generate_error(
                    ParagraphElementParsingError::CitationKeyNotFound,
                ));
                false
            }
        }
    }

    fn bib_ref(&self) -> &BibliographyBuilderEntry {
        self.bib_ref.get().unwrap()
    }

    fn set_local_bib_ref(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        let local_bib_index = *index.get(self.bib_ref.get().unwrap()).unwrap();
        self.local_bib_index.set(local_bib_index).unwrap();
    }

    // TODO: Remove.
    fn finish(&self) -> Citation {
        let local_bib_ref = LocalBibliographyRef::new(*self.local_bib_index.get().unwrap());
        Citation::new(local_bib_ref)
    }
}

#[derive(Debug)]
struct HyperlinkBuilder {
    url: String,
    contents: BareText,

    url_parsed: OnceCell<Url>,
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

            url_parsed: OnceCell::new(),
        }
    }

    fn verify_structure<'a>(&self, errors: &mut ParsingErrorContext<'a>) -> bool {
        match Url::parse(&self.url) {
            Ok(url_parsed) => {
                self.url_parsed.set(url_parsed).unwrap();
                true
            }

            Err(e) => {
                errors.err(e);
                false
            }
        }
    }

    // TODO: Remove.
    fn finish(&self) -> Hyperlink {
        let url = self.url_parsed.get().unwrap().clone();
        let contents = self.contents.clone();

        Hyperlink::new(url, contents)
    }
}

#[derive(Debug)]
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

    fn verify_structure(&self, errors: &mut ParsingErrorContext) -> bool {
        match self {
            Self::Hyperlink(hyperlink) => hyperlink.verify_structure(errors),

            _ => true,
        }
    }

    // TODO: Remove.
    fn finish(&self) -> UnformattedElement {
        match self {
            Self::Hyperlink(hyperlink) => UnformattedElement::Hyperlink(hyperlink.finish()),
            Self::BareElement(element) => UnformattedElement::BareElement(element.clone()),
        }
    }
}

#[derive(Debug)]
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

    fn verify_structure(&self, errors: &mut ParsingErrorContext) -> bool {
        let mut success = true;

        for element in &self.elements {
            if !element.verify_structure(errors) {
                success = false;
            }
        }

        success
    }

    // TODO: Remove.
    fn finish(&self) -> Unformatted {
        let elements = self
            .elements
            .iter()
            .map(UnformattedBuilderElement::finish)
            .collect();

        Unformatted::new(elements)
    }
}

#[derive(Debug)]
pub struct RawCitationContainerBuilder {
    container_titles: Vec<UnformattedBuilder>,
    other_contributors: Vec<UnformattedBuilder>,
    versions: Vec<UnformattedBuilder>,
    numbers: Vec<UnformattedBuilder>,
    publishers: Vec<UnformattedBuilder>,
    publication_dates: Vec<UnformattedBuilder>,
    locations: Vec<UnformattedBuilder>,

    verified: Cell<bool>,
}

impl RawCitationContainerBuilder {
    fn from_pest(pairs: Pairs<Rule>) -> RawCitationContainerBuilder {
        let mut container_titles = Vec::with_capacity(1);
        let mut other_contributors = Vec::with_capacity(1);
        let mut versions = Vec::with_capacity(1);
        let mut numbers = Vec::with_capacity(1);
        let mut publishers = Vec::with_capacity(1);
        let mut publication_dates = Vec::with_capacity(1);
        let mut locations = Vec::with_capacity(1);

        for pair in pairs {
            match pair.as_rule() {
                Rule::raw_citation_container_title => {
                    let container_title =
                        UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    container_titles.push(container_title);
                }

                Rule::raw_citation_other_contributors => {
                    let other_contributor =
                        UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    other_contributors.push(other_contributor);
                }

                Rule::raw_citation_version => {
                    let version = UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    versions.push(version);
                }

                Rule::raw_citation_number => {
                    let number = UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    numbers.push(number);
                }

                Rule::raw_citation_publisher => {
                    let publisher =
                        UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    publishers.push(publisher);
                }

                Rule::raw_citation_publication_date => {
                    let publication_date =
                        UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    publication_dates.push(publication_date);
                }

                Rule::raw_citation_location => {
                    let location = UnformattedBuilder::from_pest(pair.into_inner().next().unwrap());

                    locations.push(location);
                }

                _ => unreachable!(),
            }
        }

        RawCitationContainerBuilder {
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

    fn verify_structure<'a, F>(
        &'a self,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(RawCitationParsingError<'a>) -> ParsingError<'a>,
    {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.container_titles.len() {
            0 => {}
            1 => {
                let success = self.container_titles[0].verify_structure(errors);

                if !success {
                    found_error = true;
                }
            }
            _ => {
                found_error = true;

                errors.err(generate_error(RawCitationParsingError::ContainerError(
                    self,
                    RawCitationContainerParsingError::DuplicateTitle,
                )))
            }
        }

        match self.other_contributors.len() {
            0 => {}
            1 => {
                let success = self.other_contributors[0].verify_structure(errors);

                if !success {
                    found_error = true;
                }
            }
            _ => {
                found_error = true;

                errors.err(generate_error(RawCitationParsingError::ContainerError(
                    self,
                    RawCitationContainerParsingError::DuplicateOtherContributors,
                )))
            }
        }

        match self.versions.len() {
            0 => {}
            1 => {
                let success = self.versions[0].verify_structure(errors);

                if !success {
                    found_error = true;
                }
            }
            _ => {
                found_error = true;

                errors.err(generate_error(RawCitationParsingError::ContainerError(
                    self,
                    RawCitationContainerParsingError::DuplicateVersion,
                )))
            }
        }

        match self.numbers.len() {
            0 => {}
            1 => {
                let success = self.numbers[0].verify_structure(errors);

                if !success {
                    found_error = true;
                }
            }
            _ => {
                found_error = true;

                errors.err(generate_error(RawCitationParsingError::ContainerError(
                    self,
                    RawCitationContainerParsingError::DuplicateNumber,
                )))
            }
        }

        match self.publishers.len() {
            0 => {}
            1 => {
                let success = self.publishers[0].verify_structure(errors);

                if !success {
                    found_error = true;
                }
            }
            _ => {
                found_error = true;

                errors.err(generate_error(RawCitationParsingError::ContainerError(
                    self,
                    RawCitationContainerParsingError::DuplicatePublisher,
                )))
            }
        }

        match self.publication_dates.len() {
            0 => {}
            1 => {
                let success = self.publication_dates[0].verify_structure(errors);

                if !success {
                    found_error = true;
                }
            }
            _ => {
                found_error = true;

                errors.err(generate_error(RawCitationParsingError::ContainerError(
                    self,
                    RawCitationContainerParsingError::DuplicatePublicationDate,
                )))
            }
        }

        match self.locations.len() {
            0 => {}
            1 => {
                let success = self.locations[0].verify_structure(errors);

                if !success {
                    found_error = true;
                }
            }
            _ => {
                found_error = true;

                errors.err(generate_error(RawCitationParsingError::ContainerError(
                    self,
                    RawCitationContainerParsingError::DuplicateLocation,
                )));
            }
        }

        self.verified.set(!found_error);
        !found_error
    }

    // TODO: Remove
    fn finish(&self) -> MlaContainer {
        let container_title = self.container_title().map(UnformattedBuilder::finish);
        let other_contributors = self.other_contributors().map(UnformattedBuilder::finish);
        let version = self.version().map(UnformattedBuilder::finish);
        let number = self.number().map(UnformattedBuilder::finish);
        let publisher = self.publisher().map(UnformattedBuilder::finish);
        let publication_date = self.publication_date().map(UnformattedBuilder::finish);
        let location = self.location().map(UnformattedBuilder::finish);

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

    fn container_title(&self) -> Option<&UnformattedBuilder> {
        assert!(self.verified.get());
        self.container_titles.get(0)
    }

    fn other_contributors(&self) -> Option<&UnformattedBuilder> {
        assert!(self.verified.get());
        self.other_contributors.get(0)
    }

    fn version(&self) -> Option<&UnformattedBuilder> {
        assert!(self.verified.get());
        self.versions.get(0)
    }

    fn number(&self) -> Option<&UnformattedBuilder> {
        assert!(self.verified.get());
        self.numbers.get(0)
    }

    fn publisher(&self) -> Option<&UnformattedBuilder> {
        assert!(self.verified.get());
        self.publishers.get(0)
    }

    fn publication_date(&self) -> Option<&UnformattedBuilder> {
        assert!(self.verified.get());
        self.publication_dates.get(0)
    }

    fn location(&self) -> Option<&UnformattedBuilder> {
        assert!(self.verified.get());
        self.locations.get(0)
    }
}

#[derive(Debug)]
pub struct RawCitationBuilder {
    authors: Vec<UnformattedBuilder>,
    titles: Vec<UnformattedBuilder>,
    containers: Vec<RawCitationContainerBuilder>,

    verified: Cell<bool>,
}

impl RawCitationBuilder {
    pub fn from_pest_entries(pairs: Pairs<Rule>) -> RawCitationBuilder {
        let mut authors = Vec::with_capacity(1);
        let mut titles = Vec::with_capacity(1);
        let mut containers = Vec::new();

        for pair in pairs {
            match pair.as_rule() {
                Rule::raw_citation_authors => authors.push(UnformattedBuilder::from_pest(
                    pair.into_inner().next().unwrap(),
                )),
                Rule::raw_citation_title => titles.push(UnformattedBuilder::from_pest(
                    pair.into_inner().next().unwrap(),
                )),
                Rule::raw_citation_container => {
                    containers.push(RawCitationContainerBuilder::from_pest(pair.into_inner()))
                }

                _ => unreachable!(),
            }
        }

        RawCitationBuilder {
            authors,
            titles,
            containers,

            verified: Cell::new(false),
        }
    }

    pub fn from_pest(pair: Pair<Rule>) -> RawCitationBuilder {
        assert_eq!(pair.as_rule(), Rule::raw_citation);
        Self::from_pest_entries(pair.into_inner())
    }

    pub fn verify_structure<'a, F>(
        &'a self,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(RawCitationParsingError<'a>) -> ParsingError<'a> + Copy,
    {
        assert!(!self.verified.get());
        let mut found_error = false;

        match self.authors.len() {
            0 => {}
            1 => {
                let success = self.authors[0].verify_structure(errors);

                if !success {
                    found_error = true;
                }
            }
            _ => {
                found_error = true;
                errors.err(generate_error(RawCitationParsingError::DuplicateAuthor))
            }
        }

        match self.titles.len() {
            0 => {
                found_error = true;
                errors.err(generate_error(RawCitationParsingError::MissingTitle));
            }
            1 => {
                let success = self.titles[0].verify_structure(errors);

                if !success {
                    found_error = true;
                }
            }
            _ => {
                found_error = true;
                errors.err(generate_error(RawCitationParsingError::DuplicateTitle))
            }
        }

        for container in &self.containers {
            container.verify_structure(errors, generate_error);
        }

        self.verified.set(!found_error);

        !found_error
    }

    // TODO: Remove
    pub fn finish(&self) -> Mla {
        let author = self.author().map(UnformattedBuilder::finish);
        let title = self.title().finish();
        let containers = self
            .containers
            .iter()
            .map(RawCitationContainerBuilder::finish)
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

#[derive(Debug)]
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

    // TODO: Remove.
    fn finish(&self) -> SublistItem {
        let var_id = self.var_id.clone();
        let replacement = self.replacement.finish();

        SublistItem::new(var_id, replacement)
    }
}

// TODO: This could skip the building step and go straight to a Sublist.
#[derive(Debug)]
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

    // TODO: Remove.
    fn finish(&self) -> Sublist {
        let items = self.items.iter().map(SublistBuilderItem::finish).collect();

        Sublist::new(items)
    }
}

#[derive(Debug)]
pub enum MathBuilderElement {
    Fenced(MathBuilder),

    Operator(String),
    Symbol(String),
    Variable(String),
    Number(String),
}

impl MathBuilderElement {
    fn map_operator(operator: Rule) -> &'static str {
        match operator {
            Rule::operator_plus => "+",
            Rule::operator_minus => "-",
            Rule::operator_asterisk => "*",
            Rule::operator_slash => "/",

            Rule::operator_eq => "=",

            Rule::operator_negation => "\u{00AC}",
            Rule::operator_equiv => "\u{21D4}",
            Rule::operator_implies => "\u{21D2}",
            Rule::operator_and => "\u{2227}",
            Rule::operator_or => "\u{2228}",

            Rule::operator_bang => "!",

            _ => unreachable!(),
        }
    }

    fn from_pest(pair: Pair<Rule>) -> MathBuilderElement {
        match pair.as_rule() {
            Rule::math_row => Self::Fenced(MathBuilder::from_pest(pair)),

            Rule::display_operator => {
                let operator = pair.into_inner().next().unwrap().as_rule();
                let mapped_operator = Self::map_operator(operator);

                Self::Operator(mapped_operator.to_owned())
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

            Rule::read_operator => Self::Operator(
                Self::map_operator(pair.into_inner().next().unwrap().as_rule()).to_owned(),
            ),
            Rule::ident => Self::Symbol(map_ident(pair.as_str()).to_owned()),
            Rule::var => {
                Self::Variable(map_ident(pair.into_inner().next().unwrap().as_str()).to_owned())
            }

            _ => unreachable!(),
        }
    }

    // TODO: Remove.
    fn finish(&self) -> MathElement {
        match self {
            Self::Fenced(builder) => MathElement::Fenced(builder.finish()),

            Self::Operator(operator) => MathElement::Operator(operator.clone()),
            Self::Symbol(symbol) => MathElement::Symbol(symbol.clone()),
            Self::Variable(variable) => MathElement::Variable(variable.clone()),
            Self::Number(number) => MathElement::Number(number.clone()),
        }
    }
}

// TODO: This could skip the building step and go straight to a MathBlock.
#[derive(Debug)]
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

                _ => Box::new(std::iter::once(MathBuilderElement::from_pest_formula(pair)))
                    as Box<dyn Iterator<Item = MathBuilderElement>>,
            })
            .collect();

        MathBuilder { elements }
    }

    // TODO: Remove.
    pub fn finish(&self) -> MathBlock {
        let elements = self
            .elements
            .iter()
            .map(MathBuilderElement::finish)
            .collect();

        MathBlock::new(elements)
    }
}

// TODO: This could skip the building step and go straight to a DisplayMathBlock.
#[derive(Debug)]
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

    // TODO: Remove.
    fn finish(&self) -> DisplayMathBlock {
        let math = self.math.finish();
        let end = self.end.clone();

        DisplayMathBlock::new(math, end)
    }
}

struct SystemReferenceBuilder<'a> {
    id: String,

    system_ref: OnceCell<&'a SystemBuilder<'a>>,
}

impl<'a> SystemReferenceBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::ident);

        let id = pair.as_str().to_owned();

        SystemReferenceBuilder {
            id,

            system_ref: OnceCell::new(),
        }
    }

    fn verify_structure<F>(
        &self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match index.search_system(&self.id) {
            Some(system_ref) => {
                self.system_ref.set(system_ref).unwrap();
                true
            }

            None => {
                errors.err(generate_error(
                    ParagraphElementParsingError::SystemReferenceIdNotFound,
                ));
                false
            }
        }
    }

    // TODO: Remove.
    fn finish(&self) -> BlockReference {
        BlockReference::System(self.system_ref.get().unwrap().get_ref())
    }
}

impl<'a> std::fmt::Debug for SystemReferenceBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SystemReference").field(&self.id).finish()
    }
}

struct SystemChildReferenceBuilder<'a> {
    system_id: String,
    child_id: String,

    child_ref: OnceCell<SystemBuilderChild<'a>>,
}

impl<'a> SystemChildReferenceBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::fqid);

        let mut inner = pair.into_inner();
        let system_id = inner.next().unwrap().as_str().to_owned();
        let child_id = inner.next().unwrap().as_str().to_owned();

        SystemChildReferenceBuilder {
            system_id,
            child_id,

            child_ref: OnceCell::new(),
        }
    }

    fn verify_structure<F>(
        &self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match index.search_system_child(&self.system_id, &self.child_id) {
            Some(child_ref) => {
                self.child_ref.set(child_ref).unwrap();
                true
            }

            None => {
                errors.err(generate_error(
                    ParagraphElementParsingError::SystemChildReferenceIdNotFound,
                ));
                false
            }
        }
    }

    // TODO: Remove.
    fn finish(&self) -> BlockReference {
        self.child_ref.get().unwrap().get_ref()
    }
}

impl<'a> std::fmt::Debug for SystemChildReferenceBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SystemChildReference")
            .field(&self.system_id)
            .field(&self.child_id)
            .finish()
    }
}

struct TagReferenceBuilder<'a> {
    tag: String,

    step_ref: OnceCell<&'a ProofBuilderStep<'a>>,
}

impl<'a> TagReferenceBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::tag);

        let tag = pair.into_inner().next().unwrap().as_str().to_owned();

        TagReferenceBuilder {
            tag,

            step_ref: OnceCell::new(),
        }
    }

    fn verify_structure<F>(
        &self,
        tags: &HashMap<&str, &'a ProofBuilderStep<'a>>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match tags.get(self.tag.as_str()) {
            Some(step_ref) => {
                self.step_ref.set(*step_ref).unwrap();
                true
            }

            None => {
                errors.err(generate_error(
                    ParagraphElementParsingError::TagReferenceNotFound,
                ));
                false
            }
        }
    }

    fn finish(&self) -> BlockReference {
        BlockReference::ProofStep(self.step_ref.get().unwrap().get_ref())
    }
}

impl<'a> std::fmt::Debug for TagReferenceBuilder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TagReference").field(&self.tag).finish()
    }
}

#[derive(Debug)]
enum ReferenceBuilder<'a> {
    System(SystemReferenceBuilder<'a>),
    SystemChild(SystemChildReferenceBuilder<'a>),
    Tag(TagReferenceBuilder<'a>),
}

impl<'a> ReferenceBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
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
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match self {
            Self::System(r) => r.verify_structure(index, errors, generate_error),
            Self::SystemChild(r) => r.verify_structure(index, errors, generate_error),
            Self::Tag(_) => unreachable!(),
        }
    }

    fn verify_structure_with_tags<F>(
        &self,
        index: &BuilderIndex<'a>,
        tags: &HashMap<&str, &'a ProofBuilderStep<'a>>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match self {
            Self::System(r) => r.verify_structure(index, errors, generate_error),
            Self::SystemChild(r) => r.verify_structure(index, errors, generate_error),
            Self::Tag(tag) => tag.verify_structure(tags, errors, generate_error),
        }
    }

    // TODO: Remove.
    fn finish(&self) -> BlockReference {
        match self {
            Self::System(r) => r.finish(),
            Self::SystemChild(r) => r.finish(),
            Self::Tag(tag) => tag.finish(),
        }
    }
}

enum ParagraphFormattingState {
    None,
    Unicorn,
    Em,
}

impl ParagraphFormattingState {
    fn unicorn_begin<'a, F>(
        &mut self,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match self {
            Self::None => {
                *self = Self::Unicorn;
                true
            }

            _ => {
                errors.err(generate_error(
                    ParagraphElementParsingError::UnexpectedUnicornVomitBegin,
                ));
                false
            }
        }
    }

    fn unicorn_end<'a, F>(
        &mut self,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match self {
            Self::Unicorn => {
                *self = Self::None;
                true
            }

            _ => {
                errors.err(generate_error(
                    ParagraphElementParsingError::UnexpectedUnicornVomitEnd,
                ));
                false
            }
        }
    }

    fn em_begin<'a, F>(&mut self, errors: &mut ParsingErrorContext<'a>, generate_error: F) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match self {
            Self::None => {
                *self = Self::Em;
                true
            }

            _ => {
                errors.err(generate_error(
                    ParagraphElementParsingError::UnexpectedEmBegin,
                ));
                false
            }
        }
    }

    fn em_end<'a, F>(&mut self, errors: &mut ParsingErrorContext<'a>, generate_error: F) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match self {
            Self::Em => {
                *self = Self::None;
                true
            }

            _ => {
                errors.err(generate_error(
                    ParagraphElementParsingError::UnexpectedEmEnd,
                ));
                false
            }
        }
    }

    fn verify<'a, F>(self, errors: &mut ParsingErrorContext<'a>, generate_error: F) -> bool
    where
        F: Fn(ParagraphParsingError) -> ParsingError<'a>,
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

#[derive(Debug)]
enum ParagraphBuilderElement<'a> {
    Reference(ReferenceBuilder<'a>),
    InlineMath(MathBuilder),
    Citation(CitationBuilder<'a>),

    UnicornVomitBegin,
    UnicornVomitEnd,
    EmBegin,
    EmEnd,

    Unformatted(UnformattedBuilderElement),
}

impl<'a> ParagraphBuilderElement<'a> {
    fn from_pest(pair: Pair<Rule>, whitespace_rule: Rule) -> Self {
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
        index: &BuilderIndex<'a>,
        state: &mut ParagraphFormattingState,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match self {
            Self::Reference(r) => r.verify_structure(index, errors, generate_error),
            Self::InlineMath(_) => true,
            Self::Citation(citation) => citation.verify_structure(index, errors, generate_error),

            Self::UnicornVomitBegin => state.unicorn_begin(errors, generate_error),
            Self::UnicornVomitEnd => state.unicorn_end(errors, generate_error),
            Self::EmBegin => state.em_begin(errors, generate_error),
            Self::EmEnd => state.em_end(errors, generate_error),

            Self::Unformatted(builder) => builder.verify_structure(errors),
        }
    }

    fn verify_structure_with_tags<F>(
        &self,
        index: &BuilderIndex<'a>,
        tags: &HashMap<&str, &'a ProofBuilderStep<'a>>,
        state: &mut ParagraphFormattingState,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphElementParsingError) -> ParsingError<'a>,
    {
        match self {
            Self::Reference(r) => r.verify_structure_with_tags(index, tags, errors, generate_error),
            Self::InlineMath(_) => true,
            Self::Citation(citation) => citation.verify_structure(index, errors, generate_error),

            Self::UnicornVomitBegin => state.unicorn_begin(errors, generate_error),
            Self::UnicornVomitEnd => state.unicorn_end(errors, generate_error),
            Self::EmBegin => state.em_begin(errors, generate_error),
            Self::EmEnd => state.em_end(errors, generate_error),

            Self::Unformatted(builder) => builder.verify_structure(errors),
        }
    }

    fn bib_ref(&self) -> Option<&BibliographyBuilderEntry> {
        match self {
            Self::Citation(citation) => Some(citation.bib_ref()),

            _ => None,
        }
    }

    fn set_local_bib_refs(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        if let Self::Citation(citation) = self {
            citation.set_local_bib_ref(index);
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

#[derive(Debug)]
pub struct ParagraphBuilder<'a> {
    elements: Vec<ParagraphBuilderElement<'a>>,

    verified: Cell<bool>,
}

impl<'a> ParagraphBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>) -> Self {
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
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphParsingError) -> ParsingError<'a>,
    {
        assert!(!self.verified.get());

        let mut state = ParagraphFormattingState::None;

        for (i, element) in self.elements.iter().enumerate() {
            element.verify_structure(index, &mut state, errors, |e| {
                generate_error(ParagraphParsingError::ElementError(i, e))
            });
        }

        self.verified.set(state.verify(errors, generate_error));
        self.verified.get()
    }

    pub fn verify_structure_with_tags<F>(
        &self,
        index: &BuilderIndex<'a>,
        tags: &HashMap<&str, &'a ProofBuilderStep<'a>>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(ParagraphParsingError) -> ParsingError<'a>,
    {
        assert!(!self.verified.get());

        let mut state = ParagraphFormattingState::None;

        for (i, element) in self.elements.iter().enumerate() {
            element.verify_structure_with_tags(index, tags, &mut state, errors, |e| {
                generate_error(ParagraphParsingError::ElementError(i, e))
            });
        }

        self.verified.set(state.verify(errors, generate_error));
        self.verified.get()
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        assert!(self.verified.get());
        Box::new(
            self.elements
                .iter()
                .filter_map(ParagraphBuilderElement::bib_ref),
        )
    }

    pub fn set_local_bib_refs(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        for element in &self.elements {
            element.set_local_bib_refs(index);
        }
    }

    // TODO: Remove.
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

#[derive(Debug)]
struct TableBuilderRow<'a> {
    cells: Vec<ParagraphBuilder<'a>>,
}

impl<'a> TableBuilderRow<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::table_row);

        let cells = pair.into_inner().map(ParagraphBuilder::from_pest).collect();

        TableBuilderRow { cells }
    }

    fn verify_structure(
        &'a self,
        table_ref: &'a TableBuilder<'a>,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        for cell in &self.cells {
            cell.verify_structure(index, errors, |e| {
                ParsingError::TableError(table_ref, TableParsingError::CellError(cell, e))
            });
        }
    }

    fn bib_refs(&'a self) -> impl Iterator<Item = &BibliographyBuilderEntry> {
        self.cells.iter().flat_map(ParagraphBuilder::bib_refs)
    }

    fn set_local_bib_refs(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        for cell in &self.cells {
            cell.set_local_bib_refs(index)
        }
    }

    fn finish(&self) -> TableBlockRow {
        let cells = self.cells.iter().map(ParagraphBuilder::finish).collect();

        TableBlockRow::new(cells)
    }
}

#[derive(Debug)]
pub struct TableBuilder<'a> {
    head: Option<Vec<TableBuilderRow<'a>>>,
    body: Option<Vec<TableBuilderRow<'a>>>,
    foot: Option<Vec<TableBuilderRow<'a>>>,

    caption: Option<ParagraphBuilder<'a>>,

    // TODO: Remove.
    count: OnceCell<usize>,
}

impl<'a> TableBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>) -> Self {
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

            count: OnceCell::new(),
        }
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        let rows = self
            .head
            .iter()
            .chain(&self.body)
            .chain(&self.foot)
            .flatten();

        for row in rows {
            row.verify_structure(self, index, errors);
        }

        if let Some(caption) = &self.caption {
            caption.verify_structure(index, errors, |e| {
                ParsingError::TableError(self, TableParsingError::CaptionError(e))
            });
        }
    }

    pub fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        let head = self.head.iter().flatten();
        let body = self.body.iter().flatten();
        let foot = self.foot.iter().flatten();
        let rows = head.chain(body).chain(foot);

        let row_refs = rows.flat_map(TableBuilderRow::bib_refs);
        let caption_refs = self.caption.iter().flat_map(ParagraphBuilder::bib_refs);

        Box::new(row_refs.chain(caption_refs))
    }

    pub fn set_local_bib_refs(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        let head = self.head.iter().flatten();
        let body = self.body.iter().flatten();
        let foot = self.foot.iter().flatten();
        let rows = head.chain(body).chain(foot);

        for row in rows {
            row.set_local_bib_refs(index);
        }

        if let Some(caption) = self.caption.as_ref() {
            caption.set_local_bib_refs(index);
        }
    }

    // TODO: Remove.
    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap();
    }

    // TODO: Remove.
    pub fn get_ref(&self) -> TableBlockRef {
        TableBlockRef::new(*self.count.get().unwrap())
    }

    // TODO: Remove.
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

#[derive(Debug)]
struct QuoteValueBuilder<'a> {
    bib_key: String,
    quote: UnformattedBuilder,

    bib_ref: OnceCell<&'a BibliographyBuilderEntry>,
    local_bib_index: OnceCell<usize>,
}

impl<'a> QuoteValueBuilder<'a> {
    fn from_pest(pair: Pair<Rule>) -> Self {
        assert!(pair.as_rule() == Rule::quote_value || pair.as_rule() == Rule::quote_original);

        let mut inner = pair.into_inner();
        let bib_key = inner.next().unwrap().as_str().to_owned();
        let quote = UnformattedBuilder::from_pest(inner.next().unwrap());

        QuoteValueBuilder {
            bib_key,
            quote,

            bib_ref: OnceCell::new(),
            local_bib_index: OnceCell::new(),
        }
    }

    fn verify_structure<F>(
        &self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) where
        F: Fn(QuoteValueParsingError) -> ParsingError<'a>,
    {
        match index.search_bib_ref(&self.bib_key) {
            Some(bib_ref) => self.bib_ref.set(bib_ref).unwrap(),

            None => errors.err(generate_error(QuoteValueParsingError::BibKeyNotFound)),
        }
    }

    fn bib_ref(&self) -> &BibliographyBuilderEntry {
        self.bib_ref.get().unwrap()
    }

    fn set_local_bib_ref(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        let local_bib_ref = *index.get(self.bib_ref.get().unwrap()).unwrap();
        self.local_bib_index.set(local_bib_ref).unwrap();
    }

    // TODO: Remove.
    fn finish(&self) -> QuoteValue {
        let quote = self.quote.finish();
        let local_bib_index = *self.local_bib_index.get().unwrap();

        QuoteValue::new(quote, LocalBibliographyRef::new(local_bib_index))
    }
}

#[derive(Debug)]
pub struct QuoteBuilder<'a> {
    original: Option<QuoteValueBuilder<'a>>,
    value: QuoteValueBuilder<'a>,

    // TODO: Remove.
    count: OnceCell<usize>,
}

impl<'a> QuoteBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>) -> Self {
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

            count: OnceCell::new(),
        }
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        self.original.as_ref().map(|value| {
            value.verify_structure(index, errors, |e| {
                ParsingError::QuoteError(self, QuoteParsingError::OriginalError(e))
            })
        });

        self.value.verify_structure(index, errors, |e| {
            ParsingError::QuoteError(self, QuoteParsingError::ValueError(e))
        });
    }

    pub fn bib_refs(&'a self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        let original_ref = self.original.iter().map(QuoteValueBuilder::bib_ref);
        let value_ref = self.value.bib_ref();

        Box::new(original_ref.chain(Some(value_ref)))
    }

    pub fn set_local_bib_refs(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        if let Some(original) = &self.original {
            original.set_local_bib_ref(index);
        }

        self.value.set_local_bib_ref(index);
    }

    // TODO: Remove.
    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap();
    }

    // TODO: Remove.
    pub fn get_ref(&self) -> QuoteBlockRef {
        QuoteBlockRef::new(*self.count.get().unwrap())
    }

    // TODO: Remove.
    pub fn finish(&self) -> QuoteBlock {
        let original = self.original.as_ref().map(QuoteValueBuilder::finish);
        let value = self.value.finish();

        QuoteBlock::new(original, value)
    }
}

pub struct TodoBuilder<'a> {
    elements: Vec<TextBuilder<'a>>,

    // TODO: Remove.
    count: OnceCell<usize>,
}

impl<'a> TodoBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>) -> Self {
        let elements = pair.into_inner().map(TextBuilder::from_pest).collect();

        TodoBuilder {
            elements,

            count: OnceCell::new(),
        }
    }

    pub fn verify_structure(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
    ) {
        for element in &self.elements {
            element.verify_structure(index, errors, |e| ParsingError::TextError(element, e));
        }
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        Box::new(self.elements.iter().flat_map(TextBuilder::bib_refs))
    }

    pub fn set_local_bib_refs(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        for element in &self.elements {
            element.set_local_bib_refs(index);
        }
    }

    // TODO: Remove.
    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap();
    }

    // TODO: Remove.
    pub fn get_ref(&self) -> TodoBlockRef {
        TodoBlockRef::new(*self.count.get().unwrap())
    }

    // TODO: Remove.
    pub fn finish(&self) -> TodoBlock {
        let elements = self.elements.iter().map(TextBuilder::finish).collect();

        TodoBlock::new(elements)
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

    fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        for element in &self.contents {
            element.verify_structure(errors);
        }
    }

    // TODO: Remove.
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

    count: OnceCell<usize>,
}

impl HeadingBuilder {
    pub fn from_pest(pair: Pair<Rule>) -> HeadingBuilder {
        assert_eq!(pair.as_rule(), Rule::heading_block);

        let subheadings = pair
            .into_inner()
            .map(SubHeadingBuilder::from_pest)
            .collect();

        HeadingBuilder {
            subheadings,

            count: OnceCell::new(),
        }
    }

    pub fn verify_structure(&self, errors: &mut ParsingErrorContext) {
        for subheading in &self.subheadings {
            subheading.verify_structure(errors);
        }
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        Box::new(std::iter::empty())
    }

    // TODO: Remove.
    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap();
    }

    // TODO: Remove.
    pub fn get_ref(&self) -> HeadingBlockRef {
        HeadingBlockRef::new(*self.count.get().unwrap())
    }

    // TODO: Remove.
    pub fn finish(&self) -> HeadingBlock {
        let subheadings = self
            .subheadings
            .iter()
            .map(SubHeadingBuilder::finish)
            .collect();

        HeadingBlock::new(subheadings)
    }
}

#[derive(Debug)]
pub enum TextBuilder<'a> {
    RawCitation(RawCitationBuilder),
    Sublist(SublistBuilder),
    DisplayMath(DisplayMathBuilder),
    Paragraph(ParagraphBuilder<'a>),
}

impl<'a> TextBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>) -> Self {
        assert_eq!(pair.as_rule(), Rule::text_block);
        let pair = pair.into_inner().next().unwrap();

        match pair.as_rule() {
            Rule::raw_citation => Self::RawCitation(RawCitationBuilder::from_pest(pair)),
            Rule::sublist => Self::Sublist(SublistBuilder::from_pest(pair)),
            Rule::display_math => Self::DisplayMath(DisplayMathBuilder::from_pest(pair)),
            Rule::paragraph => Self::Paragraph(ParagraphBuilder::from_pest(pair)),

            _ => unreachable!(),
        }
    }

    pub fn verify_structure<F>(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(TextParsingError<'a>) -> ParsingError<'a>,
    {
        match self {
            Self::RawCitation(citation) => citation.verify_structure(errors, |e| {
                generate_error(TextParsingError::RawCitationError(e))
            }),
            Self::Paragraph(paragraph) => paragraph.verify_structure(index, errors, |e| {
                generate_error(TextParsingError::ParagraphError(e))
            }),

            Self::Sublist(_) | Self::DisplayMath(_) => true,
        }
    }

    pub fn verify_structure_with_tags<F>(
        &'a self,
        index: &BuilderIndex<'a>,
        tags: &HashMap<&str, &'a ProofBuilderStep<'a>>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(TextParsingError<'a>) -> ParsingError<'a>,
    {
        match self {
            Self::RawCitation(citation) => citation.verify_structure(errors, |e| {
                generate_error(TextParsingError::RawCitationError(e))
            }),
            Self::Paragraph(paragraph) => {
                paragraph.verify_structure_with_tags(index, tags, errors, |e| {
                    generate_error(TextParsingError::ParagraphError(e))
                })
            }

            Self::Sublist(_) | Self::DisplayMath(_) => true,
        }
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        match self {
            Self::Paragraph(paragraph) => paragraph.bib_refs(),
            _ => Box::new(std::iter::empty()),
        }
    }

    pub fn set_local_bib_refs(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        match self {
            Self::Paragraph(paragraph) => paragraph.set_local_bib_refs(index),

            _ => {}
        }
    }

    // TODO: Remove.
    pub fn finish(&self) -> Text {
        match self {
            Self::RawCitation(raw_citation) => Text::Mla(raw_citation.finish()),
            Self::Sublist(sublist) => Text::Sublist(sublist.finish()),
            Self::DisplayMath(display_math) => Text::DisplayMath(display_math.finish()),
            Self::Paragraph(paragraph) => Text::Paragraph(paragraph.finish()),
        }
    }
}

// TODO: Remove.
#[derive(Debug)]
pub struct TextBlockBuilder<'a> {
    text: TextBuilder<'a>,
    count: OnceCell<usize>,
}

impl<'a> TextBlockBuilder<'a> {
    pub fn from_pest(pair: Pair<Rule>) -> Self {
        TextBlockBuilder {
            text: TextBuilder::from_pest(pair),
            count: OnceCell::new(),
        }
    }

    pub fn verify_structure<F>(
        &'a self,
        index: &BuilderIndex<'a>,
        errors: &mut ParsingErrorContext<'a>,
        generate_error: F,
    ) -> bool
    where
        F: Fn(TextParsingError<'a>) -> ParsingError<'a>,
    {
        self.text.verify_structure(index, errors, generate_error)
    }

    pub fn bib_refs(&self) -> Box<dyn Iterator<Item = &BibliographyBuilderEntry> + '_> {
        self.text.bib_refs()
    }

    pub fn set_local_bib_refs(&self, index: &HashMap<&BibliographyBuilderEntry, usize>) {
        self.text.set_local_bib_refs(index)
    }

    pub fn text(&'a self) -> &TextBuilder {
        &self.text
    }

    pub fn count(&self, count: usize) {
        self.count.set(count).unwrap();
    }

    pub fn get_ref(&self) -> TextBlockRef {
        TextBlockRef::new(*self.count.get().unwrap())
    }

    pub fn finish(&self) -> TextBlock {
        TextBlock::new(self.text.finish())
    }
}
