use std::collections::HashMap;
use std::ops::Range;

use sequoia_core::{Color, Rect};

use crate::{FontStyle, FontWeight, TextSystem};

pub const INLINE_OBJECT_CHAR: char = '\u{FFFC}';

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u64);

impl BlockId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineElement {
    Tag(String),
    Mention(String),
    Math(String),
    Image { url: String, alt: String },
    Status(String),
    Icon(String),
}

impl InlineElement {
    pub fn label(&self) -> &str {
        match self {
            Self::Tag(label)
            | Self::Mention(label)
            | Self::Math(label)
            | Self::Status(label)
            | Self::Icon(label) => label,
            Self::Image { alt, url } => {
                if alt.is_empty() {
                    url
                } else {
                    alt
                }
            }
        }
    }

    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image { .. })
    }

    pub fn is_inline(&self) -> bool {
        !self.is_image()
    }

    pub fn approximate_width(&self, font_size: f32) -> f32 {
        match self {
            Self::Image { .. } => font_size * 2.0,
            Self::Icon(_) => font_size * 1.25,
            Self::Math(label) => font_size * 0.55 * label.chars().count() as f32 + font_size * 0.7,
            Self::Tag(label) | Self::Mention(label) | Self::Status(label) => {
                font_size * 0.55 * label.chars().count() as f32 + font_size * 0.9
            }
        }
    }

    pub fn height(&self, font_size: f32) -> f32 {
        if self.is_image() {
            font_size * 1.8
        } else {
            font_size * 1.35
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineContent {
    Text(String),
    Element(InlineElement),
}

impl InlineContent {
    pub fn len_chars(&self) -> usize {
        match self {
            Self::Text(text) => text.chars().count(),
            Self::Element(_) => 1,
        }
    }

    pub fn element(&self) -> Option<&InlineElement> {
        match self {
            Self::Element(element) => Some(element),
            Self::Text(_) => None,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Element(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadingLevel {
    H1 = 1,
    H2 = 2,
    H3 = 3,
    H4 = 4,
    H5 = 5,
    H6 = 6,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum BlockType {
    #[default]
    Paragraph,
    Heading(HeadingLevel),
    BulletListItem,
    NumberedListItem {
        number: usize,
    },
    TodoItem {
        checked: bool,
    },
    Quote,
    Code {
        language: String,
    },
    Callout {
        icon: String,
    },
    Divider,
    Table {
        rows: usize,
        cols: usize,
    },
}

impl BlockType {
    pub fn is_list_item(&self) -> bool {
        matches!(
            self,
            Self::BulletListItem | Self::NumberedListItem { .. } | Self::TodoItem { .. }
        )
    }

    pub fn continuation(&self) -> Self {
        match self {
            Self::BulletListItem => Self::BulletListItem,
            Self::NumberedListItem { number } => Self::NumberedListItem { number: number + 1 },
            Self::TodoItem { .. } => Self::TodoItem { checked: false },
            Self::Code { language } => Self::Code {
                language: language.clone(),
            },
            Self::Quote => Self::Quote,
            Self::Table { .. } | Self::Divider => Self::Paragraph,
            _ => Self::Paragraph,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub id: BlockId,
    pub block_type: BlockType,
    pub content: String,
    pub inlines: Vec<InlineContent>,
    pub children: Vec<Block>,
    pub properties: HashMap<String, String>,
    pub style_override: Option<BlockStyle>,
}

impl Block {
    pub fn new(id: BlockId, block_type: BlockType, content: impl Into<String>) -> Self {
        let text = sanitize_text(&content.into());
        let inlines = if text.is_empty() {
            Vec::new()
        } else {
            vec![InlineContent::Text(text.clone())]
        };
        Self {
            id,
            block_type,
            content: text,
            inlines,
            children: Vec::new(),
            properties: HashMap::new(),
            style_override: None,
        }
    }

    pub fn from_inlines(id: BlockId, block_type: BlockType, inlines: Vec<InlineContent>) -> Self {
        let inlines = normalize_inlines(inlines);
        let content = content_from_inlines(&inlines);
        Self {
            id,
            block_type,
            content,
            inlines,
            children: Vec::new(),
            properties: HashMap::new(),
            style_override: None,
        }
    }

    pub fn with_child(mut self, child: Block) -> Self {
        self.children.push(child);
        self
    }

    pub fn with_style(mut self, style: BlockStyle) -> Self {
        self.style_override = Some(style);
        self
    }

    pub fn set_style(&mut self, style: Option<BlockStyle>) {
        self.style_override = style;
    }

    pub fn style(&self, base_size: f32) -> BlockStyle {
        self.style_override
            .clone()
            .unwrap_or_else(|| BlockStyle::for_block(&self.block_type, base_size))
    }

    pub fn normalized_inlines(&self) -> Vec<InlineContent> {
        normalize_inlines(self.inlines.clone())
    }

    pub fn sync_content(&mut self) {
        self.inlines = normalize_inlines(std::mem::take(&mut self.inlines));
        self.content = content_from_inlines(&self.inlines);
    }

    pub fn set_inlines(&mut self, inlines: Vec<InlineContent>) {
        self.inlines = inlines;
        self.sync_content();
    }

    pub fn plain_text(&self) -> String {
        content_from_inlines(&self.normalized_inlines())
    }

    pub fn text_len(&self) -> usize {
        self.normalized_inlines()
            .iter()
            .map(InlineContent::len_chars)
            .sum()
    }

    pub fn slice_text(&self, range: Range<usize>) -> String {
        let total = self.text_len();
        let start = range.start.min(total);
        let end = range.end.min(total).max(start);
        let mut out = String::new();
        let mut pos = 0;
        for item in self.normalized_inlines() {
            match item {
                InlineContent::Text(text) => {
                    let chars: Vec<char> = text.chars().collect();
                    let seg_end = pos + chars.len();
                    if seg_end > start && pos < end {
                        let a = start.saturating_sub(pos).min(chars.len());
                        let b = end.saturating_sub(pos).min(chars.len());
                        out.extend(chars[a..b].iter());
                    }
                    pos = seg_end;
                }
                InlineContent::Element(_) => {
                    let seg_end = pos + 1;
                    if seg_end > start && pos < end {
                        out.push(INLINE_OBJECT_CHAR);
                    }
                    pos = seg_end;
                }
            }
        }
        out
    }

    pub fn slice_inlines(&self, range: Range<usize>) -> Vec<InlineContent> {
        let total = self.text_len();
        let start = range.start.min(total);
        let end = range.end.min(total).max(start);
        let mut out = Vec::new();
        let mut pos = 0;
        for item in self.normalized_inlines() {
            match item {
                InlineContent::Text(text) => {
                    let chars: Vec<char> = text.chars().collect();
                    let seg_end = pos + chars.len();
                    if seg_end > start && pos < end {
                        let a = start.saturating_sub(pos).min(chars.len());
                        let b = end.saturating_sub(pos).min(chars.len());
                        if a < b {
                            out.push(InlineContent::Text(chars[a..b].iter().collect()));
                        }
                    }
                    pos = seg_end;
                }
                InlineContent::Element(element) => {
                    let seg_end = pos + 1;
                    if seg_end > start && pos < end {
                        out.push(InlineContent::Element(element));
                    }
                    pos = seg_end;
                }
            }
        }
        normalize_inlines(out)
    }

    pub fn split_content(&self, offset: usize) -> (Vec<InlineContent>, Vec<InlineContent>) {
        let total = self.text_len();
        let offset = offset.min(total);
        (
            self.slice_inlines(0..offset),
            self.slice_inlines(offset..total),
        )
    }

    pub fn truncate_to(&mut self, offset: usize) {
        let total = self.text_len();
        let offset = offset.min(total);
        self.inlines = self.slice_inlines(0..offset);
        self.sync_content();
    }

    pub fn append_inlines(&mut self, other: Vec<InlineContent>) {
        self.inlines.extend(other);
        self.sync_content();
    }

    pub fn insert_inlines(&mut self, offset: usize, inlines: Vec<InlineContent>) -> usize {
        let total = self.text_len();
        let offset = offset.min(total);
        let mut out = self.slice_inlines(0..offset);
        out.extend(inlines);
        out.extend(self.slice_inlines(offset..total));
        self.set_inlines(out);
        self.text_len().saturating_sub(total)
    }

    pub fn apply_edit(&mut self, range: Range<usize>, replacement: &str) {
        let total = self.text_len();
        let start = range.start.min(total);
        let end = range.end.min(total).max(start);
        let replacement = sanitize_text(replacement);
        let mut out = Vec::new();
        let mut inserted = false;
        let mut pos = 0;

        for item in self.normalized_inlines() {
            match item {
                InlineContent::Text(text) => {
                    let chars: Vec<char> = text.chars().collect();
                    let seg_start = pos;
                    let seg_end = pos + chars.len();
                    if start >= seg_end {
                        out.push(InlineContent::Text(text));
                    } else if end <= seg_start {
                        push_replacement(&mut out, &replacement, &mut inserted);
                        out.push(InlineContent::Text(text));
                    } else {
                        let a = start.saturating_sub(seg_start).min(chars.len());
                        let b = end.saturating_sub(seg_start).min(chars.len()).max(a);
                        if a > 0 {
                            out.push(InlineContent::Text(chars[..a].iter().collect()));
                        }
                        push_replacement(&mut out, &replacement, &mut inserted);
                        if b < chars.len() {
                            out.push(InlineContent::Text(chars[b..].iter().collect()));
                        }
                    }
                    pos = seg_end;
                }
                InlineContent::Element(element) => {
                    let seg_start = pos;
                    let seg_end = pos + 1;
                    if start >= seg_end {
                        out.push(InlineContent::Element(element));
                    } else if end <= seg_start {
                        push_replacement(&mut out, &replacement, &mut inserted);
                        out.push(InlineContent::Element(element));
                    } else {
                        push_replacement(&mut out, &replacement, &mut inserted);
                    }
                    pos = seg_end;
                }
            }
        }

        push_replacement(&mut out, &replacement, &mut inserted);
        self.inlines = normalize_inlines(out);
        self.content = content_from_inlines(&self.inlines);
    }
}

fn push_replacement(out: &mut Vec<InlineContent>, replacement: &str, inserted: &mut bool) {
    if *inserted {
        return;
    }
    if !replacement.is_empty() {
        out.push(InlineContent::Text(replacement.to_owned()));
    }
    *inserted = true;
}

fn sanitize_text(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

fn normalize_inlines(inlines: Vec<InlineContent>) -> Vec<InlineContent> {
    let mut out: Vec<InlineContent> = Vec::with_capacity(inlines.len());
    for item in inlines {
        match item {
            InlineContent::Text(text) => {
                let text = sanitize_text(&text);
                if text.is_empty() {
                    continue;
                }
                if let Some(InlineContent::Text(previous)) = out.last_mut() {
                    previous.push_str(&text);
                } else {
                    out.push(InlineContent::Text(text));
                }
            }
            element @ InlineContent::Element(_) => out.push(element),
        }
    }
    out
}

fn content_from_inlines(inlines: &[InlineContent]) -> String {
    let mut out = String::new();
    for item in inlines {
        match item {
            InlineContent::Text(text) => out.push_str(text),
            InlineContent::Element(_) => out.push(INLINE_OBJECT_CHAR),
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub blocks: Vec<Block>,
    next_id: u64,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockPosition {
    pub block_id: BlockId,
    pub offset: usize,
}

impl BlockPosition {
    pub fn new(block_id: BlockId, offset: usize) -> Self {
        Self { block_id, offset }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentSelection {
    pub anchor: BlockPosition,
    pub focus: BlockPosition,
}

impl DocumentSelection {
    pub fn new(anchor: BlockPosition, focus: BlockPosition) -> Self {
        Self { anchor, focus }
    }

    pub fn collapsed(position: BlockPosition) -> Self {
        Self {
            anchor: position,
            focus: position,
        }
    }

    pub fn is_collapsed(self) -> bool {
        self.anchor == self.focus
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentFragment {
    pub blocks: Vec<Block>,
    pub single_inline: bool,
}

impl Document {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            next_id: 1,
        }
    }

    pub fn next_block_id(&mut self) -> BlockId {
        let id = self.next_id;
        self.next_id += 1;
        BlockId(id)
    }

    pub fn add_block(&mut self, block_type: BlockType, content: impl Into<String>) -> BlockId {
        let id = self.next_block_id();
        self.blocks.push(Block::new(id, block_type, content));
        id
    }

    pub fn insert_block(
        &mut self,
        index: usize,
        block_type: BlockType,
        content: impl Into<String>,
    ) -> BlockId {
        let id = self.next_block_id();
        let idx = index.min(self.blocks.len());
        self.blocks.insert(idx, Block::new(id, block_type, content));
        id
    }

    pub fn insert_block_value(&mut self, index: usize, block: Block) -> BlockId {
        let id = block.id;
        self.observe_ids(&block);
        let idx = index.min(self.blocks.len());
        self.blocks.insert(idx, block);
        id
    }

    fn observe_ids(&mut self, block: &Block) {
        self.next_id = self.next_id.max(block.id.0 + 1);
        for child in &block.children {
            self.observe_ids(child);
        }
    }

    pub fn remove_block(&mut self, id: BlockId) -> Option<Block> {
        if let Some(pos) = self.blocks.iter().position(|b| b.id == id) {
            Some(self.blocks.remove(pos))
        } else {
            Self::remove_block_recursive(&mut self.blocks, id)
        }
    }

    fn remove_block_recursive(blocks: &mut [Block], id: BlockId) -> Option<Block> {
        for block in blocks.iter_mut() {
            if let Some(pos) = block.children.iter().position(|b| b.id == id) {
                return Some(block.children.remove(pos));
            }
            if let Some(removed) = Self::remove_block_recursive(&mut block.children, id) {
                return Some(removed);
            }
        }
        None
    }

    pub fn find_block(&self, id: BlockId) -> Option<&Block> {
        Self::find_block_in(&self.blocks, id)
    }

    fn find_block_in(blocks: &[Block], id: BlockId) -> Option<&Block> {
        for b in blocks {
            if b.id == id {
                return Some(b);
            }
            if let Some(found) = Self::find_block_in(&b.children, id) {
                return Some(found);
            }
        }
        None
    }

    pub fn find_block_mut(&mut self, id: BlockId) -> Option<&mut Block> {
        Self::find_block_mut_in(&mut self.blocks, id)
    }

    fn find_block_mut_in(blocks: &mut [Block], id: BlockId) -> Option<&mut Block> {
        for b in blocks {
            if b.id == id {
                return Some(b);
            }
            if let Some(found) = Self::find_block_mut_in(&mut b.children, id) {
                return Some(found);
            }
        }
        None
    }

    pub fn block_index(&self, id: BlockId) -> Option<usize> {
        self.blocks.iter().position(|b| b.id == id)
    }

    pub fn block_len(&self, id: BlockId) -> Option<usize> {
        self.find_block(id).map(Block::text_len)
    }

    pub fn first_position(&self) -> Option<BlockPosition> {
        self.blocks.first().map(|b| BlockPosition::new(b.id, 0))
    }

    pub fn last_position(&self) -> Option<BlockPosition> {
        self.blocks
            .last()
            .map(|b| BlockPosition::new(b.id, b.text_len()))
    }

    pub fn clamp_position(&self, position: BlockPosition) -> Option<BlockPosition> {
        if let Some(len) = self.block_len(position.block_id) {
            return Some(BlockPosition::new(
                position.block_id,
                position.offset.min(len),
            ));
        }
        self.first_position()
    }

    pub fn order_positions(
        &self,
        a: BlockPosition,
        b: BlockPosition,
    ) -> (BlockPosition, BlockPosition) {
        let ai = self.block_index(a.block_id).unwrap_or(usize::MAX);
        let bi = self.block_index(b.block_id).unwrap_or(usize::MAX);
        if ai < bi || (ai == bi && a.offset <= b.offset) {
            (a, b)
        } else {
            (b, a)
        }
    }

    pub fn ordered_selection(
        &self,
        selection: DocumentSelection,
    ) -> (BlockPosition, BlockPosition) {
        self.order_positions(selection.anchor, selection.focus)
    }

    pub fn next_position(&self, position: BlockPosition) -> Option<BlockPosition> {
        let index = self.block_index(position.block_id)?;
        let len = self.blocks[index].text_len();
        if position.offset < len {
            return Some(BlockPosition::new(position.block_id, position.offset + 1));
        }
        self.blocks
            .get(index + 1)
            .map(|block| BlockPosition::new(block.id, 0))
    }

    pub fn previous_position(&self, position: BlockPosition) -> Option<BlockPosition> {
        let index = self.block_index(position.block_id)?;
        if position.offset > 0 {
            return Some(BlockPosition::new(position.block_id, position.offset - 1));
        }
        self.blocks
            .get(index.wrapping_sub(1))
            .filter(|_| index > 0)
            .map(|block| BlockPosition::new(block.id, block.text_len()))
    }

    pub fn selection_text(&self, selection: DocumentSelection) -> String {
        if selection.is_collapsed() {
            return String::new();
        }
        let (start, end) = self.ordered_selection(selection);
        let Some(start_index) = self.block_index(start.block_id) else {
            return String::new();
        };
        let Some(end_index) = self.block_index(end.block_id) else {
            return String::new();
        };

        if start_index == end_index {
            return self.blocks[start_index].slice_text(start.offset..end.offset);
        }

        let mut parts = Vec::new();
        for index in start_index..=end_index {
            let block = &self.blocks[index];
            let len = block.text_len();
            let text = if index == start_index {
                block.slice_text(start.offset.min(len)..len)
            } else if index == end_index {
                block.slice_text(0..end.offset.min(len))
            } else {
                block.plain_text()
            };
            parts.push(text);
        }
        parts.join("\n")
    }

    pub fn selection_fragment(&self, selection: DocumentSelection) -> DocumentFragment {
        if selection.is_collapsed() {
            return DocumentFragment::default();
        }
        let (start, end) = self.ordered_selection(selection);
        let Some(start_index) = self.block_index(start.block_id) else {
            return DocumentFragment::default();
        };
        let Some(end_index) = self.block_index(end.block_id) else {
            return DocumentFragment::default();
        };

        if start_index == end_index {
            let block = &self.blocks[start_index];
            let mut copy = Block::from_inlines(
                block.id,
                block.block_type.clone(),
                block.slice_inlines(start.offset..end.offset),
            );
            copy.properties = block.properties.clone();
            copy.style_override = block.style_override.clone();
            return DocumentFragment {
                blocks: vec![copy],
                single_inline: true,
            };
        }

        let mut blocks = Vec::new();
        for index in start_index..=end_index {
            let block = &self.blocks[index];
            let len = block.text_len();
            let inlines = if index == start_index {
                block.slice_inlines(start.offset.min(len)..len)
            } else if index == end_index {
                block.slice_inlines(0..end.offset.min(len))
            } else {
                block.normalized_inlines()
            };
            let mut copy = Block::from_inlines(block.id, block.block_type.clone(), inlines);
            copy.properties = block.properties.clone();
            copy.style_override = block.style_override.clone();
            if index != start_index && index != end_index {
                copy.children = block.children.clone();
            }
            blocks.push(copy);
        }
        DocumentFragment {
            blocks,
            single_inline: false,
        }
    }

    pub fn block_fragment(&self, id: BlockId) -> DocumentFragment {
        let Some(block) = self.find_block(id) else {
            return DocumentFragment::default();
        };
        DocumentFragment {
            blocks: vec![block.clone()],
            single_inline: false,
        }
    }

    pub fn delete_selection(&mut self, selection: DocumentSelection) -> Option<BlockPosition> {
        let (start, end) = self.ordered_selection(selection);
        let start_index = self.block_index(start.block_id)?;
        let end_index = self.block_index(end.block_id)?;

        if start_index == end_index {
            let block = &mut self.blocks[start_index];
            let len = block.text_len();
            let a = start.offset.min(len);
            let b = end.offset.min(len).max(a);
            block.apply_edit(a..b, "");
            return Some(BlockPosition::new(start.block_id, a));
        }

        let end_block = &self.blocks[end_index];
        let end_len = end_block.text_len();
        let suffix = end_block.slice_inlines(end.offset.min(end_len)..end_len);

        let start_block = &mut self.blocks[start_index];
        let start_len = start_block.text_len();
        let a = start.offset.min(start_len);
        start_block.apply_edit(a..start_len, "");
        start_block.append_inlines(suffix);

        self.blocks.drain(start_index + 1..=end_index);
        Some(BlockPosition::new(start.block_id, a))
    }

    pub fn insert_text(&mut self, position: BlockPosition, text: &str) -> Option<BlockPosition> {
        if text.is_empty() {
            return self.clamp_position(position);
        }
        if self.blocks.is_empty() {
            self.add_block(BlockType::Paragraph, "");
        }
        let start = self.clamp_position(position)?;
        let index = self.block_index(start.block_id)?;
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let parts: Vec<&str> = text.split('\n').collect();
        let mut current_index = index;
        let mut offset = start.offset.min(self.blocks[index].text_len());
        let mut current_id = start.block_id;

        for (part_index, part) in parts.iter().enumerate() {
            if part_index == 0 {
                let block = &mut self.blocks[current_index];
                block.apply_edit(offset..offset, part);
                offset += part.chars().count();
                continue;
            }

            current_id = self
                .split_block(BlockPosition::new(current_id, offset))?
                .block_id;
            current_index += 1;
            let block = &mut self.blocks[current_index];
            block.apply_edit(0..0, part);
            offset = part.chars().count();
        }

        Some(BlockPosition::new(current_id, offset))
    }

    pub fn insert_inlines(
        &mut self,
        position: BlockPosition,
        inlines: Vec<InlineContent>,
    ) -> Option<BlockPosition> {
        if inlines.is_empty() {
            return self.clamp_position(position);
        }
        if self.blocks.is_empty() {
            self.add_block(BlockType::Paragraph, "");
        }
        let position = self.clamp_position(position)?;
        let index = self.block_index(position.block_id)?;
        let offset = self.blocks[index].insert_inlines(position.offset, inlines);
        Some(BlockPosition::new(
            position.block_id,
            position.offset + offset,
        ))
    }

    pub fn replace_selection(
        &mut self,
        selection: DocumentSelection,
        text: &str,
    ) -> Option<BlockPosition> {
        if selection.is_collapsed() {
            return self.insert_text(selection.focus, text);
        }
        let position = self.delete_selection(selection)?;
        self.insert_text(position, text)
    }

    pub fn split_block(&mut self, position: BlockPosition) -> Option<BlockPosition> {
        if self.blocks.is_empty() {
            self.add_block(BlockType::Paragraph, "");
        }
        let position = self.clamp_position(position)?;
        let index = self.block_index(position.block_id)?;
        let (left, right) = self.blocks[index].split_content(position.offset);
        let continuation = self.blocks[index].block_type.continuation();
        let new_id = self.next_block_id();
        self.blocks[index].set_inlines(left);
        let mut new_block = Block::from_inlines(new_id, continuation, right);
        new_block.properties = self.blocks[index].properties.clone();
        new_block.style_override = self.blocks[index].style_override.clone();
        self.blocks.insert(index + 1, new_block);
        Some(BlockPosition::new(new_id, 0))
    }

    pub fn merge_with_previous(&mut self, id: BlockId) -> Option<BlockPosition> {
        let index = self.block_index(id)?;
        if index == 0 {
            return None;
        }
        let current = self.blocks.remove(index);
        let previous = &mut self.blocks[index - 1];
        let caret = previous.text_len();
        previous.append_inlines(current.normalized_inlines());
        Some(BlockPosition::new(previous.id, caret))
    }

    pub fn merge_with_next(&mut self, id: BlockId) -> Option<BlockPosition> {
        let index = self.block_index(id)?;
        if index + 1 >= self.blocks.len() {
            return None;
        }
        let next = self.blocks.remove(index + 1);
        let current = &mut self.blocks[index];
        let caret = current.text_len();
        current.append_inlines(next.normalized_inlines());
        Some(BlockPosition::new(current.id, caret))
    }

    pub fn insert_fragment(
        &mut self,
        index: usize,
        fragment: &DocumentFragment,
    ) -> Option<BlockPosition> {
        if fragment.blocks.is_empty() {
            return None;
        }
        let index = index.min(self.blocks.len());
        let mut last = None;
        for (offset, block) in fragment.blocks.iter().enumerate() {
            let mut block = block.clone();
            self.assign_fresh_ids(&mut block);
            let id = block.id;
            let len = block.text_len();
            self.blocks.insert(index + offset, block);
            last = Some(BlockPosition::new(id, len));
        }
        last
    }

    pub fn paste_fragment(
        &mut self,
        position: BlockPosition,
        fragment: &DocumentFragment,
    ) -> Option<BlockPosition> {
        if fragment.single_inline {
            let inlines = fragment
                .blocks
                .first()
                .map(Block::normalized_inlines)
                .unwrap_or_default();
            return self.insert_inlines(position, inlines);
        }
        if fragment.blocks.is_empty() {
            return self.clamp_position(position);
        }
        if self.blocks.is_empty() {
            return self.insert_fragment(0, fragment);
        }
        let position = self.clamp_position(position)?;
        let index = self.block_index(position.block_id)?;
        if self.blocks[index].text_len() == 0 && self.blocks[index].children.is_empty() {
            self.blocks.remove(index);
            return self.insert_fragment(index, fragment);
        }
        self.insert_fragment(index + 1, fragment)
    }

    pub fn select_all(&self) -> DocumentSelection {
        let anchor = self
            .first_position()
            .unwrap_or(BlockPosition::new(BlockId(0), 0));
        let focus = self.last_position().unwrap_or(anchor);
        DocumentSelection::new(anchor, focus)
    }

    fn assign_fresh_ids(&mut self, block: &mut Block) {
        block.id = self.next_block_id();
        for child in &mut block.children {
            self.assign_fresh_ids(child);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockTextStyle {
    pub size: f32,
    pub line_height: f32,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub family: Option<String>,
}

impl BlockTextStyle {
    pub fn for_block(block_type: &BlockType, base_size: f32) -> Self {
        let base_size = if base_size > 0.0 { base_size } else { 14.0 };
        let (size, weight, style) = match block_type {
            BlockType::Heading(HeadingLevel::H1) => {
                (base_size * 1.9, FontWeight::BOLD, FontStyle::Normal)
            }
            BlockType::Heading(HeadingLevel::H2) => {
                (base_size * 1.55, FontWeight::BOLD, FontStyle::Normal)
            }
            BlockType::Heading(HeadingLevel::H3) => {
                (base_size * 1.3, FontWeight::SEMIBOLD, FontStyle::Normal)
            }
            BlockType::Heading(_) => (base_size * 1.12, FontWeight::SEMIBOLD, FontStyle::Normal),
            BlockType::Code { .. } => (base_size * 0.94, FontWeight::NORMAL, FontStyle::Normal),
            BlockType::Quote => (base_size, FontWeight::NORMAL, FontStyle::Italic),
            _ => (base_size, FontWeight::NORMAL, FontStyle::Normal),
        };
        Self {
            size,
            line_height: size * 1.45,
            weight,
            style,
            family: None,
        }
    }

    pub fn line_height(&self) -> f32 {
        self.line_height.max(self.size)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockStyle {
    pub text: BlockTextStyle,
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub gutter: f32,
    pub text_color: Option<Color>,
    pub background: Option<Color>,
}

impl BlockStyle {
    pub fn for_block(block_type: &BlockType, base_size: f32) -> Self {
        let text = BlockTextStyle::for_block(block_type, base_size);
        let mut style = Self {
            text,
            margin_top: 3.0,
            margin_bottom: 3.0,
            padding_x: 0.0,
            padding_y: 2.0,
            gutter: 0.0,
            text_color: None,
            background: None,
        };
        match block_type {
            BlockType::Heading(_) => {
                style.margin_top = 12.0;
                style.margin_bottom = 6.0;
            }
            BlockType::BulletListItem
            | BlockType::NumberedListItem { .. }
            | BlockType::TodoItem { .. } => {
                style.gutter = 28.0;
                style.margin_top = 2.0;
                style.margin_bottom = 2.0;
            }
            BlockType::Quote => {
                style.gutter = 16.0;
                style.padding_x = 8.0;
                style.margin_top = 6.0;
                style.margin_bottom = 6.0;
                style.text_color = Some(Color::rgb8(155, 162, 175));
            }
            BlockType::Code { .. } => {
                style.padding_x = 10.0;
                style.padding_y = 8.0;
                style.margin_top = 8.0;
                style.margin_bottom = 8.0;
                style.background = Some(Color::rgba(0.12, 0.13, 0.16, 1.0));
            }
            BlockType::Callout { .. } => {
                style.padding_x = 44.0;
                style.padding_y = 8.0;
                style.margin_top = 6.0;
                style.margin_bottom = 6.0;
                style.background = Some(Color::rgba(0.16, 0.24, 0.35, 0.6));
            }
            BlockType::Divider => {
                style.padding_y = 7.0;
                style.margin_top = 8.0;
                style.margin_bottom = 8.0;
            }
            BlockType::Table { .. } => {
                style.padding_x = 8.0;
                style.padding_y = 6.0;
                style.margin_top = 6.0;
                style.margin_bottom = 6.0;
                style.background = Some(Color::rgba(0.14, 0.15, 0.18, 0.8));
            }
            BlockType::Paragraph => {}
        }
        style
    }
}

pub trait BlockMeasurer {
    fn caret_positions(&mut self, text: &str, style: &BlockTextStyle) -> Vec<f32>;
}

impl BlockMeasurer for TextSystem {
    fn caret_positions(&mut self, text: &str, style: &BlockTextStyle) -> Vec<f32> {
        self.caret_positions_styled_spaced(
            text,
            style.size,
            style.weight,
            0.0,
            style.family.as_deref(),
            style.style,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub chars: Range<usize>,
    pub text: String,
    pub x: f32,
    pub width: f32,
    pub style: BlockTextStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LaidOutObject {
    pub chars: Range<usize>,
    pub element: InlineElement,
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockLine {
    pub chars: Range<usize>,
    pub top: f32,
    pub height: f32,
    pub baseline: f32,
    pub width: f32,
    pub x_positions: Vec<f32>,
    pub runs: Vec<TextRun>,
    pub objects: Vec<LaidOutObject>,
}

impl BlockLine {
    pub fn x_at(&self, offset: usize) -> f32 {
        let start = self.chars.start;
        let col = offset.clamp(start, self.chars.end) - start;
        self.x_positions
            .get(col)
            .copied()
            .unwrap_or_else(|| self.x_positions.last().copied().unwrap_or(0.0))
    }

    pub fn offset_at_x(&self, x: f32) -> usize {
        if self.x_positions.is_empty() {
            return self.chars.start;
        }
        let mut best_index = 0;
        let mut best_distance = f32::INFINITY;
        for (index, position) in self.x_positions.iter().enumerate() {
            let distance = (position - x).abs();
            if distance < best_distance {
                best_distance = distance;
                best_index = index;
            }
        }
        (self.chars.start + best_index).min(self.chars.end)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DecorationKind {
    Bullet,
    Number(String),
    Todo { checked: bool },
    QuoteBar,
    Divider,
    Callout { icon: String },
    CodeBlock,
    Table,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockDecoration {
    pub kind: DecorationKind,
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LaidOutBlock {
    pub id: BlockId,
    pub index: usize,
    pub block_type: BlockType,
    pub style: BlockStyle,
    pub rect: Rect,
    pub content_rect: Rect,
    pub lines: Vec<BlockLine>,
    pub decoration: Option<BlockDecoration>,
    pub text_len: usize,
}

impl LaidOutBlock {
    pub fn line_at_y(&self, y: f32) -> Option<usize> {
        if self.lines.is_empty() {
            return None;
        }
        let y = y - self.content_rect.top();
        for (index, line) in self.lines.iter().enumerate() {
            if y < line.top + line.height {
                return Some(index);
            }
        }
        Some(self.lines.len() - 1)
    }

    pub fn line_for_offset(&self, offset: usize) -> Option<usize> {
        if self.lines.is_empty() {
            return None;
        }
        for (index, line) in self.lines.iter().enumerate() {
            if offset <= line.chars.end {
                return Some(index);
            }
        }
        Some(self.lines.len() - 1)
    }

    pub fn caret_rect(&self, offset: usize) -> Rect {
        let offset = offset.min(self.text_len);
        if let Some(line_index) = self.line_for_offset(offset) {
            let line = &self.lines[line_index];
            let x = self.content_rect.left() + line.x_at(offset);
            let y = self.content_rect.top() + line.top;
            return Rect::from_xywh(x, y, 1.0, line.height.max(1.0));
        }
        Rect::from_xywh(
            self.content_rect.left(),
            self.content_rect.top(),
            1.0,
            self.style.text.line_height(),
        )
    }

    pub fn line_rect(&self, line_index: usize) -> Option<Rect> {
        let line = self.lines.get(line_index)?;
        Some(Rect::from_xywh(
            self.content_rect.left(),
            self.content_rect.top() + line.top,
            self.content_rect.size.width,
            line.height,
        ))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LaidOutDocument {
    pub blocks: Vec<LaidOutBlock>,
    pub width: f32,
    pub height: f32,
}

impl LaidOutDocument {
    pub fn layout(
        document: &Document,
        width: f32,
        base_size: f32,
        measurer: &mut dyn BlockMeasurer,
    ) -> Self {
        layout_document(document, width, base_size, measurer)
    }

    pub fn block(&self, id: BlockId) -> Option<&LaidOutBlock> {
        self.blocks.iter().find(|block| block.id == id)
    }

    pub fn block_index(&self, id: BlockId) -> Option<usize> {
        self.blocks.iter().position(|block| block.id == id)
    }

    pub fn caret_rect(&self, position: BlockPosition) -> Option<Rect> {
        self.block(position.block_id)
            .map(|block| block.caret_rect(position.offset))
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<BlockPosition> {
        let first = self.blocks.first()?;
        if y < first.rect.top() {
            return Some(BlockPosition::new(first.id, 0));
        }
        let last = self.blocks.last()?;
        if y >= last.rect.bottom() {
            return Some(BlockPosition::new(last.id, last.text_len));
        }

        for block in &self.blocks {
            if y < block.rect.top() || y > block.rect.bottom() {
                continue;
            }
            let local_x = x - block.content_rect.left();
            if let Some(line_index) = block.line_at_y(y) {
                let line = &block.lines[line_index];
                if local_x <= 0.0 {
                    return Some(BlockPosition::new(block.id, line.chars.start));
                }
                let offset = line.offset_at_x(local_x);
                return Some(BlockPosition::new(block.id, offset));
            }
            let offset = if y < block.content_rect.top() {
                0
            } else {
                block.text_len
            };
            return Some(BlockPosition::new(block.id, offset));
        }

        for pair in self.blocks.windows(2) {
            let previous = &pair[0];
            let next = &pair[1];
            if y >= previous.rect.bottom() && y < next.rect.top() {
                let previous_distance = y - previous.rect.bottom();
                let next_distance = next.rect.top() - y;
                if previous_distance <= next_distance {
                    return Some(BlockPosition::new(previous.id, previous.text_len));
                }
                return Some(BlockPosition::new(next.id, 0));
            }
        }

        self.blocks.last().map(|block| {
            let offset = if y > block.content_rect.top() {
                block.text_len
            } else {
                0
            };
            BlockPosition::new(block.id, offset)
        })
    }

    pub fn object_at(
        &self,
        x: f32,
        y: f32,
    ) -> Option<(BlockPosition, Range<usize>, &InlineElement)> {
        for block in &self.blocks {
            for line in &block.lines {
                for object in &line.objects {
                    let rect = Rect::from_xywh(
                        block.content_rect.left() + object.rect.left(),
                        block.content_rect.top() + object.rect.top(),
                        object.rect.size.width,
                        object.rect.size.height,
                    );
                    if rect.contains(sequoia_core::Point::new(x, y)) {
                        return Some((
                            BlockPosition::new(block.id, object.chars.start),
                            object.chars.clone(),
                            &object.element,
                        ));
                    }
                }
            }
        }
        None
    }

    pub fn selection_rects(&self, selection: DocumentSelection) -> Vec<Rect> {
        if selection.is_collapsed() {
            return Vec::new();
        }
        let Some(start_index) = self.block_index(selection.anchor.block_id) else {
            return Vec::new();
        };
        let Some(end_index) = self.block_index(selection.focus.block_id) else {
            return Vec::new();
        };
        let (start_index, end_index, start, end) = if start_index < end_index
            || (start_index == end_index && selection.anchor.offset <= selection.focus.offset)
        {
            (start_index, end_index, selection.anchor, selection.focus)
        } else {
            (end_index, start_index, selection.focus, selection.anchor)
        };

        let mut rects = Vec::new();
        for index in start_index..=end_index {
            let block = &self.blocks[index];
            let selection_start = if index == start_index {
                start.offset
            } else {
                0
            };
            let selection_end = if index == end_index {
                end.offset
            } else {
                block.text_len
            };

            if block.lines.is_empty() {
                let height = block.style.text.line_height().max(4.0);
                rects.push(Rect::from_xywh(
                    block.content_rect.left(),
                    block.content_rect.top(),
                    block.content_rect.size.width.max(2.0),
                    height,
                ));
                continue;
            }

            for line in &block.lines {
                let line_start = line.chars.start;
                let line_end = line.chars.end;
                let overlaps = selection_start <= line_end && selection_end >= line_start;
                if !overlaps {
                    continue;
                }
                let full_line = selection_start <= line_start && selection_end >= line_end;
                let a = selection_start.max(line_start);
                let b = selection_end.min(line_end);
                let x0 = if full_line { 0.0 } else { line.x_at(a) };
                let x1 = if full_line {
                    block.content_rect.size.width.max(x0 + 2.0)
                } else {
                    line.x_at(b)
                };
                if x1 <= x0 && !full_line && line_start != line_end {
                    continue;
                }
                let width = (x1 - x0).max(2.0);
                rects.push(Rect::from_xywh(
                    block.content_rect.left() + x0,
                    block.content_rect.top() + line.top,
                    width,
                    line.height.max(1.0),
                ));
            }
        }
        rects
    }
}

#[derive(Clone, Debug)]
struct Token {
    text: String,
    chars: Range<usize>,
    width: f32,
    positions: Vec<f32>,
    is_space: bool,
    object: Option<InlineElement>,
}

fn layout_document(
    document: &Document,
    width: f32,
    base_size: f32,
    measurer: &mut dyn BlockMeasurer,
) -> LaidOutDocument {
    let width = width.max(1.0);
    let mut y = 0.0;
    let mut blocks = Vec::with_capacity(document.blocks.len());

    for (index, block) in document.blocks.iter().enumerate() {
        let style = block.style(base_size);
        let gutter = style.gutter;
        let content_width = (width - style.padding_x * 2.0 - gutter).max(1.0);
        let mut laid = layout_block(block, index, width, &style, content_width, measurer);

        let lines_height = laid
            .lines
            .last()
            .map(|line| line.top + line.height)
            .unwrap_or(style.text.line_height());
        let content_height = if matches!(block.block_type, BlockType::Divider) {
            1.0
        } else {
            lines_height
        };
        let height = content_height + style.padding_y * 2.0;
        laid.rect = Rect::from_xywh(0.0, y + style.margin_top, width, height);
        laid.content_rect = Rect::from_xywh(
            style.padding_x + gutter,
            laid.rect.top() + style.padding_y,
            content_width,
            content_height,
        );
        laid.decoration = decoration_for(block, &laid, style.padding_x);
        y += style.margin_top + height + style.margin_bottom;
        blocks.push(laid);
    }

    let height = blocks
        .last()
        .map(|block| block.rect.bottom())
        .unwrap_or(0.0);
    LaidOutDocument {
        blocks,
        width,
        height,
    }
}

fn layout_block(
    block: &Block,
    index: usize,
    width: f32,
    style: &BlockStyle,
    content_width: f32,
    measurer: &mut dyn BlockMeasurer,
) -> LaidOutBlock {
    let text_len = block.text_len();
    let mut lines = Vec::new();
    let lines_height = if matches!(block.block_type, BlockType::Divider) {
        0.0
    } else {
        let tokens = tokens_for(block, style, measurer);
        lines = build_lines(tokens, style, content_width);
        lines
            .last()
            .map(|line| line.top + line.height)
            .unwrap_or(style.text.line_height())
    };

    LaidOutBlock {
        id: block.id,
        index,
        block_type: block.block_type.clone(),
        style: style.clone(),
        rect: Rect::from_xywh(0.0, 0.0, width, lines_height + style.padding_y * 2.0),
        content_rect: Rect::from_xywh(
            style.padding_x + style.gutter,
            style.padding_y,
            content_width,
            lines_height,
        ),
        lines,
        decoration: None,
        text_len,
    }
}

fn tokens_for(block: &Block, style: &BlockStyle, measurer: &mut dyn BlockMeasurer) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut offset = 0;
    for item in block.normalized_inlines() {
        match item {
            InlineContent::Text(text) => {
                tokens.extend(tokenize_text(&text, offset, &style.text, measurer));
                offset += text.chars().count();
            }
            InlineContent::Element(element) => {
                let mut positions = measurer.caret_positions(element.label(), &style.text);
                let label_width = positions.last().copied().unwrap_or(0.0);
                let width = match &element {
                    InlineElement::Image { .. } => style.text.size * 2.0,
                    InlineElement::Icon(_) => style.text.size * 1.25,
                    InlineElement::Math(_) => label_width + style.text.size * 0.7,
                    _ => label_width + style.text.size * 0.9,
                }
                .max(style.text.size * 0.6);
                positions.clear();
                positions.push(0.0);
                positions.push(width);
                tokens.push(Token {
                    text: element.label().to_owned(),
                    chars: offset..offset + 1,
                    width,
                    positions,
                    is_space: false,
                    object: Some(element),
                });
                offset += 1;
            }
        }
    }
    tokens
}

fn tokenize_text(
    text: &str,
    offset: usize,
    style: &BlockTextStyle,
    measurer: &mut dyn BlockMeasurer,
) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let is_space = chars[index].is_whitespace();
        let start = index;
        while index < chars.len() && chars[index].is_whitespace() == is_space {
            index += 1;
        }
        let segment: String = chars[start..index].iter().collect();
        let positions = measurer.caret_positions(&segment, style);
        let width = positions.last().copied().unwrap_or(0.0);
        tokens.push(Token {
            text: segment,
            chars: offset + start..offset + index,
            width,
            positions,
            is_space,
            object: None,
        });
    }
    tokens
}

fn split_token(token: Token) -> Vec<Token> {
    if token.object.is_some() || token.width <= 0.0 {
        return vec![token];
    }
    let chars: Vec<char> = token.text.chars().collect();
    let mut out = Vec::with_capacity(chars.len());
    for (index, ch) in chars.iter().enumerate() {
        let width = token.positions[index + 1] - token.positions[index];
        out.push(Token {
            text: ch.to_string(),
            chars: token.chars.start + index..token.chars.start + index + 1,
            width,
            positions: vec![0.0, width],
            is_space: token.is_space,
            object: None,
        });
    }
    out
}

fn build_lines(mut tokens: Vec<Token>, style: &BlockStyle, content_width: f32) -> Vec<BlockLine> {
    let max_width = content_width.max(1.0);
    let mut prepared = Vec::with_capacity(tokens.len());
    for token in tokens.drain(..) {
        if token.width > max_width && token.object.is_none() {
            prepared.extend(split_token(token));
        } else {
            prepared.push(token);
        }
    }

    let mut lines = Vec::new();
    let mut current: Vec<(Token, f32)> = Vec::new();
    let mut line_width = 0.0;
    let mut line_top = 0.0;

    for token in prepared {
        if !token.is_space && !current.is_empty() && line_width + token.width > max_width {
            let line = finalize_line(std::mem::take(&mut current), line_top, line_width, style);
            line_top += line.height;
            lines.push(line);
            line_width = 0.0;
        }
        current.push((token.clone(), line_width));
        line_width += token.width;
    }

    if !current.is_empty() || lines.is_empty() {
        let line = finalize_line(current, line_top, line_width, style);
        lines.push(line);
    }

    lines
}

fn finalize_line(placed: Vec<(Token, f32)>, top: f32, width: f32, style: &BlockStyle) -> BlockLine {
    let char_start = placed
        .first()
        .map(|(token, _)| token.chars.start)
        .unwrap_or(0);
    let char_end = placed
        .last()
        .map(|(token, _)| token.chars.end)
        .unwrap_or(char_start);
    let object_height = placed
        .iter()
        .filter_map(|(token, _)| token.object.as_ref())
        .map(|element| element.height(style.text.size))
        .fold(0.0f32, f32::max);
    let base_height = style.text.line_height();
    let height = base_height.max(object_height + style.text.size * 0.2);
    let extra = (height - base_height).max(0.0);
    let baseline = top + extra * 0.5 + style.text.size * 0.8;

    let mut x_positions = Vec::with_capacity(char_end.saturating_sub(char_start) + 1);
    let mut runs = Vec::new();
    let mut objects = Vec::new();
    let mut cursor = char_start;
    let mut x = 0.0;

    for (token, token_x) in &placed {
        if cursor < token.chars.start {
            while cursor < token.chars.start {
                x_positions.push(x);
                cursor += 1;
            }
        }
        if let Some(element) = &token.object {
            if x_positions.is_empty() {
                x_positions.push(x);
            }
            objects.push(LaidOutObject {
                chars: token.chars.clone(),
                element: element.clone(),
                rect: Rect::from_xywh(
                    *token_x,
                    top + (height - element.height(style.text.size)).max(0.0) * 0.5,
                    token.width,
                    element.height(style.text.size),
                ),
            });
            x += token.width;
            x_positions.push(x);
            cursor = token.chars.end;
            continue;
        }

        let positions = &token.positions;
        if x_positions.is_empty() {
            x_positions.extend(positions.iter().map(|position| x + position));
        } else {
            x_positions.extend(positions.iter().skip(1).map(|position| x + position));
        }
        runs.push(TextRun {
            chars: token.chars.clone(),
            text: token.text.clone(),
            x: *token_x,
            width: token.width,
            style: style.text.clone(),
        });
        x += token.width;
        cursor = token.chars.end;
    }

    if x_positions.is_empty() {
        x_positions.push(0.0);
    }

    BlockLine {
        chars: char_start..cursor.max(char_end),
        top,
        height,
        baseline,
        width: width.max(x),
        x_positions,
        runs,
        objects,
    }
}

fn decoration_for(block: &Block, laid: &LaidOutBlock, padding_x: f32) -> Option<BlockDecoration> {
    let first_line_height = laid
        .lines
        .first()
        .map(|line| line.height)
        .unwrap_or(laid.style.text.line_height());
    let first_top =
        laid.content_rect.top() + laid.lines.first().map(|line| line.top).unwrap_or(0.0);
    let size = laid.style.text.size;

    match &block.block_type {
        BlockType::BulletListItem => Some(BlockDecoration {
            kind: DecorationKind::Bullet,
            rect: Rect::from_xywh(
                padding_x + 8.0,
                first_top + first_line_height * 0.5 - 3.0,
                6.0,
                6.0,
            ),
        }),
        BlockType::NumberedListItem { number } => Some(BlockDecoration {
            kind: DecorationKind::Number(number.to_string()),
            rect: Rect::from_xywh(
                padding_x,
                first_top,
                (laid.style.gutter - 6.0).max(8.0),
                first_line_height,
            ),
        }),
        BlockType::TodoItem { checked } => {
            let box_size = (size * 1.0).clamp(12.0, 18.0);
            Some(BlockDecoration {
                kind: DecorationKind::Todo { checked: *checked },
                rect: Rect::from_xywh(
                    padding_x + 5.0,
                    first_top + (first_line_height - box_size) * 0.5,
                    box_size,
                    box_size,
                ),
            })
        }
        BlockType::Quote => Some(BlockDecoration {
            kind: DecorationKind::QuoteBar,
            rect: Rect::from_xywh(padding_x + 1.0, laid.rect.top(), 3.0, laid.rect.size.height),
        }),
        BlockType::Divider => Some(BlockDecoration {
            kind: DecorationKind::Divider,
            rect: Rect::from_xywh(
                padding_x,
                laid.rect.top() + laid.rect.size.height * 0.5,
                (laid.rect.size.width - padding_x * 2.0).max(1.0),
                1.0,
            ),
        }),
        BlockType::Callout { icon } => Some(BlockDecoration {
            kind: DecorationKind::Callout { icon: icon.clone() },
            rect: laid.rect,
        }),
        BlockType::Code { .. } => Some(BlockDecoration {
            kind: DecorationKind::CodeBlock,
            rect: laid.rect,
        }),
        BlockType::Table { .. } => Some(BlockDecoration {
            kind: DecorationKind::Table,
            rect: laid.rect,
        }),
        BlockType::Paragraph | BlockType::Heading(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paragraph(id: u64, text: &str) -> Block {
        Block::new(BlockId(id), BlockType::Paragraph, text)
    }

    #[test]
    fn block_edit_replaces_text() {
        let mut block = paragraph(1, "hello world");
        block.apply_edit(6..11, "there");
        assert_eq!(block.plain_text(), "hello there");
        assert_eq!(block.content, "hello there");
    }

    #[test]
    fn block_edit_handles_inline_objects() {
        let mut block = Block::from_inlines(
            BlockId(1),
            BlockType::Paragraph,
            vec![
                InlineContent::Text("see ".into()),
                InlineContent::Element(InlineElement::Mention("ada".into())),
                InlineContent::Text(" now".into()),
            ],
        );
        assert_eq!(block.text_len(), 9);
        assert_eq!(block.content, "see \u{FFFC} now");
        block.apply_edit(4..4, "the ");
        assert_eq!(block.content, "see the \u{FFFC} now");
        block.apply_edit(4..8, "");
        assert_eq!(block.content, "see \u{FFFC} now");
        block.apply_edit(4..5, "X");
        assert_eq!(block.content, "see X now");
    }

    #[test]
    fn document_insert_text_splits_blocks() {
        let mut document = Document::new();
        let id = document.add_block(BlockType::Paragraph, "hello");
        let position = document
            .insert_text(BlockPosition::new(id, 2), " there\nworld")
            .unwrap();
        assert_eq!(document.blocks.len(), 2);
        assert_eq!(document.blocks[0].plain_text(), "he there");
        assert_eq!(document.blocks[1].plain_text(), "worldllo");
        assert_eq!(position.offset, 5);
    }

    #[test]
    fn document_delete_cross_block_selection() {
        let mut document = Document::new();
        let first = document.add_block(BlockType::Paragraph, "hello");
        let second = document.add_block(BlockType::Paragraph, "world");
        let selection =
            DocumentSelection::new(BlockPosition::new(first, 2), BlockPosition::new(second, 3));
        let position = document.delete_selection(selection).unwrap();
        assert_eq!(document.blocks.len(), 1);
        assert_eq!(document.blocks[0].plain_text(), "held");
        assert_eq!(position, BlockPosition::new(first, 2));
    }

    #[test]
    fn document_split_and_merge_blocks() {
        let mut document = Document::new();
        let first = document.add_block(BlockType::Heading(HeadingLevel::H1), "Title");
        let second = document.split_block(BlockPosition::new(first, 5)).unwrap();
        assert_eq!(document.blocks.len(), 2);
        assert_eq!(document.blocks[1].block_type, BlockType::Paragraph);
        assert_eq!(second.offset, 0);
        let merged = document.merge_with_previous(second.block_id).unwrap();
        assert_eq!(document.blocks.len(), 1);
        assert_eq!(merged, BlockPosition::new(first, 5));
    }

    #[test]
    fn document_selection_text_and_fragment() {
        let mut document = Document::new();
        let first = document.add_block(BlockType::Paragraph, "alpha");
        let second = document.add_block(BlockType::Paragraph, "beta");
        let selection =
            DocumentSelection::new(BlockPosition::new(first, 3), BlockPosition::new(second, 2));
        assert_eq!(document.selection_text(selection), "ha\nbe");
        let fragment = document.selection_fragment(selection);
        assert!(!fragment.single_inline);
        assert_eq!(fragment.blocks.len(), 2);
    }

    #[test]
    fn pasting_single_inline_fragment_keeps_objects() {
        let mut document = Document::new();
        let id = document.add_block(BlockType::Paragraph, "");
        document.find_block_mut(id).unwrap().set_inlines(vec![
            InlineContent::Text("a ".into()),
            InlineContent::Element(InlineElement::Tag("x".into())),
            InlineContent::Text(" b".into()),
        ]);
        let selection =
            DocumentSelection::new(BlockPosition::new(id, 2), BlockPosition::new(id, 3));
        let fragment = document.selection_fragment(selection);
        assert!(fragment.single_inline);

        let position = document.delete_selection(selection).unwrap();
        let pasted = document.paste_fragment(position, &fragment).unwrap();
        let block = document.find_block(id).unwrap();
        assert_eq!(block.plain_text(), "a \u{FFFC} b");
        assert_eq!(pasted.offset, 3);
        assert!(matches!(
            block.inlines.as_slice(),
            [
                InlineContent::Text(_),
                InlineContent::Element(InlineElement::Tag(_)),
                InlineContent::Text(_)
            ]
        ));
    }

    #[test]
    fn document_paste_fragment_assigns_fresh_ids() {
        let mut document = Document::new();
        let first = document.add_block(BlockType::Paragraph, "one");
        let second = document.add_block(BlockType::Paragraph, "two");
        let fragment = document.block_fragment(second);
        let pasted = document
            .paste_fragment(BlockPosition::new(first, 3), &fragment)
            .unwrap();
        assert_eq!(document.blocks.len(), 3);
        assert_ne!(pasted.block_id, second);
        assert_eq!(
            document.find_block(pasted.block_id).unwrap().plain_text(),
            "two"
        );
    }

    #[test]
    fn paste_fragment_into_an_empty_document_inserts_blocks() {
        let mut source = Document::new();
        let id = source.add_block(BlockType::Paragraph, "one");
        let fragment = source.block_fragment(id);
        let mut target = Document::new();
        let pasted = target
            .paste_fragment(BlockPosition::new(BlockId(0), 0), &fragment)
            .unwrap();
        assert_eq!(target.blocks.len(), 1);
        assert_eq!(
            target.find_block(pasted.block_id).unwrap().plain_text(),
            "one"
        );
    }

    #[test]
    fn layout_breaks_lines_and_places_objects() {
        let mut document = Document::new();
        let id = document.add_block(BlockType::Paragraph, "");
        let block = document.find_block_mut(id).unwrap();
        block.set_inlines(vec![
            InlineContent::Text("hello world ".into()),
            InlineContent::Element(InlineElement::Tag("rust".into())),
            InlineContent::Text(" trailing words here".into()),
        ]);
        let mut text = TextSystem::new();
        let layout = LaidOutDocument::layout(&document, 100.0, 14.0, &mut text);
        let laid = &layout.blocks[0];
        assert!(!laid.lines.is_empty());
        assert!(laid.lines.iter().all(|line| line.width <= 100.0));
        assert!(laid.lines.iter().any(|line| !line.objects.is_empty()));

        let caret = laid.caret_rect(0);
        assert!(caret.left() >= laid.content_rect.left());
        let hit = layout
            .hit_test(caret.left() + 1.0, caret.top() + 1.0)
            .unwrap();
        assert_eq!(hit.block_id, id);
    }

    #[test]
    fn layout_selection_spans_blocks() {
        let mut document = Document::new();
        let first = document.add_block(BlockType::Paragraph, "hello");
        let second = document.add_block(BlockType::Paragraph, "world");
        let mut text = TextSystem::new();
        let layout = LaidOutDocument::layout(&document, 300.0, 14.0, &mut text);
        let selection =
            DocumentSelection::new(BlockPosition::new(first, 2), BlockPosition::new(second, 3));
        let rects = layout.selection_rects(selection);
        assert!(!rects.is_empty());
        assert!(rects.len() >= 2);
    }

    struct FixedMeasurer;

    impl BlockMeasurer for FixedMeasurer {
        fn caret_positions(&mut self, text: &str, _style: &BlockTextStyle) -> Vec<f32> {
            (0..=text.chars().count())
                .map(|index| index as f32 * 5.0)
                .collect()
        }
    }

    #[test]
    fn inline_object_starts_at_its_text_offset() {
        let mut document = Document::new();
        let id = document.add_block(BlockType::Paragraph, "");
        let block = document.find_block_mut(id).unwrap();
        block.set_inlines(vec![
            InlineContent::Text("ab".into()),
            InlineContent::Element(InlineElement::Tag("rust".into())),
            InlineContent::Text("cd".into()),
        ]);
        let layout = LaidOutDocument::layout(&document, 200.0, 14.0, &mut FixedMeasurer);
        let object = &layout.blocks[0].lines[0].objects[0];
        assert!((object.rect.left() - 10.0).abs() < 0.01);
        assert_eq!(object.chars, 2..3);

        let laid = &layout.blocks[0];
        let center = object.rect.center();
        let hit = layout
            .object_at(
                laid.content_rect.left() + center.x,
                laid.content_rect.top() + center.y,
            )
            .unwrap();
        assert_eq!(hit.1, 2..3);
        assert!(matches!(hit.2, InlineElement::Tag(_)));
    }

    #[test]
    fn leading_spaces_remain_addressable() {
        let mut document = Document::new();
        let id = document.add_block(BlockType::Paragraph, "  hi");
        let layout = LaidOutDocument::layout(&document, 200.0, 14.0, &mut FixedMeasurer);
        let laid = &layout.blocks[0];
        assert_eq!(laid.lines[0].chars, 0..4);
        let hit = layout
            .hit_test(laid.content_rect.left(), laid.content_rect.top() + 1.0)
            .unwrap();
        assert_eq!(hit.block_id, id);
        assert_eq!(hit.offset, 0);
    }

    #[test]
    fn style_override_controls_margins_and_padding() {
        let mut document = Document::new();
        let first = document.add_block(BlockType::Paragraph, "first");
        document.add_block(BlockType::Paragraph, "second");
        let mut style = BlockStyle::for_block(&BlockType::Paragraph, 14.0);
        style.margin_top = 0.0;
        style.margin_bottom = 40.0;
        style.padding_x = 30.0;
        document
            .find_block_mut(first)
            .unwrap()
            .set_style(Some(style));

        let layout = LaidOutDocument::layout(&document, 240.0, 14.0, &mut TextSystem::new());
        let laid = &layout.blocks[0];
        assert_eq!(laid.content_rect.left(), 30.0);
        assert!(layout.blocks[1].rect.top() - laid.rect.bottom() >= 40.0);
    }

    #[test]
    fn splitting_preserves_block_style_and_properties() {
        let mut document = Document::new();
        let first = document.add_block(BlockType::BulletListItem, "hello");
        let mut style = BlockStyle::for_block(&BlockType::BulletListItem, 14.0);
        style.padding_x = 24.0;
        let block = document.find_block_mut(first).unwrap();
        block.set_style(Some(style.clone()));
        block.properties.insert("key".into(), "value".into());

        let second = document
            .split_block(BlockPosition::new(first, 2))
            .unwrap()
            .block_id;
        let block = document.find_block(second).unwrap();
        assert_eq!(block.block_type, BlockType::BulletListItem);
        assert_eq!(block.style_override, Some(style.clone()));
        assert_eq!(
            block.properties.get("key").map(String::as_str),
            Some("value")
        );

        let third = document
            .insert_text(BlockPosition::new(second, 2), "\n")
            .unwrap()
            .block_id;
        let block = document.find_block(third).unwrap();
        assert_eq!(block.block_type, BlockType::BulletListItem);
        assert_eq!(block.style_override, Some(style));
        assert_eq!(
            block.properties.get("key").map(String::as_str),
            Some("value")
        );
    }

    #[test]
    fn code_and_quote_blocks_continue_with_their_type() {
        let mut document = Document::new();
        let code = document.add_block(
            BlockType::Code {
                language: "rust".into(),
            },
            "fn main() {}",
        );
        let quote = document.add_block(BlockType::Quote, "quote");
        let code_split = document
            .split_block(BlockPosition::new(code, 2))
            .unwrap()
            .block_id;
        let quote_split = document
            .split_block(BlockPosition::new(quote, 2))
            .unwrap()
            .block_id;
        assert_eq!(
            document.find_block(code_split).unwrap().block_type,
            BlockType::Code {
                language: "rust".into()
            }
        );
        assert_eq!(
            document.find_block(quote_split).unwrap().block_type,
            BlockType::Quote
        );
    }

    #[test]
    fn every_block_gets_independent_layout() {
        let mut document = Document::new();
        document.add_block(BlockType::Paragraph, "first");
        document.add_block(BlockType::Heading(HeadingLevel::H1), "second");
        document.add_block(BlockType::BulletListItem, "third");
        let mut text = TextSystem::new();
        let layout = LaidOutDocument::layout(&document, 240.0, 14.0, &mut text);
        assert_eq!(layout.blocks.len(), 3);
        assert!(
            layout
                .blocks
                .windows(2)
                .all(|pair| pair[0].rect.bottom() <= pair[1].rect.top())
        );
        assert!(layout.blocks[1].rect.size.height > layout.blocks[0].rect.size.height);
        assert!(layout.blocks[2].decoration.is_some());
    }
}
