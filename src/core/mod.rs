// Editor Core - 编辑器核心
//
// 职责：管理 Piece Table 缓冲区、光标、选区状态，
//       接收 EditorAction，通过事务模型更新状态

pub mod buffer;

pub use buffer::{PieceTable, Piece, PieceType, OriginalBuffer, BufferMode};
