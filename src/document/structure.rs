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

use crate::deduction::directory::CheckableDirectory;

use crate::rendered::{BookRendered, ChapterRendered, DocumentRendered, PageRendered};

use super::directory::{Block, BlockDirectory, LocalBibliography};
use super::text::Paragraph;

pub struct Page {
    id: String,
    name: String,
    href: String,

    blocks: Vec<Block>,

    local_bibliography: Option<LocalBibliography>,
}

impl Page {
    pub fn new(
        id: String,
        name: String,
        href: String,
        blocks: Vec<Block>,
        local_bibliography: Option<LocalBibliography>,
    ) -> Page {
        Page {
            id,
            name,
            href,

            blocks,

            local_bibliography,
        }
    }

    fn render(
        &self,
        chapter_num: usize,
        chapter_name: &str,
        page_num: usize,
        prev_href: &str,
        up_href: &str,
        next_href: Option<&str>,
        directory: &BlockDirectory,
    ) -> PageRendered {
        let id = self.id.clone();
        let page_name = self.name.clone();
        let href = self.href.clone();

        let chapter_name = chapter_name.to_owned();

        let blocks = self
            .blocks
            .iter()
            .map(|block| block.render(directory))
            .collect();

        let local_bibliography = self
            .local_bibliography
            .as_ref()
            .map(|local_bibliography| local_bibliography.render(directory));

        PageRendered::new(
            id,
            href,
            page_num,
            chapter_num,
            page_name,
            chapter_name,
            prev_href.to_owned(),
            up_href.to_owned(),
            next_href.map(str::to_owned),
            blocks,
            local_bibliography,
        )
    }
}

pub struct Chapter {
    id: String,
    name: String,
    href: String,
    tagline: Paragraph,

    pages: Vec<Page>,
}

impl Chapter {
    pub fn new(
        id: String,
        name: String,
        href: String,
        tagline: Paragraph,
        pages: Vec<Page>,
    ) -> Chapter {
        Chapter {
            id,
            name,
            href,
            tagline,

            pages,
        }
    }

    fn render(
        &self,
        chapter_num: usize,
        next_chapter_href: Option<&str>,
        directory: &BlockDirectory,
    ) -> ChapterRendered {
        let id = self.id.clone();
        let href = self.href.clone();
        let chapter_name = self.name.clone();
        let tagline = self.tagline.render(directory);

        let mut prev_href = &href;
        let up_href = &href;
        let mut next_href = None;
        let pages = (0..self.pages.len())
            .map(|page_num| {
                let page = &self.pages[page_num];

                next_href = self
                    .pages
                    .get(page_num + 1)
                    .map(|page| page.href.as_ref())
                    .or(next_chapter_href);

                let ret = page.render(
                    chapter_num,
                    &chapter_name,
                    page_num,
                    prev_href,
                    up_href,
                    next_href,
                    directory,
                );

                prev_href = &page.href;

                ret
            })
            .collect();

        ChapterRendered::new(id, href, chapter_num, chapter_name, tagline, pages)
    }
}

pub struct Book {
    id: String,
    name: String,
    href: String,
    tagline: Paragraph,

    chapters: Vec<Chapter>,
}

impl Book {
    pub fn new(
        id: String,
        name: String,
        href: String,
        tagline: Paragraph,
        chapters: Vec<Chapter>,
    ) -> Book {
        Book {
            id,
            name,
            href,
            tagline,

            chapters,
        }
    }

    fn render(&self, book_num: usize, directory: &BlockDirectory) -> BookRendered {
        let id = self.id.clone();
        let href = self.href.clone();
        let name = self.name.clone();
        let tagline = self.tagline.render(directory);

        let chapters = (0..self.chapters.len())
            .map(|chapter_num| {
                let chapter = &self.chapters[chapter_num];

                let next_chapter_href = self
                    .chapters
                    .get(chapter_num + 1)
                    .map(|chapter| chapter.href.as_ref());

                chapter.render(chapter_num, next_chapter_href, directory)
            })
            .collect();

        BookRendered::new(id, href, book_num, name, tagline, chapters)
    }
}

pub struct Document {
    books: Vec<Book>,

    directory: BlockDirectory,
}

impl Document {
    pub fn new(books: Vec<Book>, directory: BlockDirectory) -> Document {
        Document { books, directory }
    }

    pub fn checkable(&self) -> CheckableDirectory {
        self.directory.checkable()
    }

    pub fn render(&self) -> DocumentRendered {
        let books = self
            .books
            .iter()
            .enumerate()
            .map(|(book_num, book)| book.render(book_num, &self.directory))
            .collect();

        DocumentRendered::new(books)
    }
}
