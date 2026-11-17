//! Ranges of the text being parsed.

/// A range of the text being parsed, by byte offsets into it. `@microsoft/tsdoc`'s TextRange keeps
/// its buffer and counts UTF-16 code units; this one is plain offsets into the parsed `&str`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextRange {
    /// Where it starts.
    pub pos: usize,
    /// Where it ends, past its last byte.
    pub end: usize,
}

/// A line and column, from 1, as TSDoc reports them in its messages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextLocation {
    /// The line, from 1.
    pub line: usize,
    /// The column, from 1, in UTF-16 code units as TSDoc counts them, with a tab one column.
    pub column: usize,
}

impl TextRange {
    /// The empty range at the start.
    pub const EMPTY: TextRange = TextRange { pos: 0, end: 0 };

    /// The range from `pos` to `end`.
    pub fn new(pos: usize, end: usize) -> Self {
        debug_assert!(pos <= end, "TextRange.end cannot be smaller than TextRange.pos");
        Self { pos, end }
    }

    /// Its length in bytes.
    pub fn len(&self) -> usize {
        self.end - self.pos
    }

    /// Whether it's empty.
    pub fn is_empty(&self) -> bool {
        self.pos == self.end
    }

    /// Its text, in `buffer`.
    pub fn text<'b>(&self, buffer: &'b str) -> &'b str {
        &buffer[self.pos..self.end]
    }

    /// The line and column of `index` in `buffer`, or line and column 0 when it's outside it. A
    /// `\r` counts for nothing, since TSDoc expects a `\n` after it.
    pub fn location(buffer: &str, index: isize) -> TextLocation {
        let Some(index) = usize::try_from(index).ok().filter(|&index| index <= buffer.len()) else {
            return TextLocation { line: 0, column: 0 };
        };
        Locator::default().locate(buffer, index)
    }
}

/// Finds the line and column of offsets into one buffer, going on from the last one it found when
/// the next is past it, so a parse's messages, which mostly come in order, scan the buffer once
/// rather than once each.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Locator {
    /// Where the characters it has counted end.
    at: usize,
    /// The location there.
    location: TextLocation,
}

impl Default for Locator {
    fn default() -> Self {
        Self { at: 0, location: TextLocation { line: 1, column: 1 } }
    }
}

impl Locator {
    /// The line and column of `index` in `buffer`, which has to be at most its length, as
    /// [`TextRange::location`] gives them.
    pub(crate) fn locate(&mut self, buffer: &str, index: usize) -> TextLocation {
        if index < self.at {
            *self = Self::default();
        }
        let start = self.at;
        let TextLocation { line, column } = &mut self.location;
        for (offset, c) in buffer[start..].char_indices().take_while(|&(offset, _)| start + offset < index) {
            match c {
                '\r' => {}
                '\n' => (*line, *column) = (*line + 1, 1),
                c => *column += c.len_utf16(),
            }
            self.at = start + offset + c.len_utf8();
        }
        self.location
    }
}
