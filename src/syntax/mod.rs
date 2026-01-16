// Syntax Highlight System - 语法高亮系统
//
// 职责：提供语法高亮分析能力，
//       采用自定义 lexer（正则+状态机）

pub mod lexer;
pub mod highlight;
pub mod token;

pub use lexer::Lexer;
