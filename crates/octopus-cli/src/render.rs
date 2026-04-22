use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

use crossterm::cursor::{MoveToColumn, RestorePosition, SavePosition};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor, Stylize};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{execute, queue};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorTheme {
    heading: Color,
    emphasis: Color,
    strong: Color,
    inline_code: Color,
    link: Color,
    quote: Color,
    table_border: Color,
    code_block_border: Color,
    spinner_active: Color,
    spinner_done: Color,
    spinner_failed: Color,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            heading: Color::Cyan,
            emphasis: Color::Magenta,
            strong: Color::Yellow,
            inline_code: Color::Green,
            link: Color::Blue,
            quote: Color::DarkGrey,
            table_border: Color::DarkCyan,
            code_block_border: Color::DarkGrey,
            spinner_active: Color::Blue,
            spinner_done: Color::Green,
            spinner_failed: Color::Red,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Spinner {
    frame_index: usize,
}

impl Spinner {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(
        &mut self,
        label: &str,
        theme: &ColorTheme,
        out: &mut impl Write,
    ) -> io::Result<()> {
        let frame = Self::FRAMES[self.frame_index % Self::FRAMES.len()];
        self.frame_index += 1;
        queue!(
            out,
            SavePosition,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(theme.spinner_active),
            Print(format!("{frame} {label}")),
            ResetColor,
            RestorePosition
        )?;
        out.flush()
    }

    pub fn finish(
        &mut self,
        label: &str,
        theme: &ColorTheme,
        out: &mut impl Write,
    ) -> io::Result<()> {
        self.frame_index = 0;
        execute!(
            out,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(theme.spinner_done),
            Print(format!("✔ {label}\n")),
            ResetColor
        )?;
        out.flush()
    }

    pub fn fail(
        &mut self,
        label: &str,
        theme: &ColorTheme,
        out: &mut impl Write,
    ) -> io::Result<()> {
        self.frame_index = 0;
        execute!(
            out,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(theme.spinner_failed),
            Print(format!("✘ {label}\n")),
            ResetColor
        )?;
        out.flush()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ListKind {
    Unordered,
    Ordered { next_index: u64 },
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct TableState {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    in_head: bool,
}

impl TableState {
    fn push_cell(&mut self) {
        let cell = self.current_cell.trim().to_string();
        self.current_row.push(cell);
        self.current_cell.clear();
    }

    fn finish_row(&mut self) {
        if self.current_row.is_empty() {
            return;
        }
        let row = std::mem::take(&mut self.current_row);
        if self.in_head {
            self.headers = row;
        } else {
            self.rows.push(row);
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RenderState {
    emphasis: usize,
    strong: usize,
    heading_level: Option<u8>,
    quote: usize,
    list_stack: Vec<ListKind>,
    link_stack: Vec<LinkState>,
    table: Option<TableState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkState {
    destination: String,
    text: String,
}

impl RenderState {
    fn style_text(&self, text: &str, theme: &ColorTheme) -> String {
        let mut style = text.stylize();
        if matches!(self.heading_level, Some(1 | 2)) || self.strong > 0 {
            style = style.bold();
        }
        if self.emphasis > 0 {
            style = style.italic();
        }
        if let Some(level) = self.heading_level {
            style = match level {
                1 => style.with(theme.heading),
                2 => style.white(),
                3 => style.with(Color::Blue),
                _ => style.with(Color::Grey),
            };
        } else if self.strong > 0 {
            style = style.with(theme.strong);
        } else if self.emphasis > 0 {
            style = style.with(theme.emphasis);
        }
        if self.quote > 0 {
            style = style.with(theme.quote);
        }
        format!("{style}")
    }

    fn append_raw(&mut self, output: &mut String, text: &str) {
        if let Some(link) = self.link_stack.last_mut() {
            link.text.push_str(text);
        } else if let Some(table) = self.table.as_mut() {
            table.current_cell.push_str(text);
        } else {
            output.push_str(text);
        }
    }

    fn append_styled(&mut self, output: &mut String, text: &str, theme: &ColorTheme) {
        let styled = self.style_text(text, theme);
        self.append_raw(output, &styled);
    }
}

#[derive(Debug)]
pub struct TerminalRenderer {
    syntax_set: SyntaxSet,
    syntax_theme: Theme,
    color_theme: ColorTheme,
}

impl Default for TerminalRenderer {
    fn default() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax_theme = ThemeSet::load_defaults()
            .themes
            .remove("base16-ocean.dark")
            .unwrap_or_default();
        Self {
            syntax_set,
            syntax_theme,
            color_theme: ColorTheme::default(),
        }
    }
}

impl TerminalRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn color_theme(&self) -> &ColorTheme {
        &self.color_theme
    }

    #[must_use]
    pub fn render_markdown(&self, markdown: &str) -> String {
        let mut output = String::new();
        let mut state = RenderState::default();
        let mut code_language = String::new();
        let mut code_buffer = String::new();
        let mut in_code_block = false;

        for event in Parser::new_ext(markdown, Options::all()) {
            self.render_event(
                event,
                &mut output,
                &mut state,
                &mut code_language,
                &mut code_buffer,
                &mut in_code_block,
            );
        }

        output
    }

    #[allow(clippy::too_many_arguments)]
    fn render_event(
        &self,
        event: Event<'_>,
        output: &mut String,
        state: &mut RenderState,
        code_language: &mut String,
        code_buffer: &mut String,
        in_code_block: &mut bool,
    ) {
        match event {
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => output.push('\n'),
            Event::Start(Tag::Heading { level, .. }) => {
                if !output.ends_with('\n') && !output.is_empty() {
                    output.push('\n');
                }
                state.heading_level = Some(level as u8);
            }
            Event::End(TagEnd::Heading(_)) => {
                state.heading_level = None;
                output.push('\n');
            }
            Event::Text(text) => {
                if *in_code_block {
                    code_buffer.push_str(&text);
                } else {
                    state.append_styled(output, &text, &self.color_theme);
                }
            }
            Event::Code(text) => {
                let rendered = format!("{}", text.with(self.color_theme.inline_code));
                state.append_raw(output, &rendered);
            }
            Event::SoftBreak | Event::HardBreak => output.push('\n'),
            Event::Start(Tag::Emphasis) => state.emphasis += 1,
            Event::End(TagEnd::Emphasis) => state.emphasis = state.emphasis.saturating_sub(1),
            Event::Start(Tag::Strong) => state.strong += 1,
            Event::End(TagEnd::Strong) => state.strong = state.strong.saturating_sub(1),
            Event::Start(Tag::BlockQuote(_)) => state.quote += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                state.quote = state.quote.saturating_sub(1);
                output.push('\n');
            }
            Event::Start(Tag::List(start)) => {
                state.list_stack.push(match start {
                    Some(next_index) => ListKind::Ordered { next_index },
                    None => ListKind::Unordered,
                });
            }
            Event::End(TagEnd::List(_)) => {
                state.list_stack.pop();
                output.push('\n');
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(state.list_stack.len().saturating_sub(1));
                let marker = match state.list_stack.last_mut() {
                    Some(ListKind::Ordered { next_index }) => {
                        let marker = format!("{next_index}. ");
                        *next_index += 1;
                        marker
                    }
                    _ => "- ".to_string(),
                };
                output.push_str(&indent);
                output.push_str(&marker);
            }
            Event::End(TagEnd::Item) => output.push('\n'),
            Event::Start(Tag::Link { dest_url, .. }) => {
                state.link_stack.push(LinkState {
                    destination: dest_url.to_string(),
                    text: String::new(),
                });
            }
            Event::End(TagEnd::Link) => {
                if let Some(link) = state.link_stack.pop() {
                    let _ = write!(
                        output,
                        "{} ({})",
                        link.text,
                        link.destination.as_str().with(self.color_theme.link)
                    );
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                *in_code_block = true;
                code_language.clear();
                code_buffer.clear();
                if let CodeBlockKind::Fenced(language) = kind {
                    code_language.push_str(&language);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                *in_code_block = false;
                output.push_str(&self.render_code_block(code_language, code_buffer));
                code_language.clear();
                code_buffer.clear();
            }
            Event::Rule => output.push_str("\n---\n"),
            Event::Start(Tag::Table(_)) => state.table = Some(TableState::default()),
            Event::End(TagEnd::Table) => {
                if let Some(table) = state.table.take() {
                    output.push_str(&render_table(&table, self.color_theme.table_border));
                }
            }
            Event::Start(Tag::TableHead) => {
                if let Some(table) = state.table.as_mut() {
                    table.in_head = true;
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(table) = state.table.as_mut() {
                    table.push_cell();
                    table.finish_row();
                    table.in_head = false;
                }
            }
            Event::Start(Tag::TableRow) => {}
            Event::End(TagEnd::TableRow) => {
                if let Some(table) = state.table.as_mut() {
                    table.push_cell();
                    table.finish_row();
                }
            }
            Event::Start(Tag::TableCell) => {}
            Event::End(TagEnd::TableCell) => {
                if let Some(table) = state.table.as_mut() {
                    table.push_cell();
                }
            }
            Event::Html(html) | Event::InlineHtml(html) => state.append_raw(output, &html),
            Event::FootnoteReference(text) => state.append_raw(output, &format!("[{text}]")),
            Event::TaskListMarker(checked) => {
                state.append_raw(output, if checked { "[x] " } else { "[ ] " })
            }
            _ => {}
        }
    }

    fn render_code_block(&self, language: &str, source: &str) -> String {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let mut highlighter = HighlightLines::new(syntax, &self.syntax_theme);
        let mut output = String::new();
        output.push('\n');
        for line in LinesWithEndings::from(source) {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();
            output.push_str(&as_24_bit_terminal_escaped(&ranges[..], false));
        }
        output.push('\n');
        output
    }
}

fn render_table(table: &TableState, border: Color) -> String {
    let mut rows = Vec::new();
    if !table.headers.is_empty() {
        rows.push(table.headers.clone());
    }
    rows.extend(table.rows.clone());
    if rows.is_empty() {
        return String::new();
    }

    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths = (0..column_count)
        .map(|index| {
            rows.iter()
                .filter_map(|row| row.get(index))
                .map(|cell| cell.len())
                .max()
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();

    let border_line = format!(
        "{}\n",
        widths
            .iter()
            .map(|width| format!("+{}", "-".repeat(width + 2)))
            .collect::<String>()
            + "+"
    );
    let mut output = String::new();
    output.push_str(&format!("{}", border_line.as_str().with(border)));
    for (index, row) in rows.iter().enumerate() {
        output.push('|');
        for (cell, width) in row.iter().zip(widths.iter()) {
            let _ = write!(output, " {:width$} |", cell, width = width);
        }
        output.push('\n');
        if index == 0 && !table.headers.is_empty() {
            output.push_str(&format!("{}", border_line.as_str().with(border)));
        }
    }
    output.push_str(&format!("{}", border_line.as_str().with(border)));
    output
}

#[cfg(test)]
mod tests {
    use super::TerminalRenderer;

    #[test]
    fn renders_markdown_headings_and_lists() {
        let renderer = TerminalRenderer::new();
        let rendered = renderer.render_markdown("# Title\n\n- one\n- two");
        assert!(rendered.contains("Title"));
        assert!(rendered.contains("- one"));
    }
}
