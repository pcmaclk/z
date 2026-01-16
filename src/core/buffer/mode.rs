// 缓冲区模式配置
//
// 职责：根据文件大小自适应缓冲区工作模式

use std::time::Duration;

/// 缓冲区工作模式（根据文件大小自适应）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferMode {
    /// 小文件模式（<10MB）：全内存
    InMemory {
        /// Piece数量合并阈值
        merge_threshold: usize,
        /// 是否在编辑时合并
        merge_on_edit: bool,
    },

    /// 大文件模式（10-100MB）：内存映射
    MemoryMapped {
        /// Piece数量合并阈值
        merge_threshold: usize,
        /// 是否在空闲时合并
        merge_on_idle: bool,
        /// 单次合并最大字节数
        max_merge_size: usize,
    },

    /// 超大文件模式（>100MB）：受限
    Restricted {
        /// Piece数量合并阈值
        merge_threshold: usize,
        /// 是否禁用合并
        disable_merge: bool,
    },
}

impl BufferMode {
    /// 根据文件大小选择模式（按冻结清单实现）
    pub fn for_file_size(file_size: usize) -> Self {
        if file_size < crate::core::buffer::SMALL_FILE_THRESHOLD {
            BufferMode::InMemory {
                merge_threshold: 1000,
                merge_on_edit: true,
            }
        } else if file_size < crate::core::buffer::LARGE_FILE_THRESHOLD {
            BufferMode::MemoryMapped {
                merge_threshold: 2000,
                merge_on_idle: true,
                max_merge_size: 1 * 1024 * 1024, // 1MB
            }
        } else {
            BufferMode::Restricted {
                merge_threshold: 5000,
                disable_merge: false,
            }
        }
    }

    /// 默认模式（空文件或新文件）
    pub fn default() -> Self {
        BufferMode::InMemory {
            merge_threshold: 1000,
            merge_on_edit: true,
        }
    }

    /// 获取合并阈值
    pub fn merge_threshold(&self) -> usize {
        match self {
            BufferMode::InMemory { merge_threshold, .. } => *merge_threshold,
            BufferMode::MemoryMapped { merge_threshold, .. } => *merge_threshold,
            BufferMode::Restricted { merge_threshold, .. } => *merge_threshold,
        }
    }

    /// 是否应该自动合并
    pub fn should_auto_merge(&self) -> bool {
        match self {
            BufferMode::InMemory { merge_on_edit, .. } => *merge_on_edit,
            BufferMode::MemoryMapped { merge_on_idle, .. } => *merge_on_idle,
            BufferMode::Restricted { disable_merge, .. } => !disable_merge,
        }
    }

    /// 是否是大文件模式
    pub fn is_large_file(&self) -> bool {
        matches!(self, BufferMode::MemoryMapped { .. } | BufferMode::Restricted { .. })
    }
}

impl Default for BufferMode {
    fn default() -> Self {
        Self::default()
    }
}
