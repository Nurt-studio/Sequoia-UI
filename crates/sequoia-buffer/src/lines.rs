#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineIndex {
    starts: Vec<usize>,
    len: usize,
}

fn push_unique(out: &mut Vec<usize>, v: usize) {
    if out.last() != Some(&v) {
        out.push(v);
    }
}

impl LineIndex {
    pub fn from_str(text: &str) -> Self {
        let mut starts = vec![0usize];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        Self {
            starts,
            len: text.len(),
        }
    }

    pub fn apply_edit(&mut self, start: usize, deleted_len: usize, inserted: &str) {
        debug_assert!(start <= self.len, "edit start past end");
        let start = start.min(self.len);
        let deleted_len = deleted_len.min(self.len - start);
        let ins_len = inserted.len();
        let old_len = self.len;
        let had_start = self.starts.binary_search(&start).is_ok();
        let old_trailing_nl = old_len > 0 && self.starts.last() == Some(&old_len);

        let need = start == 0 || self.starts.binary_search(&start).is_ok();

        let mut next = Vec::with_capacity(self.starts.len() + 1);
        for &s in &self.starts {
            if s < start {
                next.push(s);
            } else {
                break;
            }
        }
        if need {
            push_unique(&mut next, start);
        }
        for (i, b) in inserted.bytes().enumerate() {
            if b == b'\n' {
                push_unique(&mut next, start + i + 1);
            }
        }
        let delta = ins_len as isize - deleted_len as isize;
        for &s in &self.starts {
            if s >= start + deleted_len && !(deleted_len == 0 && s == start) {
                if deleted_len > 0 && s == start + deleted_len {
                    let reopened = if !inserted.is_empty() {
                        inserted.as_bytes().last() == Some(&b'\n')
                    } else {
                        start > 0 && had_start
                    };
                    if !reopened {
                        continue;
                    }
                }
                push_unique(&mut next, ((s as isize) + delta) as usize);
            }
        }
        let new_len = old_len - deleted_len + ins_len;

        let suffix_nonempty = start + deleted_len < old_len;

        let new_ends_with_nl = if suffix_nonempty {
            old_trailing_nl
        } else if !inserted.is_empty() {
            inserted.as_bytes().last() == Some(&b'\n')
        } else {
            start > 0 && had_start
        };
        while new_len > 0 && next.last() == Some(&new_len) && !new_ends_with_nl {
            next.pop();
        }
        self.starts = next;
        self.len = new_len;
    }

    pub fn starts(&self) -> &[usize] {
        &self.starts
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    pub fn line_of(&self, offset: usize) -> usize {
        let offset = offset.min(self.len);
        self.starts
            .partition_point(|&s| s <= offset)
            .saturating_sub(1)
            .min(self.starts.len().saturating_sub(1))
    }

    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let line = self.line_of(offset);
        let offset = offset.min(self.len);
        (line, offset.saturating_sub(self.starts[line]))
    }

    pub fn line_start(&self, line: usize) -> usize {
        let line = line.min(self.starts.len().saturating_sub(1));
        self.starts[line]
    }

    pub fn line_range(&self, line: usize) -> (usize, usize) {
        let line = line.min(self.starts.len().saturating_sub(1));
        let start = self.starts[line];
        let end = match self.starts.get(line + 1) {
            Some(&next) => next.saturating_sub(1),
            None => self.len,
        };
        (start, end.max(start))
    }

    pub fn offset_of(&self, line: usize, col_bytes: usize) -> usize {
        let (start, end) = self.line_range(line);
        (start + col_bytes).min(end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(text: &str) {
        let idx = LineIndex::from_str(text);
        let mut expect = vec![0usize];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                expect.push(i + 1);
            }
        }
        assert_eq!(idx.starts(), expect, "from_str({text:?})");
        assert_eq!(idx.len(), text.len());
    }

    #[test]
    fn from_str_models() {
        check("");
        check("a");
        check("a\n");
        check("\n");
        check("\n\n");
        check("a\nb\nc");
        check("a\nb\nc\n");
        check("yellow\ngoose\nself");
    }

    #[test]
    fn queries() {
        let idx = LineIndex::from_str("ab\ncde\n\nf");
        assert_eq!(idx.line_count(), 4);
        assert_eq!(idx.line_of(0), 0);
        assert_eq!(idx.line_of(3), 1);
        assert_eq!(idx.line_col(4), (1, 1));
        assert_eq!(idx.line_range(1), (3, 6));
        assert_eq!(idx.line_range(2), (7, 7));
        assert_eq!(idx.line_range(3), (8, 9));
        assert_eq!(idx.offset_of(1, 99), 6);
        assert_eq!(idx.offset_of(99, 0), 8);

        assert_eq!(idx.line_of(10_000), 3);
        assert_eq!(idx.line_col(10_000), (3, 1));
    }

    fn apply_check(initial: &str, start: usize, delete_len: usize, insert: &str) {
        let mut ref_text = initial.to_string();
        ref_text.replace_range(start..start + delete_len, insert);
        let mut idx = LineIndex::from_str(initial);
        idx.apply_edit(start, delete_len, insert);
        let fresh = LineIndex::from_str(&ref_text);
        assert_eq!(
            idx,
            fresh,
            "edit [{start}..{}] + {insert:?} in {initial:?} -> {ref_text:?}",
            start + delete_len,
        );
    }

    #[test]
    fn edits_mid_line() {
        apply_check("hello world", 5, 0, ",");
        apply_check("hello world", 5, 6, "");
        apply_check("hello", 5, 0, " world");
        apply_check("hello world", 0, 5, "bye");
        apply_check("abc", 0, 3, "");
        apply_check("", 0, 0, "x");
        apply_check("", 0, 0, "");
    }

    #[test]
    fn edits_crossing_lines() {
        apply_check("a\nb\nc", 2, 0, "X\nY");
        apply_check("a\nb\nc", 2, 2, "");
        apply_check("a\nb\nc", 0, 4, "");
        apply_check("a\nb", 2, 1, "");
        apply_check("a\nb", 1, 2, "");
        apply_check("ab\ncd", 2, 2, "X");
        apply_check("ab\ncd", 3, 0, "\n");
        apply_check("ab", 2, 0, "\n");
        apply_check("ab\n", 3, 0, "cd\n");
        apply_check("a\nb\n", 2, 2, "X\n");
        apply_check("a\nb\nc\n", 4, 2, "xyz\n123\n");
    }

    #[test]
    fn edits_unicode() {
        apply_check("yellow\ngoose\nself", 10, 0, "!\n?");
        apply_check("a👨‍👩‍👧b\nc", 0, 1, "");
        apply_check("a👨‍👩‍👧b\nc", 1, 18, "");
        apply_check("\n", 0, 1, "");
        apply_check("\n\n", 1, 0, "x");
    }

    #[test]
    fn deleting_trailing_newline_drops_dangling_start() {
        apply_check("a\nb\n", 3, 1, "");
        apply_check("a\nb\n", 0, 4, "");
        apply_check("a\nb", 2, 1, "");

        apply_check("a\nb\n", 0, 0, "X");
        apply_check("a\nb\n", 2, 0, "X\n");
        apply_check("a\n", 1, 0, "b");
    }
}
