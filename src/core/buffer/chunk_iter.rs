// 流式迭代器
//
// 职责：按块迭代 PieceTable 内容，避免一次性加载全部文本

use std::ops::Range;
use crate::core::buffer::{PieceTable, DEFAULT_CHUNK_SIZE};

/// PieceTable的流式迭代器
pub struct ChunkIter<'a> {
    piece_table: &'a PieceTable,
    current_pos: usize,
    chunk_size: usize,
    total_bytes: usize,
}

impl<'a> ChunkIter<'a> {
    pub fn new(piece_table: &'a PieceTable, chunk_size: usize) -> Self {
        Self {
            piece_table,
            current_pos: 0,
            chunk_size,
            total_bytes: piece_table.total_bytes(),
        }
    }

    pub fn with_default_chunk_size(piece_table: &'a PieceTable) -> Self {
        Self::new(piece_table, DEFAULT_CHUNK_SIZE)
    }
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_pos >= self.total_bytes {
            return None;
        }

        let end = (self.current_pos + self.chunk_size).min(self.total_bytes);
        let chunk = self.piece_table.get_text_range(self.current_pos..end);
        self.current_pos = end;

        Some(chunk)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total_bytes.saturating_sub(self.current_pos);
        let chunks = (remaining + self.chunk_size - 1) / self.chunk_size;
        (chunks, Some(chunks))
    }
}
