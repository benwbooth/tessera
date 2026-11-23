//! (tessh addition.) A hook that sees each editor event with the buffer it
//! will act on, and can put another event in its place: for edits that
//! depend on the text (auto-indentation, Enter that knows when a block is
//! done, Backspace over indentation).
use crate::ReedlineEvent;

/// What an [`EditHook`] is shown.
pub struct EditHookContext<'a> {
    /// The whole buffer.
    pub buffer: &'a str,
    /// The cursor, a byte offset into `buffer`.
    pub cursor: usize,
    /// Whether a menu (completions, history) is open and takes keys first.
    pub menu_active: bool,
}

/// See the module's docs.
pub trait EditHook: Send {
    /// The event to handle in place of `event`, or `None` to handle it as
    /// it is. (A replacement goes through the hook again, so return `None`
    /// for what the hook itself makes.)
    fn edit(&mut self, event: &ReedlineEvent, ctx: &EditHookContext<'_>) -> Option<ReedlineEvent>;
}
