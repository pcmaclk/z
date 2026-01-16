// IO System - IO系统
//
// 职责：提供统一、高效的文件访问接口，
//       支持内存映射、编码处理、大文件滑动窗口

pub mod file;
pub mod encoding;
pub mod mmap;

pub use file::FileIO;
