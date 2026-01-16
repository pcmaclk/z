// Search System - 搜索系统
//
// 职责：提供异步文本搜索能力，
//       与编辑器状态解耦，通过 ViewModel 集成高亮结果

pub mod search;
pub mod result;

pub use search::SearchEngine;
