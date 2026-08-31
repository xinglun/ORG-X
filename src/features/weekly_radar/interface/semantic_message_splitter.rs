//! Source-preserving semantic splitting for already-rendered Telegram Markdown.
//!
//! The splitter only recognizes top-level rendered sections. It never parses
//! domain facts, rewrites Markdown, or sends a message to an external system.

use std::fmt;

/// Semantic boundary assigned to one complete rendered section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticBoundary {
    /// The rendered title or explicit No Change statement.
    ExecutiveSummary,
    /// Important Structural Change and Stage Transition sections.
    ImportantTransition,
    /// Top5 and Threshold Distance sections.
    Top5,
    /// Rising and Dropped sections.
    RisingDropped,
    /// The independent system and human reference section.
    JudgmentReference,
    /// System Health section.
    SystemHealth,
}

impl SemanticBoundary {
    /// Returns the stable human-readable boundary label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExecutiveSummary => "Executive Summary",
            Self::ImportantTransition => "Important Transition",
            Self::Top5 => "Top5",
            Self::RisingDropped => "Rising/Dropped",
            Self::JudgmentReference => "Judgment Reference",
            Self::SystemHealth => "System Health",
        }
    }
}

/// Caller-supplied atomic chunk limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSplitLimits {
    max_characters: usize,
    max_lines: usize,
}

impl SemanticSplitLimits {
    /// Creates non-zero limits for complete semantic chunks.
    pub const fn new(max_characters: usize, max_lines: usize) -> Result<Self, SemanticSplitError> {
        if max_characters == 0 {
            return Err(SemanticSplitError::InvalidLimit {
                field: "max characters",
            });
        }
        if max_lines == 0 {
            return Err(SemanticSplitError::InvalidLimit { field: "max lines" });
        }
        Ok(Self {
            max_characters,
            max_lines,
        })
    }

    /// Returns the character limit.
    pub const fn max_characters(self) -> usize {
        self.max_characters
    }

    /// Returns the line limit.
    pub const fn max_lines(self) -> usize {
        self.max_lines
    }
}

/// A complete rendered semantic chunk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticMessageChunk {
    boundary: SemanticBoundary,
    markdown: String,
    character_count: usize,
    line_count: usize,
}

impl SemanticMessageChunk {
    fn new(boundary: SemanticBoundary, markdown: String) -> Self {
        let character_count = markdown.chars().count();
        let line_count = markdown.lines().count();
        Self {
            boundary,
            markdown,
            character_count,
            line_count,
        }
    }

    /// Returns the semantic boundary label.
    pub const fn boundary(&self) -> SemanticBoundary {
        self.boundary
    }

    /// Returns the exact source Markdown retained by this chunk.
    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    /// Returns the Unicode scalar character count.
    pub const fn character_count(&self) -> usize {
        self.character_count
    }

    /// Returns the source line count.
    pub const fn line_count(&self) -> usize {
        self.line_count
    }
}

/// Ordered result of semantic splitting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticMessageSplit {
    chunks: Vec<SemanticMessageChunk>,
}

impl SemanticMessageSplit {
    /// Returns complete chunks in source order.
    pub fn chunks(&self) -> &[SemanticMessageChunk] {
        &self.chunks
    }

    /// Returns whether the split contains no chunks.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}

/// Deterministic errors for source-preserving semantic splitting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticSplitError {
    /// The rendered source was empty or whitespace-only.
    EmptyMessage,
    /// A caller supplied a zero limit.
    InvalidLimit { field: &'static str },
    /// A top-level heading is not one of the rendered section aliases.
    UnknownSection { heading: String },
    /// A fenced Markdown block was opened but not closed.
    UnclosedCodeFence,
    /// One complete section cannot fit within the caller's atomic limits.
    AtomicSectionTooLarge {
        boundary: SemanticBoundary,
        characters: usize,
        lines: usize,
    },
}

impl fmt::Display for SemanticSplitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMessage => formatter.write_str("rendered message cannot be empty"),
            Self::InvalidLimit { field } => write!(formatter, "{field} must be greater than zero"),
            Self::UnknownSection { heading } => {
                write!(formatter, "unknown rendered section {heading}")
            }
            Self::UnclosedCodeFence => {
                formatter.write_str("rendered message has an unclosed code fence")
            }
            Self::AtomicSectionTooLarge {
                boundary,
                characters,
                lines,
            } => write!(
                formatter,
                "complete {} section is too large: {characters} characters and {lines} lines",
                boundary.as_str()
            ),
        }
    }
}

impl std::error::Error for SemanticSplitError {}

#[derive(Clone, Copy)]
struct RawSection<'a> {
    boundary: SemanticBoundary,
    markdown: &'a str,
}

fn fence_marker(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn top_level_heading(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("## ") && !trimmed.starts_with("### ") {
        Some(trimmed[3..].trim())
    } else {
        None
    }
}

fn boundary_for_heading(heading: &str) -> Result<SemanticBoundary, SemanticSplitError> {
    match heading {
        "Important Structural Change"
        | "Important Organizational Changes"
        | "重要组织变化"
        | "重要な組織変化"
        | "Structural Evidence"
        | "结构性证据"
        | "構造的証拠"
        | "Structural Change Evidence"
        | "结构性变化证据"
        | "構造的変化の根拠"
        | "Stage Transition" => Ok(SemanticBoundary::ImportantTransition),
        "Top5" | "Companies to Watch" | "重点公司" | "注目企業" | "Threshold Distance" => {
            Ok(SemanticBoundary::Top5)
        }
        "Rising" | "Dropped" => Ok(SemanticBoundary::RisingDropped),
        "System Reference Judgment"
        | "系统参考判断"
        | "システム参考判断"
        | "AI 时代范本验证"
        | "AI 時代の参照モデル検証"
        | "AI-era Reference Model Validation" => Ok(SemanticBoundary::JudgmentReference),
        "System Health" | "系统状态" | "システム状態" => {
            Ok(SemanticBoundary::SystemHealth)
        }
        "Executive Summary"
        | "本周摘要"
        | "已确认信息"
        | "Validated Facts"
        | "已验证事实"
        | "検証済み事実"
        | "週次サマリー"
        | "確認済み情報"
        | "No Change"
        | "Confirmed Information" => Ok(SemanticBoundary::ExecutiveSummary),
        value if value.starts_with("No Change (") => Ok(SemanticBoundary::ExecutiveSummary),
        value => Err(SemanticSplitError::UnknownSection {
            heading: value.to_owned(),
        }),
    }
}

fn raw_sections(source: &str) -> Result<Vec<RawSection<'_>>, SemanticSplitError> {
    let mut starts = Vec::new();
    let mut headings = Vec::new();
    let mut offset = 0usize;
    let mut fenced = false;

    for line_with_ending in source.split_inclusive('\n') {
        let line = line_with_ending
            .strip_suffix('\n')
            .unwrap_or(line_with_ending)
            .strip_suffix('\r')
            .unwrap_or_else(|| {
                line_with_ending
                    .strip_suffix('\n')
                    .unwrap_or(line_with_ending)
            });
        if !fenced {
            if let Some(heading) = top_level_heading(line) {
                starts.push(offset);
                headings.push(heading.to_owned());
            }
        }
        if fence_marker(line) {
            fenced = !fenced;
        }
        offset += line_with_ending.len();
    }
    if !source.ends_with('\n') {
        let line = source.rsplit('\n').next().unwrap_or(source);
        if !fenced {
            if let Some(heading) = top_level_heading(line) {
                if starts.last().copied() != Some(source.len() - line.len()) {
                    starts.push(source.len() - line.len());
                    headings.push(heading.to_owned());
                }
            }
        }
        if fence_marker(line) {
            fenced = !fenced;
        }
    }
    if fenced {
        return Err(SemanticSplitError::UnclosedCodeFence);
    }

    let mut sections = Vec::new();
    let mut boundaries = Vec::with_capacity(headings.len());
    for heading in &headings {
        boundaries.push(boundary_for_heading(heading)?);
    }
    let mut all_starts = Vec::with_capacity(starts.len() + 1);
    all_starts.push(0);
    all_starts.extend(starts);
    for index in 0..all_starts.len() {
        let end = all_starts.get(index + 1).copied().unwrap_or(source.len());
        let markdown = &source[all_starts[index]..end];
        let boundary = if index == 0 {
            SemanticBoundary::ExecutiveSummary
        } else {
            boundaries[index - 1]
        };
        if !markdown.trim().is_empty() {
            sections.push(RawSection { boundary, markdown });
        }
    }
    Ok(sections)
}

/// Stateless source-preserving semantic splitter.
pub struct SemanticMessageSplitter;

impl SemanticMessageSplitter {
    /// Splits a rendered Markdown message only between complete top-level sections.
    pub fn split(
        source: &str,
        limits: SemanticSplitLimits,
    ) -> Result<SemanticMessageSplit, SemanticSplitError> {
        if source.trim().is_empty() {
            return Err(SemanticSplitError::EmptyMessage);
        }
        let sections = raw_sections(source)?;
        let mut chunks = Vec::new();

        for section in sections {
            let characters = section.markdown.chars().count();
            let lines = section.markdown.lines().count();
            if characters > limits.max_characters || lines > limits.max_lines {
                return Err(SemanticSplitError::AtomicSectionTooLarge {
                    boundary: section.boundary,
                    characters,
                    lines,
                });
            }

            let can_append = chunks.last().is_some_and(|chunk: &SemanticMessageChunk| {
                chunk.boundary == section.boundary
                    && chunk.character_count + characters <= limits.max_characters
                    && chunk.line_count + lines <= limits.max_lines
            });
            if can_append {
                let chunk = chunks.last_mut().expect("checked above");
                chunk.markdown.push_str(section.markdown);
                chunk.character_count += characters;
                chunk.line_count += lines;
            } else {
                chunks.push(SemanticMessageChunk::new(
                    section.boundary,
                    section.markdown.to_owned(),
                ));
            }
        }

        Ok(SemanticMessageSplit { chunks })
    }
}

#[cfg(test)]
#[path = "semantic_message_splitter_test.rs"]
mod module_tests;
