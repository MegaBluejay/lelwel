pub type Span = core::ops::Range<usize>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NodeRef(pub usize);

impl NodeRef {
    #[allow(dead_code)]
    pub const ROOT: NodeRef = NodeRef(0);
}

#[cfg(target_pointer_width = "64")]
#[derive(Copy, Clone)]
pub struct CstIndex([u8; 6]);

#[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
#[derive(Copy, Clone)]
pub struct CstIndex(usize);

impl core::fmt::Debug for CstIndex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        usize::from(*self).fmt(f)
    }
}

impl From<CstIndex> for usize {
    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn from(value: CstIndex) -> Self {
        let [b0, b1, b2, b3, b4, b5] = value.0;
        usize::from_le_bytes([b0, b1, b2, b3, b4, b5, 0, 0])
    }
    #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
    #[inline]
    fn from(value: CstIndex) -> Self {
        value.0
    }
}

impl From<usize> for CstIndex {
    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn from(value: usize) -> Self {
        let [b0, b1, b2, b3, b4, b5, b6, b7] = value.to_le_bytes();
        debug_assert!(b6 == 0 && b7 == 0);
        Self([b0, b1, b2, b3, b4, b5])
    }
    #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
    #[inline]
    #[allow(unused_comparisons)]
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy)]
pub struct MarkOpened(pub(crate) usize);
#[derive(Clone, Copy)]
pub struct MarkClosed(pub usize);
#[derive(Clone)]
pub struct MarkTruncation {
    pub(crate) node_count: usize,
    pub(crate) token_count: usize,
    pub(crate) non_skip_len: usize,
}
