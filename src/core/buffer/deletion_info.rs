// 延迟删除信息
//
// 职责：支持延迟加载删除的文本内容，优化大文件性能

use std::ops::Range;
use crate::core::buffer::PieceType;

/// 被删除的Piece信息
#[derive(Debug, Clone)]
pub struct DeletionPiece {
    pub piece_type: PieceType,
    pub range: Range<usize>,
}

/// 删除操作的信息（支持延迟加载）
#[derive(Debug, Clone)]
pub struct DeletionInfo {
    /// 删除的字节范围
    pub byte_range: Range<usize>,
    /// 被删除的Piece信息
    pub pieces: Vec<DeletionPiece>,
    /// 缓存的删除文本（延迟加载）
    cached_text: Option<String>,
}

impl DeletionInfo {
    pub fn new(byte_range: Range<usize>, pieces: Vec<DeletionPiece>) -> Self {
        Self {
            byte_range,
            pieces,
            cached_text: None,
        }
    }

    /// 获取删除的文本（延迟加载）
    pub fn get_text<F>(&mut self, loader: F) -> String
    where
        F: FnOnce(&[DeletionPiece]) -> String,
    {
        if self.cached_text.is_none() {
            self.cached_text = Some(loader(&self.pieces));
        }

        self.cached_text.clone().unwrap_or_default()
    }

    /// 清空缓存的文本（节省内存）
    pub fn clear_cache(&mut self) {
        self.cached_text = None;
    }

    /// 获取删除的长度
    pub fn len(&self) -> usize {
        self.byte_range.len()
    }

    /// 是否为空删除
    pub fn is_empty(&self) -> bool {
        self.byte_range.is_empty()
    }
}
