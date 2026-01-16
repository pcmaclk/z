// Piece Table 缓冲区 - 文本缓冲区核心实现
//
// 职责：使用 Piece Table 数据结构管理文本内容，
//       支持大文件、内存映射、UTF-8安全、撤销重做

mod piece_table;
mod mode;
mod utf8;
mod mmap;
mod lines;
mod deletion_info;
mod chunk_iter;

// 重新导出
pub use self::piece_table::{PieceTable, Piece, PieceType, OriginalBuffer};
pub use self::mode::BufferMode;
pub use self::utf8::Utf8Validator;
pub use self::mmap::MmapBuffer;
pub use self::lines::{Lines, LineInfo};
pub use self::deletion_info::{DeletionInfo, DeletionPiece};
pub use self::chunk_iter::ChunkIter;

/// 文件大小阈值配置（根据冻结清单）
pub const SMALL_FILE_THRESHOLD: usize = 10 * 1024 * 1024; // 10MB
pub const LARGE_FILE_THRESHOLD: usize = 100 * 1024 * 1024; // 100MB

/// 性能相关常量
pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024; // 64KB，流式处理块大小
pub const LARGE_OPERATION_THRESHOLD: usize = 10 * 1024 * 1024; // 10MB，大型操作阈值
