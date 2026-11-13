use std::cell::RefCell;
use std::rc::Rc;

use sequoia_core::{Color, Length, Point, Rect, Vec2};
use sequoia_input::{
    CursorIcon, Event, EventPhase, ImeEvent, Key, KeyEvent, Modifiers, PointerButton, PointerEvent,
};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;
use sequoia_style::{Theme, VisualState};
use sequoia_text::{
    BlockId, BlockMeasurer, BlockTextStyle, BlockType, DecorationKind, Document, DocumentFragment,
    InlineElement, LaidOutBlock, LaidOutDocument,
};
use sequoia_view::{AnyView, EventCtx, PaintCtx, Unit, View};

use crate::clipboard::SharedClipboard;

pub use sequoia_text::{BlockPosition, DocumentSelection};

const HISTORY_LIMIT: usize = 128;

#[derive(Clone)]
struct EditorState {
    focus: BlockPosition,
    anchor: Option<BlockPosition>,
    goal_x: Option<f32>,
    preedit: String,
    preedit_cursor: Option<(usize, usize)>,
    ime_rect: Option<Rect>,
    dragging: bool,
    clipboard: Option<DocumentFragment>,
    clipboard_text: Option<String>,
    scroll_y: f32,
    ensure_visible: bool,
    caret_line: Option<usize>,
    undo: Vec<Document>,
    redo: Vec<Document>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            focus: BlockPosition::new(BlockId(0), 0),
            anchor: None,
            goal_x: None,
            preedit: String::new(),
            preedit_cursor: None,
            ime_rect: None,
            dragging: false,
            clipboard: None,
            clipboard_text: None,
            scroll_y: 0.0,
            ensure_visible: true,
            caret_line: None,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}

pub struct BlockEditor {
    document: Signal<Document>,
    focused_block: Signal<Option<BlockId>>,
    selection: Signal<Option<DocumentSelection>>,
    style: LayoutStyle,
    state: Rc<RefCell<EditorState>>,
    revision: Signal<u64>,
    base_font_size: Option<f32>,
    pad_x: f32,
    pad_y: f32,
    background: Option<Color>,
}

impl BlockEditor {
    pub fn new(document: Signal<Document>) -> Self {
        let focus = document
            .get_untracked()
            .first_position()
            .unwrap_or_else(|| BlockPosition::new(BlockId(0), 0));
        let focused_block = Signal::new(document.get_untracked().blocks.first().map(|b| b.id));
        let state = Rc::new(RefCell::new(EditorState {
            focus,
            ..EditorState::default()
        }));
        Self {
            document,
            focused_block,
            selection: Signal::new(None),
            style: LayoutStyle::default(),
            state,
            revision: Signal::new(0),
            base_font_size: None,
            pad_x: 12.0,
            pad_y: 8.0,
            background: None,
        }
    }

    pub fn style(mut self, style: LayoutStyle) -> Self {
        self.style = style;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        if size > 0.0 {
            self.base_font_size = Some(size);
        }
        self
    }

    pub fn padding(mut self, x: f32, y: f32) -> Self {
        self.pad_x = x.max(0.0);
        self.pad_y = y.max(0.0);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn document(&self) -> Signal<Document> {
        self.document.clone()
    }

    pub fn focused_block(&self) -> Signal<Option<BlockId>> {
        self.focused_block.clone()
    }

    pub fn selection_signal(&self) -> Signal<Option<DocumentSelection>> {
        self.selection.clone()
    }

    pub fn focus_position(&self) -> Option<BlockPosition> {
        let document = self.document.get_untracked();
        document.clamp_position(self.state.borrow().focus)
    }

    pub fn set_focus_position(&self, position: BlockPosition) {
        let document = self.document.get_untracked();
        let Some(position) = document.clamp_position(position) else {
            return;
        };
        {
            let mut state = self.state.borrow_mut();
            state.focus = position;
            state.anchor = None;
            state.goal_x = None;
        }
        self.after_change();
    }

    pub fn set_selection(&self, selection: Option<DocumentSelection>) {
        let document = self.document.get_untracked();
        let mut state = self.state.borrow_mut();
        match selection {
            Some(selection) => {
                let anchor = document
                    .clamp_position(selection.anchor)
                    .unwrap_or(selection.anchor);
                let focus = document
                    .clamp_position(selection.focus)
                    .unwrap_or(selection.focus);
                state.focus = focus;
                state.anchor = if anchor == focus { None } else { Some(anchor) };
            }
            None => {
                state.anchor = None;
            }
        }
        state.goal_x = None;
        drop(state);
        self.after_change();
    }

    pub fn selection(&self) -> Option<DocumentSelection> {
        let state = self.state.borrow();
        state.anchor.and_then(|anchor| {
            (anchor != state.focus).then_some(DocumentSelection::new(anchor, state.focus))
        })
    }

    pub fn selection_text(&self) -> String {
        let document = self.document.get_untracked();
        self.selection()
            .map(|selection| document.selection_text(selection))
            .unwrap_or_default()
    }

    pub fn text(&self) -> String {
        let document = self.document.get_untracked();
        document
            .blocks
            .iter()
            .map(|block| block.plain_text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn len_blocks(&self) -> usize {
        self.document.get_untracked().blocks.len()
    }

    pub fn scroll_offset(&self) -> f32 {
        self.state.borrow().scroll_y
    }

    fn base_size(&self, theme: &Theme) -> f32 {
        self.base_font_size
            .unwrap_or_else(|| theme.font_size.max(14.0))
    }

    fn content_rect(&self, rect: Rect) -> Rect {
        Rect::from_xywh(
            rect.left() + self.pad_x,
            rect.top() + self.pad_y,
            (rect.size.width - self.pad_x * 2.0).max(1.0),
            (rect.size.height - self.pad_y * 2.0).max(1.0),
        )
    }

    fn bump(&self) {
        self.revision
            .set(self.revision.get_untracked().wrapping_add(1));
    }

    fn after_change(&self) {
        let document = self.document.get_untracked();
        self.normalize_state(&document);
        let (focus, selection) = {
            let mut state = self.state.borrow_mut();
            state.ensure_visible = true;
            state.caret_line = None;
            let selection = state.anchor.and_then(|anchor| {
                (anchor != state.focus).then_some(DocumentSelection::new(anchor, state.focus))
            });
            (state.focus, selection)
        };
        self.focused_block
            .set(document.find_block(focus.block_id).map(|block| block.id));
        self.selection.set(selection);
        self.bump();
    }

    fn normalize_state(&self, document: &Document) {
        if document.blocks.is_empty() {
            let mut state = self.state.borrow_mut();
            state.focus = BlockPosition::new(BlockId(0), 0);
            state.anchor = None;
            return;
        }
        let first = document
            .first_position()
            .unwrap_or_else(|| BlockPosition::new(BlockId(0), 0));
        let mut state = self.state.borrow_mut();
        state.focus = document.clamp_position(state.focus).unwrap_or(first);
        if let Some(anchor) = state.anchor {
            state.anchor = document.clamp_position(anchor);
        }
        if state.anchor == Some(state.focus) {
            state.anchor = None;
        }
    }

    fn edit_document(&self, f: impl FnOnce(&mut Document, BlockPosition) -> Option<BlockPosition>) {
        let current = self.document.get_untracked();
        let focus = self.state.borrow().focus;
        let mut next = current.clone();
        let result = f(&mut next, focus);
        if next != current {
            {
                let mut state = self.state.borrow_mut();
                state.undo.push(current);
                if state.undo.len() > HISTORY_LIMIT {
                    state.undo.remove(0);
                }
                state.redo.clear();
                state.anchor = None;
                state.goal_x = None;
                if let Some(position) = result {
                    state.focus = position;
                }
            }
            self.document.set(next);
        }
        self.after_change();
    }

    fn insert_text(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        let selection = self.selection();
        let focus = self.state.borrow().focus;
        self.edit_document(move |document, _| {
            if let Some(selection) = selection {
                document.replace_selection(selection, text)
            } else {
                document.insert_text(focus, text)
            }
        });
        self.clear_preedit();
    }

    fn delete_backward(&self) {
        if let Some(selection) = self.selection() {
            self.edit_document(move |document, _| document.delete_selection(selection));
            return;
        }
        self.edit_document(|document, focus| {
            let position = document.clamp_position(focus)?;
            if position.offset > 0 {
                let block = document.find_block_mut(position.block_id)?;
                block.apply_edit(position.offset - 1..position.offset, "");
                Some(BlockPosition::new(position.block_id, position.offset - 1))
            } else {
                document.merge_with_previous(position.block_id)
            }
        });
    }

    fn delete_forward(&self) {
        if let Some(selection) = self.selection() {
            self.edit_document(move |document, _| document.delete_selection(selection));
            return;
        }
        self.edit_document(|document, focus| {
            let position = document.clamp_position(focus)?;
            let len = document.block_len(position.block_id)?;
            if position.offset < len {
                let block = document.find_block_mut(position.block_id)?;
                block.apply_edit(position.offset..position.offset + 1, "");
                Some(position)
            } else {
                document.merge_with_next(position.block_id)
            }
        });
    }

    fn insert_newline(&self) {
        let selection = self.selection();
        self.edit_document(move |document, focus| {
            let position = match selection {
                Some(selection) => document.delete_selection(selection)?,
                None => document.clamp_position(focus).unwrap_or_else(|| {
                    let id = document.add_block(BlockType::Paragraph, "");
                    BlockPosition::new(id, 0)
                }),
            };
            document.split_block(position)
        });
        self.clear_preedit();
    }

    fn move_to(&self, position: BlockPosition, extend: bool, keep_goal: bool) {
        let document = self.document.get_untracked();
        let Some(position) = document.clamp_position(position) else {
            return;
        };
        {
            let mut state = self.state.borrow_mut();
            if extend {
                let old = state.focus;
                if state.anchor.is_none() && old != position {
                    state.anchor = Some(old);
                }
                state.focus = position;
                if state.anchor == Some(position) {
                    state.anchor = None;
                }
            } else {
                state.focus = position;
                state.anchor = None;
                state.goal_x = None;
            }
            if !keep_goal {
                state.goal_x = None;
            }
        }
        self.after_change();
    }

    fn move_horizontal(&self, ctx: &mut EventCtx, forward: bool, extend: bool) {
        let document = self.document.get_untracked();
        let focus = self.state.borrow().focus;
        let Some(focus) = document.clamp_position(focus) else {
            return;
        };
        let target = if !extend && let Some(selection) = self.selection() {
            let (start, end) = document.ordered_selection(selection);
            if forward { end } else { start }
        } else if forward {
            document.next_position(focus).unwrap_or(focus)
        } else {
            document.previous_position(focus).unwrap_or(focus)
        };
        let layout = self.event_layout(ctx, &document);
        let line = layout
            .block(target.block_id)
            .and_then(|block| horizontal_line(block, target.offset, forward));
        self.move_to(target, extend, false);
        self.state.borrow_mut().caret_line = line;
        self.bump();
    }

    fn move_vertical(&self, ctx: &mut EventCtx, down: bool, extend: bool) {
        let document = self.document.get_untracked();
        let focus = self.state.borrow().focus;
        let Some(focus) = document.clamp_position(focus) else {
            return;
        };
        let layout = self.event_layout(ctx, &document);
        let Some(block) = layout.block(focus.block_id) else {
            return;
        };

        let mut entries = Vec::new();
        for block in &layout.blocks {
            if block.lines.is_empty() {
                entries.push((
                    block.id,
                    usize::MAX,
                    block.rect.top(),
                    block.rect.size.height,
                ));
            } else {
                for (index, line) in block.lines.iter().enumerate() {
                    entries.push((
                        block.id,
                        index,
                        block.content_rect.top() + line.top,
                        line.height,
                    ));
                }
            }
        }
        let preferred_line = self.state.borrow().caret_line;
        let current_line = caret_line(block, focus.offset, preferred_line).unwrap_or(usize::MAX);
        let Some(current_index) = entries
            .iter()
            .position(|(id, line, _, _)| *id == focus.block_id && *line == current_line)
        else {
            return;
        };

        let goal_x = self
            .state
            .borrow()
            .goal_x
            .unwrap_or_else(|| block.caret_rect(focus.offset).left());

        let target_index = if down {
            if current_index + 1 >= entries.len() {
                if let Some(last) = document.last_position() {
                    self.move_to(last, extend, false);
                }
                return;
            }
            current_index + 1
        } else {
            if current_index == 0 {
                if let Some(first) = document.first_position() {
                    self.move_to(first, extend, false);
                }
                return;
            }
            current_index - 1
        };

        let (_, target_line, top, height) = entries[target_index];
        let Some(position) = layout.hit_test(goal_x, top + height * 0.5) else {
            return;
        };
        self.move_to(position, extend, true);
        {
            let mut state = self.state.borrow_mut();
            state.goal_x = Some(goal_x);
            state.caret_line = (target_line != usize::MAX).then_some(target_line);
        }
        self.bump();
    }

    fn move_line_edge(&self, ctx: &mut EventCtx, home: bool, extend: bool) {
        let document = self.document.get_untracked();
        let focus = self.state.borrow().focus;
        let Some(focus) = document.clamp_position(focus) else {
            return;
        };
        let layout = self.event_layout(ctx, &document);
        let Some(block) = layout.block(focus.block_id) else {
            return;
        };
        let preferred_line = self.state.borrow().caret_line;
        let line_index = caret_line(block, focus.offset, preferred_line);
        let offset = line_index
            .and_then(|index| block.lines.get(index))
            .map(|line| {
                if home {
                    line.chars.start
                } else {
                    line.chars.end
                }
            })
            .unwrap_or_else(|| if home { 0 } else { block.text_len });
        self.move_to(BlockPosition::new(focus.block_id, offset), extend, false);
        self.state.borrow_mut().caret_line = line_index;
        self.bump();
    }

    fn select_all(&self) {
        let document = self.document.get_untracked();
        let selection = document.select_all();
        {
            let mut state = self.state.borrow_mut();
            state.focus = selection.focus;
            state.anchor = (selection.anchor != selection.focus).then_some(selection.anchor);
            state.goal_x = None;
        }
        self.after_change();
    }

    fn copy(&self, cut: bool, ctx: &mut EventCtx) {
        let document = self.document.get_untracked();
        let (fragment, text) = if let Some(selection) = self.selection() {
            (
                document.selection_fragment(selection),
                document.selection_text(selection),
            )
        } else {
            let focus = self.state.borrow().focus;
            let fragment = document.block_fragment(focus.block_id);
            let text = document
                .find_block(focus.block_id)
                .map(|block| block.plain_text())
                .unwrap_or_default();
            (fragment, text)
        };
        {
            let mut state = self.state.borrow_mut();
            state.clipboard = Some(fragment);
            state.clipboard_text = Some(text.clone());
        }
        if let Some(clip) = ctx.environment.get::<SharedClipboard>() {
            clip.0.set_text(&text);
        }
        if cut {
            if let Some(selection) = self.selection() {
                self.edit_document(move |document, _| document.delete_selection(selection));
            } else {
                let focus = self.state.borrow().focus;
                self.edit_document(move |document, _| {
                    let index = document.block_index(focus.block_id)?;
                    if document.blocks.len() == 1 {
                        let block = document.find_block_mut(focus.block_id)?;
                        let len = block.text_len();
                        block.apply_edit(0..len, "");
                        return Some(BlockPosition::new(focus.block_id, 0));
                    }
                    document.remove_block(focus.block_id)?;
                    if let Some(next) = document.blocks.get(index) {
                        Some(BlockPosition::new(next.id, 0))
                    } else {
                        document
                            .blocks
                            .last()
                            .map(|last| BlockPosition::new(last.id, last.text_len()))
                    }
                });
            }
        }
        self.bump();
    }

    fn paste(&self, ctx: &mut EventCtx) {
        let clipboard_text = ctx
            .environment
            .get::<SharedClipboard>()
            .and_then(|clip| clip.0.get_text());
        let (internal_fragment, internal_text) = {
            let state = self.state.borrow();
            (state.clipboard.clone(), state.clipboard_text.clone())
        };
        let internal_matches = internal_fragment.is_some()
            && match (&clipboard_text, &internal_text) {
                (Some(external), Some(internal)) => external == internal,
                (None, _) => true,
                (Some(_), None) => false,
            };

        if !internal_matches {
            if let Some(text) = clipboard_text {
                let selection = self.selection();
                let focus = self.state.borrow().focus;
                self.edit_document(move |document, _| match selection {
                    Some(selection) => document.replace_selection(selection, &text),
                    None => document.insert_text(focus, &text),
                });
                self.clear_preedit();
            }
            return;
        }

        let Some(fragment) = internal_fragment else {
            return;
        };
        let selection = self.selection();
        let focus = self.state.borrow().focus;
        self.edit_document(move |document, _| {
            let position = match selection {
                Some(selection) => document.delete_selection(selection)?,
                None => document.clamp_position(focus)?,
            };
            document.paste_fragment(position, &fragment)
        });
        self.clear_preedit();
    }

    fn undo(&self) {
        let previous = self.state.borrow_mut().undo.pop();
        let Some(previous) = previous else {
            return;
        };
        let current = self.document.get_untracked();
        {
            let mut state = self.state.borrow_mut();
            state.redo.push(current);
            state.anchor = None;
            state.goal_x = None;
        }
        self.document.set(previous);
        self.after_change();
    }

    fn redo(&self) {
        let next = self.state.borrow_mut().redo.pop();
        let Some(next) = next else {
            return;
        };
        let current = self.document.get_untracked();
        {
            let mut state = self.state.borrow_mut();
            state.undo.push(current);
            state.anchor = None;
            state.goal_x = None;
        }
        self.document.set(next);
        self.after_change();
    }

    fn clear_preedit(&self) {
        {
            let mut state = self.state.borrow_mut();
            if state.preedit.is_empty() && state.preedit_cursor.is_none() {
                return;
            }
            state.preedit.clear();
            state.preedit_cursor = None;
        }
        self.bump();
    }

    fn on_ime(&self, ime: &ImeEvent) {
        match ime {
            ImeEvent::Enabled => {}
            ImeEvent::Preedit { value, cursor } => {
                {
                    let mut state = self.state.borrow_mut();
                    state.preedit = value.clone();
                    state.preedit_cursor = *cursor;
                }
                self.bump();
            }
            ImeEvent::Commit(value) => {
                if value.is_empty() {
                    self.clear_preedit();
                } else {
                    self.insert_text(value);
                }
            }
            ImeEvent::Disabled => self.clear_preedit(),
            _ => {}
        }
    }

    fn on_clipboard_key(&self, key: &KeyEvent, ctx: &mut EventCtx) -> bool {
        if !self.state.borrow().preedit.is_empty() {
            return false;
        }
        let command =
            key.modifiers.contains(Modifiers::CONTROL) || key.modifiers.contains(Modifiers::SUPER);
        if !command {
            return false;
        }
        let Some(character) = key.key.char().map(|value| value.to_ascii_lowercase()) else {
            return false;
        };
        match character {
            'a' => self.select_all(),
            'c' => self.copy(false, ctx),
            'x' => self.copy(true, ctx),
            'v' => self.paste(ctx),
            'z' if key.modifiers.contains(Modifiers::SHIFT) => self.redo(),
            'z' => self.undo(),
            'y' => self.redo(),
            _ => return false,
        }
        true
    }

    fn on_key(&self, key: &KeyEvent, ctx: &mut EventCtx) {
        if !self.state.borrow().preedit.is_empty() {
            ctx.handled();
            return;
        }
        let command =
            key.modifiers.contains(Modifiers::CONTROL) || key.modifiers.contains(Modifiers::SUPER);
        let shift = key.modifiers.contains(Modifiers::SHIFT);
        match key.key {
            Key::ENTER => self.insert_newline(),
            Key::BACKSPACE => self.delete_backward(),
            Key::DELETE => self.delete_forward(),
            Key::ARROW_LEFT => self.move_horizontal(ctx, false, shift),
            Key::ARROW_RIGHT => self.move_horizontal(ctx, true, shift),
            Key::ARROW_UP if !command => self.move_vertical(ctx, false, shift),
            Key::ARROW_DOWN if !command => self.move_vertical(ctx, true, shift),
            Key::HOME if command => {
                let document = self.document.get_untracked();
                if let Some(first) = document.first_position() {
                    self.move_to(first, shift, false);
                }
            }
            Key::END if command => {
                let document = self.document.get_untracked();
                if let Some(last) = document.last_position() {
                    self.move_to(last, shift, false);
                }
            }
            Key::HOME => self.move_line_edge(ctx, true, shift),
            Key::END => self.move_line_edge(ctx, false, shift),
            Key::SPACE if !command => self.insert_text(" "),
            _ => {
                if !command && let Some(character) = key.key.char() {
                    self.insert_text(&character.to_string());
                }
            }
        }
        ctx.handled();
    }

    fn on_pointer_down(&self, ctx: &mut EventCtx, position: Point, modifiers: Modifiers) {
        let document = self.document.get_untracked();
        let layout = self.event_layout(ctx, &document);
        let content = self.content_rect(ctx.rect);
        let scroll_y = self.state.borrow().scroll_y;
        let local_x = position.x - content.left();
        let local_y = position.y - content.top() + scroll_y;
        let Some(position) = layout.hit_test(local_x, local_y) else {
            return;
        };
        let line = layout
            .block(position.block_id)
            .and_then(|block| block.line_at_y(local_y));
        let extend = modifiers.contains(Modifiers::SHIFT);
        {
            let mut state = self.state.borrow_mut();
            if !extend {
                state.anchor = Some(position);
            } else if state.anchor.is_none() {
                state.anchor = Some(state.focus);
            }
            state.focus = position;
            state.dragging = true;
            state.goal_x = None;
        }
        self.after_change();
        self.state.borrow_mut().caret_line = line;
        self.bump();
        ctx.request_focus();
        ctx.capture_pointer();
        ctx.handled();
    }

    fn on_pointer_move(&self, ctx: &mut EventCtx, position: Point) {
        if !self.state.borrow().dragging {
            return;
        }
        let document = self.document.get_untracked();
        let layout = self.event_layout(ctx, &document);
        let content = self.content_rect(ctx.rect);
        let scroll_y = self.state.borrow().scroll_y;
        let local_y = position.y - content.top() + scroll_y;
        let Some(position) = layout.hit_test(position.x - content.left(), local_y) else {
            return;
        };
        let line = layout
            .block(position.block_id)
            .and_then(|block| block.line_at_y(local_y));
        {
            let mut state = self.state.borrow_mut();
            if state.anchor.is_none() {
                state.anchor = Some(state.focus);
            }
            state.focus = position;
            state.goal_x = None;
        }
        self.after_change();
        self.state.borrow_mut().caret_line = line;
        self.bump();
        ctx.handled();
    }

    fn on_pointer_up(&self, ctx: &mut EventCtx) {
        self.state.borrow_mut().dragging = false;
        ctx.release_pointer();
        ctx.handled();
    }

    fn on_wheel(&self, ctx: &mut EventCtx, delta: Vec2, modifiers: Modifiers) {
        let document = self.document.get_untracked();
        let layout = self.event_layout(ctx, &document);
        let content = self.content_rect(ctx.rect);
        let max_scroll = (layout.height - content.size.height).max(0.0);
        if max_scroll <= 0.0 {
            return;
        }
        let delta_y = if modifiers.contains(Modifiers::SHIFT) {
            delta.x
        } else {
            delta.y
        };
        let mut state = self.state.borrow_mut();
        let next = (state.scroll_y - delta_y).clamp(0.0, max_scroll);
        if next != state.scroll_y {
            state.scroll_y = next;
            state.ensure_visible = false;
            drop(state);
            self.bump();
            ctx.handled();
        }
    }

    fn event_layout(&self, ctx: &mut EventCtx, document: &Document) -> LaidOutDocument {
        let theme = ctx.environment.get::<Theme>().unwrap_or_else(Theme::dark);
        let base_size = self.base_size(&theme);
        let content = self.content_rect(ctx.rect);
        let mut measure = EventMeasure { ctx };
        LaidOutDocument::layout(document, content.size.width, base_size, &mut measure)
    }

    fn paint_layout(&self, ctx: &mut PaintCtx, document: &Document) -> (LaidOutDocument, f32) {
        let theme = ctx.theme();
        let base_size = self.base_size(&theme);
        let content = self.content_rect(ctx.rect);
        let mut measure = PaintMeasure { ctx };
        let layout = LaidOutDocument::layout(document, content.size.width, base_size, &mut measure);
        (layout, base_size)
    }
}

struct EventMeasure<'a, 'b> {
    ctx: &'a mut EventCtx<'b>,
}

impl BlockMeasurer for EventMeasure<'_, '_> {
    fn caret_positions(&mut self, text: &str, style: &BlockTextStyle) -> Vec<f32> {
        self.ctx.caret_positions_styled_spaced(
            text,
            style.size,
            style.weight,
            0.0,
            style.family.as_deref(),
            style.style,
        )
    }
}

struct PaintMeasure<'a, 'b> {
    ctx: &'a mut PaintCtx<'b>,
}

impl BlockMeasurer for PaintMeasure<'_, '_> {
    fn caret_positions(&mut self, text: &str, style: &BlockTextStyle) -> Vec<f32> {
        self.ctx.caret_positions_styled_spaced(
            text,
            style.size,
            style.weight,
            0.0,
            style.family.as_deref(),
            style.style,
        )
    }
}

impl View for BlockEditor {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn focusable(&self) -> bool {
        true
    }

    fn accepts_text_input(&self) -> bool {
        true
    }

    fn hit_testable(&self) -> bool {
        true
    }

    fn captures_pointer(&self) -> bool {
        true
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn clip_self(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..self.style
        }
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Text
    }

    fn ime_caret_rect(&self) -> Option<Rect> {
        self.state.borrow().ime_rect
    }

    fn reuse_state(&self, old: &Self) {
        if !Rc::ptr_eq(&self.state, &old.state) {
            *self.state.borrow_mut() = old.state.borrow().clone();
        }
        self.revision.set(old.revision.get_untracked());
    }

    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let _ = self.document.get();
        let _ = self.revision.get();
        Some(Vec::new())
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            return;
        }
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                modifiers,
                ..
            }) => self.on_pointer_down(ctx, *pos, *modifiers),
            Event::Pointer(PointerEvent::Move { pos, .. }) => self.on_pointer_move(ctx, *pos),
            Event::Pointer(PointerEvent::Wheel {
                delta, modifiers, ..
            }) => self.on_wheel(ctx, *delta, *modifiers),
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                ..
            }) => self.on_pointer_up(ctx),
            Event::Pointer(PointerEvent::Cancel) => {
                self.state.borrow_mut().dragging = false;
            }
            Event::Key(key) => {
                if !self.on_clipboard_key(key, ctx) {
                    self.on_key(key, ctx);
                } else {
                    ctx.handled();
                }
            }
            Event::Ime(ime) => {
                self.on_ime(ime);
                ctx.handled();
            }
            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let rect = ctx.rect;
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        let document = self.document.get_untracked();
        self.normalize_state(&document);
        let theme = ctx.theme();
        let content = self.content_rect(rect);
        let (layout, _) = self.paint_layout(ctx, &document);
        let (focus, anchor, preedit, preedit_cursor, ensure_visible, preferred_line) = {
            let state = self.state.borrow();
            (
                state.focus,
                state.anchor,
                state.preedit.clone(),
                state.preedit_cursor,
                state.ensure_visible,
                state.caret_line,
            )
        };

        let max_scroll = (layout.height - content.size.height).max(0.0);
        let mut scroll_y = self.state.borrow().scroll_y;
        if ensure_visible && let Some(block) = layout.block(focus.block_id) {
            let (top, bottom) = match caret_line(block, focus.offset, preferred_line) {
                Some(index) => {
                    let line = &block.lines[index];
                    (
                        block.content_rect.top() + line.top,
                        block.content_rect.top() + line.top + line.height,
                    )
                }
                None => (
                    block.content_rect.top(),
                    block.content_rect.top() + block.style.text.line_height(),
                ),
            };
            let margin = 2.0;
            if top < scroll_y + margin {
                scroll_y = (top - margin).max(0.0);
            } else if bottom + margin > scroll_y + content.size.height {
                scroll_y = (bottom + margin - content.size.height).max(0.0);
            }
        }
        scroll_y = scroll_y.clamp(0.0, max_scroll);
        {
            let mut state = self.state.borrow_mut();
            state.scroll_y = scroll_y;
            state.ensure_visible = false;
        }
        let origin = Point::new(content.left(), content.top() - scroll_y);

        if let Some(background) = self.background {
            ctx.quad(rect, background, 0.0);
        } else if theme.background.a > 0.0 {
            ctx.quad(rect, theme.background, 0.0);
        }

        ctx.push_clip(rect);

        for block in &layout.blocks {
            let block_rect = translate(block.rect, origin);
            if let Some(background) = block.style.background {
                ctx.quad(block_rect, background, theme.radius.max(2.0));
            }
            if let Some(decoration) = &block.decoration {
                let decoration_rect = translate(decoration.rect, origin);
                paint_decoration(
                    ctx,
                    &theme,
                    decoration.kind.clone(),
                    decoration_rect,
                    block.style.text.size,
                );
            }
            if let BlockType::Table { rows, cols } = block.block_type {
                paint_table_grid(ctx, block_rect, rows, cols, &theme);
            }
        }

        if let Some(anchor) = anchor {
            let selection = DocumentSelection::new(anchor, focus);
            for selection_rect in layout.selection_rects(selection) {
                let selection_rect = translate(selection_rect, origin);
                ctx.quad(
                    selection_rect,
                    Color::rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.24),
                    1.0,
                );
            }
        }

        let composing = !preedit.is_empty();
        let mut ime_rect = None;

        for block in &layout.blocks {
            let is_focused = block.id == focus.block_id;
            let focused_line = is_focused
                .then(|| caret_line(block, focus.offset, preferred_line))
                .flatten();
            let text_color = block.style.text_color.unwrap_or(theme.text);
            let line_origin = Point::new(
                content.left() + block.content_rect.left(),
                origin.y + block.content_rect.top(),
            );

            for (line_index, line) in block.lines.iter().enumerate() {
                let line_has_caret = composing && focused_line == Some(line_index);
                let caret_offset = focus.offset.clamp(line.chars.start, line.chars.end);
                let preedit_x = if line_has_caret {
                    line_origin.x + line.x_at(caret_offset)
                } else {
                    0.0
                };
                let preedit_width = if line_has_caret {
                    ctx.text_width_styled_spaced(
                        &preedit,
                        block.style.text.size,
                        block.style.text.weight,
                        0.0,
                        block.style.text.family.as_deref(),
                        block.style.text.style,
                    )
                } else {
                    0.0
                };
                let baseline = line_origin.y + line.baseline;

                for run in &line.runs {
                    let x = line_origin.x + run.x;
                    if !line_has_caret || run.chars.end <= caret_offset {
                        draw_run(ctx, run, x, baseline, text_color);
                    } else if run.chars.start >= caret_offset {
                        draw_run(ctx, run, x + preedit_width, baseline, text_color);
                    } else {
                        let split = caret_offset - run.chars.start;
                        let byte = char_byte_index(&run.text, split);
                        let (prefix, suffix) = run.text.split_at(byte);
                        if !prefix.is_empty() {
                            ctx.draw_text_styled_spaced(
                                prefix,
                                (x, baseline),
                                run.style.size,
                                run.style.weight,
                                0.0,
                                run.style.family.as_deref(),
                                run.style.style,
                                text_color,
                            );
                        }
                        if !suffix.is_empty() {
                            let prefix_width = ctx.text_width_styled_spaced(
                                prefix,
                                run.style.size,
                                run.style.weight,
                                0.0,
                                run.style.family.as_deref(),
                                run.style.style,
                            );
                            ctx.draw_text_styled_spaced(
                                suffix,
                                (x + prefix_width + preedit_width, baseline),
                                run.style.size,
                                run.style.weight,
                                0.0,
                                run.style.family.as_deref(),
                                run.style.style,
                                text_color,
                            );
                        }
                    }
                }

                for object in &line.objects {
                    let shift = if line_has_caret && object.chars.start >= caret_offset {
                        preedit_width
                    } else {
                        0.0
                    };
                    paint_object(
                        ctx,
                        &theme,
                        &object.element,
                        Rect::from_xywh(
                            line_origin.x + object.rect.left() + shift,
                            line_origin.y + object.rect.top(),
                            object.rect.size.width,
                            object.rect.size.height,
                        ),
                        block.style.text.size,
                    );
                }

                if line_has_caret {
                    let cursor_byte = preedit_cursor
                        .map(|(cursor, _)| clamp_char_boundary(&preedit, cursor))
                        .unwrap_or(preedit.len());
                    let cursor_prefix = &preedit[..cursor_byte];
                    let cursor_width = ctx.text_width_styled_spaced(
                        cursor_prefix,
                        block.style.text.size,
                        block.style.text.weight,
                        0.0,
                        block.style.text.family.as_deref(),
                        block.style.text.style,
                    );
                    let caret_x = preedit_x + cursor_width;
                    if ctx.state.focused {
                        ime_rect = Some(Rect::from_xywh(
                            caret_x,
                            line_origin.y + line.top,
                            1.0,
                            line.height.max(1.0),
                        ));
                        ctx.quad(
                            Rect::from_xywh(
                                caret_x,
                                line_origin.y + line.top,
                                1.5,
                                line.height.max(1.0),
                            ),
                            theme.accent,
                            0.0,
                        );
                    }
                    ctx.draw_text_styled_spaced(
                        &preedit,
                        (preedit_x, baseline),
                        block.style.text.size,
                        block.style.text.weight,
                        0.0,
                        block.style.text.family.as_deref(),
                        block.style.text.style,
                        theme.accent,
                    );
                    ctx.quad(
                        Rect::from_xywh(
                            preedit_x,
                            baseline + block.style.text.size * 0.15,
                            preedit_width,
                            1.0,
                        ),
                        theme.accent,
                        0.0,
                    );
                } else if is_focused && focused_line == Some(line_index) && ctx.state.focused {
                    let caret_x = line_origin.x + line.x_at(focus.offset);
                    let caret_y = line_origin.y + line.top;
                    let caret_height = line.height.max(1.0);
                    ime_rect = Some(Rect::from_xywh(caret_x, caret_y, 1.0, caret_height));
                    ctx.quad(
                        Rect::from_xywh(caret_x, caret_y, 1.5, caret_height),
                        theme.accent,
                        0.0,
                    );
                }
            }

            if is_focused && ctx.state.focused && block.lines.is_empty() {
                let height = block.style.text.line_height();
                let caret_x = content.left() + block.content_rect.left();
                let caret_y = origin.y + block.content_rect.top();
                ime_rect = Some(Rect::from_xywh(caret_x, caret_y, 1.0, height));
                ctx.quad(
                    Rect::from_xywh(caret_x, caret_y, 1.5, height),
                    theme.accent,
                    0.0,
                );
            }
        }

        if max_scroll > 0.0 {
            paint_scrollbar(ctx, &theme, rect, content, scroll_y, max_scroll);
        }
        self.state.borrow_mut().ime_rect = ime_rect;
        ctx.pop_clip();
    }
}

fn paint_scrollbar(
    ctx: &mut PaintCtx,
    theme: &Theme,
    editor_rect: Rect,
    content: Rect,
    scroll_y: f32,
    max_scroll: f32,
) {
    let viewport = content.size.height;
    let total = viewport + max_scroll;
    let thumb_height = (viewport * viewport / total).clamp(24.0, viewport);
    let travel = (viewport - thumb_height).max(0.0);
    let x = editor_rect.right() - 5.0;
    ctx.quad(
        Rect::from_xywh(x, content.top(), 3.0, viewport),
        Color::rgba(theme.border.r, theme.border.g, theme.border.b, 0.35),
        1.5,
    );
    let thumb_y = content.top() + travel * (scroll_y / max_scroll);
    ctx.quad(
        Rect::from_xywh(x, thumb_y, 3.0, thumb_height),
        Color::rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.7),
        1.5,
    );
}

fn translate(rect: Rect, origin: Point) -> Rect {
    Rect::from_xywh(
        origin.x + rect.left(),
        origin.y + rect.top(),
        rect.size.width,
        rect.size.height,
    )
}

fn draw_run(ctx: &mut PaintCtx, run: &sequoia_text::TextRun, x: f32, baseline: f32, color: Color) {
    ctx.draw_text_styled_spaced(
        &run.text,
        (x, baseline),
        run.style.size,
        run.style.weight,
        0.0,
        run.style.family.as_deref(),
        run.style.style,
        color,
    );
}

fn paint_decoration(
    ctx: &mut PaintCtx,
    theme: &Theme,
    kind: DecorationKind,
    rect: Rect,
    font_size: f32,
) {
    match kind {
        DecorationKind::Bullet => {
            ctx.circle(
                rect.center(),
                rect.size.width.max(2.0) * 0.5,
                theme.text_secondary,
            );
        }
        DecorationKind::Number(number) => {
            let width =
                ctx.text_width_weighted(&number, font_size, sequoia_view::FontWeight::NORMAL);
            let x = rect.right() - width - 4.0;
            let baseline = rect.top() + rect.size.height * 0.5 + font_size * 0.35;
            ctx.draw_text(&number, (x, baseline), font_size, theme.text_secondary);
        }
        DecorationKind::Todo { checked } => {
            let background = Color::rgba(0.0, 0.0, 0.0, 0.0);
            ctx.bordered_quad(rect, background, 3.0, 1.5, theme.border);
            if checked {
                let inset = rect.size.width * 0.2;
                ctx.quad(
                    Rect::from_xywh(
                        rect.left() + inset,
                        rect.top() + inset,
                        rect.size.width - inset * 2.0,
                        rect.size.height - inset * 2.0,
                    ),
                    theme.accent,
                    2.0,
                );
            }
        }
        DecorationKind::QuoteBar => {
            ctx.quad(rect, theme.text_secondary, rect.size.width.max(1.0) * 0.5);
        }
        DecorationKind::Divider => {
            ctx.quad(rect, theme.border, 0.0);
        }
        DecorationKind::Callout { icon } => {
            if !icon.is_empty() {
                let baseline = rect.top() + font_size * 1.35;
                ctx.draw_text(
                    &icon,
                    (rect.left() + 14.0, baseline),
                    font_size,
                    theme.accent,
                );
            } else {
                ctx.circle(
                    Point::new(rect.left() + 20.0, rect.top() + font_size * 0.9),
                    4.0,
                    theme.accent,
                );
            }
        }
        DecorationKind::CodeBlock | DecorationKind::Table => {}
    }
}

fn paint_table_grid(ctx: &mut PaintCtx, rect: Rect, rows: usize, cols: usize, theme: &Theme) {
    if rows == 0 || cols == 0 {
        return;
    }
    ctx.bordered_quad(
        rect,
        Color::rgba(0.0, 0.0, 0.0, 0.0),
        3.0,
        1.0,
        theme.border,
    );
    for row in 1..rows {
        let y = rect.top() + rect.size.height * row as f32 / rows as f32;
        ctx.quad(
            Rect::from_xywh(rect.left(), y, rect.size.width, 1.0),
            theme.border,
            0.0,
        );
    }
    for col in 1..cols {
        let x = rect.left() + rect.size.width * col as f32 / cols as f32;
        ctx.quad(
            Rect::from_xywh(x, rect.top(), 1.0, rect.size.height),
            theme.border,
            0.0,
        );
    }
}

fn paint_object(
    ctx: &mut PaintCtx,
    theme: &Theme,
    element: &InlineElement,
    rect: Rect,
    font_size: f32,
) {
    let baseline = rect.top() + rect.size.height * 0.5 + font_size * 0.35;
    match element {
        InlineElement::Tag(label) => {
            ctx.bordered_quad(
                rect,
                Color::rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.18),
                3.0,
                0.0,
                theme.accent,
            );
            ctx.draw_text(
                label,
                (rect.left() + 4.0, baseline),
                font_size * 0.9,
                theme.accent,
            );
        }
        InlineElement::Mention(label) => {
            ctx.bordered_quad(
                rect,
                Color::rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.22),
                3.0,
                0.0,
                theme.accent,
            );
            ctx.draw_text(
                label,
                (rect.left() + 4.0, baseline),
                font_size * 0.9,
                theme.accent,
            );
        }
        InlineElement::Status(label) => {
            let color = Color::rgb8(74, 222, 128);
            ctx.bordered_quad(
                rect,
                Color::rgba(color.r, color.g, color.b, 0.2),
                3.0,
                0.0,
                color,
            );
            ctx.draw_text(label, (rect.left() + 4.0, baseline), font_size * 0.9, color);
        }
        InlineElement::Math(label) => {
            ctx.bordered_quad(
                rect,
                Color::rgba(theme.surface.r, theme.surface.g, theme.surface.b, 0.7),
                2.0,
                0.0,
                theme.border,
            );
            ctx.draw_text(
                label,
                (rect.left() + 4.0, baseline),
                font_size * 0.9,
                theme.text,
            );
        }
        InlineElement::Image { alt, .. } => {
            ctx.bordered_quad(rect, theme.surface, 4.0, 1.0, theme.border);
            if !alt.is_empty() {
                ctx.draw_text(
                    alt,
                    (rect.left() + 4.0, baseline),
                    font_size * 0.8,
                    theme.text_secondary,
                );
            }
        }
        InlineElement::Icon(label) => {
            ctx.circle(
                Point::new(rect.left() + rect.size.width * 0.5, rect.center().y),
                rect.size.width.min(rect.size.height) * 0.4,
                theme.accent,
            );
            if !label.is_empty() {
                ctx.draw_text(
                    label,
                    (rect.left() + 2.0, baseline),
                    font_size * 0.85,
                    theme.text,
                );
            }
        }
    }
}

fn horizontal_line(block: &LaidOutBlock, offset: usize, forward: bool) -> Option<usize> {
    let boundary = if forward {
        block
            .lines
            .iter()
            .position(|line| line.chars.start == offset)
    } else {
        block
            .lines
            .iter()
            .rposition(|line| line.chars.end == offset)
    };
    boundary.or_else(|| block.line_for_offset(offset))
}

fn caret_line(block: &LaidOutBlock, offset: usize, preferred: Option<usize>) -> Option<usize> {
    if let Some(index) = preferred
        && let Some(line) = block.lines.get(index)
        && offset >= line.chars.start
        && offset <= line.chars.end
    {
        return Some(index);
    }
    block.line_for_offset(offset)
}

fn char_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(byte, _)| byte)
        .unwrap_or(text.len())
}

fn clamp_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}
