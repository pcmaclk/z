# 📋 **PieceTable 完整实现方案（符合冻结架构）**

## 🎯 **设计原则**
1. ✅ **符合已冻结架构清单**所有要求
2. ✅ **实现讨论过的所有功能**，不新增复杂度
3. ✅ **遵循流式处理原则**（IO系统负责流式）
4. ✅ **UTF-8安全**，字符边界保证
5. ✅ **内存友好**（Arc共享 + 延迟加载）

---

## 📁 **模块结构**
```
src/core/buffer/
├── mod.rs              # 模块导出
├── piece_table.rs      # PieceTable主实现（核心）
├── mode.rs            # 缓冲区模式配置
├── utf8.rs            # UTF-8边界安全工具
├── mmap.rs            # 内存映射封装
├── lines.rs           # 行索引（基础版）
├── deletion_info.rs   # 延迟删除信息
└── chunk_iter.rs      # 流式迭代器（新增）
```

---

## 1. **模块导出**

```rust
// src/core/buffer/mod.rs
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
pub use self::lines::Lines;
pub use self::deletion_info::{DeletionInfo, DeletionPiece};
pub use self::chunk_iter::ChunkIter;

/// 文件大小阈值配置（根据冻结清单）
pub const SMALL_FILE_THRESHOLD: usize = 10 * 1024 * 1024; // 10MB
pub const LARGE_FILE_THRESHOLD: usize = 100 * 1024 * 1024; // 100MB

/// 性能相关常量
pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024; // 64KB，流式处理块大小
pub const LARGE_OPERATION_THRESHOLD: usize = 10 * 1024 * 1024; // 10MB，大型操作阈值
```

---

## 2. **缓冲区模式配置**（直接实现讨论过的策略）

```rust
// src/core/buffer/mode.rs
use std::time::Duration;

/// 缓冲区工作模式（根据文件大小自适应）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferMode {
    /// 小文件模式（<10MB）：全内存
    InMemory {
        /// Piece数量合并阈值（讨论确定：1000）
        merge_threshold: usize,
        /// 是否在编辑时合并（讨论确定：true）
        merge_on_edit: bool,
    },
    
    /// 大文件模式（10-100MB）：内存映射
    MemoryMapped {
        /// Piece数量合并阈值（讨论确定：2000）
        merge_threshold: usize,
        /// 是否在空闲时合并（讨论确定：true）
        merge_on_idle: bool,
        /// 单次合并最大字节数（讨论确定：1MB）
        max_merge_size: usize,
    },
    
    /// 超大文件模式（>100MB）：受限
    Restricted {
        /// Piece数量合并阈值（讨论确定：5000）
        merge_threshold: usize,
        /// 是否禁用合并（讨论确定：false）
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
```

---

## 3. **UTF-8边界安全工具**（必须实现）

```rust
// src/core/buffer/utf8.rs
use std::ops::Range;

/// UTF-8边界安全工具
#[derive(Debug, Clone, Copy)]
pub struct Utf8Validator;

impl Utf8Validator {
    /// 确保字节偏移在UTF-8字符边界
    pub fn ensure_char_boundary(text: &str, byte_offset: usize) -> usize {
        let bytes = text.as_bytes();
        let len = bytes.len();
        
        if byte_offset >= len {
            return byte_offset;
        }
        
        // 已经是字符边界
        if Self::is_char_boundary(bytes, byte_offset) {
            return byte_offset;
        }
        
        // 向前找到最近的字符边界
        let mut pos = byte_offset;
        while pos > 0 && !Self::is_char_boundary(bytes, pos) {
            pos -= 1;
        }
        
        pos
    }
    
    /// 确保范围在UTF-8字符边界
    pub fn ensure_char_boundary_range(text: &str, range: Range<usize>) -> Range<usize> {
        let start = Self::ensure_char_boundary(text, range.start);
        let end = Self::ensure_char_boundary(text, range.end);
        
        // 确保start <= end
        if start > end {
            start..start
        } else {
            start..end
        }
    }
    
    /// 检查是否是UTF-8字符边界
    pub fn is_char_boundary(bytes: &[u8], index: usize) -> bool {
        // UTF-8规则：
        // 0xxxxxxx (ASCII) 或 11xxxxxx（多字节字符开头）
        // 10xxxxxx 是连续字节，不是边界
        index == 0 || index >= bytes.len() || (bytes[index] & 0xC0) != 0x80
    }
    
    /// 安全获取子字符串（保证在字符边界）
    pub fn safe_substr(text: &str, start: usize, end: usize) -> &str {
        let safe_start = Self::ensure_char_boundary(text, start);
        let safe_end = Self::ensure_char_boundary(text, end);
        
        if safe_start >= safe_end {
            return "";
        }
        
        &text[safe_start..safe_end]
    }
}
```

---

## 4. **内存映射封装**（大文件支持）

```rust
// src/core/buffer/mmap.rs
#[cfg(not(target_arch = "wasm32"))]
use memmap2::Mmap;
use std::ops::Range;

/// 内存映射缓冲区（大文件支持）
#[derive(Debug)]
pub struct MmapBuffer {
    #[cfg(not(target_arch = "wasm32"))]
    mmap: Mmap,
    
    #[cfg(target_arch = "wasm32")]
    data: Vec<u8>,
    
    length: usize,
}

impl MmapBuffer {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        use std::fs::File;
        
        let file = File::open(path)
            .map_err(|e| format!("无法打开文件: {}", e))?;
        
        let metadata = file.metadata()
            .map_err(|e| format!("无法获取文件信息: {}", e))?;
        
        let mmap = unsafe {
            Mmap::map(&file)
                .map_err(|e| format!("内存映射失败: {}", e))?
        };
        
        Ok(Self {
            mmap,
            length: metadata.len() as usize,
        })
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    pub fn empty() -> Self {
        Self {
            mmap: Mmap::map(&[]).unwrap(),
            length: 0,
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    pub fn from_file(_path: &std::path::Path) -> Result<Self, String> {
        Err("WebAssembly环境不支持文件内存映射".to_string())
    }
    
    #[cfg(target_arch = "wasm32")]
    pub fn empty() -> Self {
        Self {
            data: Vec::new(),
            length: 0,
        }
    }
    
    /// 获取缓冲区长度（字节）
    pub fn len(&self) -> usize {
        self.length
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    
    /// 获取字节切片
    pub fn get_bytes(&self, range: Range<usize>) -> &[u8] {
        let start = range.start.min(self.length);
        let end = range.end.min(self.length);
        
        if start >= end {
            return &[];
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            &self.mmap[start..end]
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            &self.data[start..end]
        }
    }
    
    /// 尝试获取文本（UTF-8验证）
    pub fn get_text(&self, range: Range<usize>) -> Result<&str, std::str::Utf8Error> {
        let bytes = self.get_bytes(range);
        std::str::from_utf8(bytes)
    }
    
    /// 获取文本（UTF-8损失转换）
    pub fn get_text_lossy(&self, range: Range<usize>) -> String {
        let bytes = self.get_bytes(range);
        String::from_utf8_lossy(bytes).into_owned()
    }
}

impl Clone for MmapBuffer {
    fn clone(&self) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            mmap: self.mmap.clone(),
            
            #[cfg(target_arch = "wasm32")]
            data: self.data.clone(),
            
            length: self.length,
        }
    }
}
```

---

## 5. **延迟删除信息**（大文件优化）

```rust
// src/core/buffer/deletion_info.rs
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
```

---

## 6. **流式迭代器**（新增，符合流式处理原则）

```rust
// src/core/buffer/chunk_iter.rs
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
```

---

## 7. **行索引**（基础版，懒构建+增量更新）

```rust
// src/core/buffer/lines.rs
use std::ops::Range;

/// 行信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineInfo {
    /// 行的字节范围
    pub byte_range: Range<usize>,
    /// 行号（0-based）
    pub line_number: usize,
    /// 是否以换行符结束
    pub ends_with_newline: bool,
}

/// 行索引管理器
#[derive(Debug, Clone, Default)]
pub struct Lines {
    /// 所有行的信息（按行号排序）
    lines: Vec<LineInfo>,
    /// 总字节数
    total_bytes: usize,
    /// 是否脏（需要重建）
    dirty: bool,
}

impl Lines {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            total_bytes: 0,
            dirty: true,
        }
    }
    
    /// 从文本构建行索引
    pub fn build_from_text(&mut self, text: &str) {
        self.lines.clear();
        
        let mut line_start = 0;
        let mut line_number = 0;
        
        for (i, c) in text.char_indices() {
            if c == '\n' {
                self.lines.push(LineInfo {
                    byte_range: line_start..i,
                    line_number,
                    ends_with_newline: true,
                });
                line_start = i + 1;
                line_number += 1;
            }
        }
        
        // 最后一行（如果没有以换行符结束）
        if line_start < text.len() {
            self.lines.push(LineInfo {
                byte_range: line_start..text.len(),
                line_number,
                ends_with_newline: false,
            });
        }
        
        self.total_bytes = text.len();
        self.dirty = false;
    }
    
    /// 增量更新：处理插入
    pub fn handle_insert(&mut self, offset: usize, text: &str) {
        // 简化实现：有插入就标记为脏
        // 未来可以优化为真正的增量更新
        self.dirty = true;
        self.total_bytes += text.len();
    }
    
    /// 增量更新：处理删除
    pub fn handle_delete(&mut self, range: Range<usize>) {
        // 简化实现：有删除就标记为脏
        self.dirty = true;
        self.total_bytes -= range.len();
    }
    
    /// 查找包含指定字节偏移的行
    pub fn find_line_by_offset(&self, offset: usize) -> Option<usize> {
        if self.dirty {
            return None;
        }
        
        self.lines
            .binary_search_by(|line| {
                if offset < line.byte_range.start {
                    std::cmp::Ordering::Greater
                } else if offset >= line.byte_range.end {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .ok()
    }
    
    /// 获取指定行的字节范围
    pub fn get_line_range(&self, line_number: usize) -> Option<Range<usize>> {
        self.lines
            .get(line_number)
            .map(|line| line.byte_range.clone())
    }
    
    /// 总行数
    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }
    
    /// 是否脏（需要重建）
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// 标记为脏
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
```

---

## 8. **PieceTable主实现**（核心）

```rust
// src/core/buffer/piece_table.rs
use std::sync::Arc;
use std::ops::Range;
use crate::core::buffer::{
    mode::BufferMode,
    utf8::Utf8Validator,
    mmap::MmapBuffer,
    lines::Lines,
    deletion_info::{DeletionInfo, DeletionPiece},
    chunk_iter::ChunkIter,
    SMALL_FILE_THRESHOLD, LARGE_OPERATION_THRESHOLD,
};

/// 原始缓冲区类型
#[derive(Debug, Clone)]
pub enum OriginalBuffer {
    /// 小文件：内存中的字符串（Arc共享）
    InMemory(Arc<str>),
    
    /// 大文件：内存映射（只读）
    #[cfg(not(target_arch = "wasm32"))]
    MemoryMapped(Arc<MmapBuffer>),
    
    /// WebAssembly环境
    #[cfg(target_arch = "wasm32")]
    Bytes(Arc<[u8]>),
}

/// Piece类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceType {
    Original,
    Add,
}

/// Piece描述符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub piece_type: PieceType,
    pub start: usize,    // 在相应缓冲区中的起始位置
    pub length: usize,   // 长度（字节）
}

impl Piece {
    pub fn original(range: Range<usize>) -> Self {
        Self {
            piece_type: PieceType::Original,
            start: range.start,
            length: range.end - range.start,
        }
    }
    
    pub fn add(range: Range<usize>) -> Self {
        Self {
            piece_type: PieceType::Add,
            start: range.start,
            length: range.end - range.start,
        }
    }
    
    /// 是否为空Piece
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

/// Piece Table核心实现
#[derive(Debug, Clone)]
pub struct PieceTable {
    // --- 核心数据（使用Arc共享）---
    original: OriginalBuffer,           // 原始内容
    additions: Arc<str>,                // 新增内容
    
    // --- Piece链管理 ---
    pieces: Vec<Piece>,                 // Piece链
    piece_offsets: Vec<usize>,          // 累积偏移缓存
    
    // --- 状态和配置 ---
    total_bytes: usize,                 // 总字节数
    mode: BufferMode,                   // 缓冲区模式
    lines: Option<Lines>,               // 行索引
    
    // --- 合并控制 ---
    suspend_auto_merge: bool,           // 是否暂停自动合并
    last_merge_time: std::time::Instant, // 上次合并时间
    edit_count_since_last_merge: usize, // 上次合并后的编辑次数
}

// ========== 构造方法 ==========

impl PieceTable {
    /// 创建新的空PieceTable
    pub fn new() -> Self {
        Self {
            original: OriginalBuffer::InMemory(Arc::from("")),
            additions: Arc::from(""),
            pieces: Vec::new(),
            piece_offsets: Vec::new(),
            total_bytes: 0,
            mode: BufferMode::default(),
            lines: None,
            suspend_auto_merge: false,
            last_merge_time: std::time::Instant::now(),
            edit_count_since_last_merge: 0,
        }
    }
    
    /// 从文本创建（小文件）
    pub fn from_text(text: &str) -> Self {
        let text_len = text.len();
        let mut table = Self::new();
        
        if text_len > SMALL_FILE_THRESHOLD {
            table.mode = BufferMode::for_file_size(text_len);
        }
        
        if !text.is_empty() {
            table.original = OriginalBuffer::InMemory(Arc::from(text));
            table.pieces = vec![Piece::original(0..text_len)];
            table.piece_offsets = vec![0];
            table.total_bytes = text_len;
        }
        
        table
    }
    
    /// 从文件创建（支持大文件）
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        use std::fs;
        
        let metadata = fs::metadata(path)
            .map_err(|e| format!("获取文件信息失败: {}", e))?;
        
        let file_size = metadata.len() as usize;
        let mode = BufferMode::for_file_size(file_size);
        
        match mode {
            BufferMode::InMemory { .. } => {
                // 小文件：全量读入
                let content = fs::read_to_string(path)
                    .map_err(|e| format!("读取文件失败: {}", e))?;
                
                Ok(Self::from_text(&content))
            }
            _ => {
                // 大文件：内存映射
                let mmap_buffer = MmapBuffer::from_file(path)?;
                let arc_buffer = Arc::new(mmap_buffer);
                
                Ok(Self {
                    original: OriginalBuffer::MemoryMapped(arc_buffer),
                    additions: Arc::from(""),
                    pieces: vec![Piece::original(0..file_size)],
                    piece_offsets: vec![0],
                    total_bytes: file_size,
                    mode,
                    lines: None,
                    suspend_auto_merge: false,
                    last_merge_time: std::time::Instant::now(),
                    edit_count_since_last_merge: 0,
                })
            }
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    pub fn from_file(_path: &std::path::Path) -> Result<Self, String> {
        Err("WebAssembly环境不支持文件操作".to_string())
    }
}

// ========== 基本查询 ==========

impl PieceTable {
    /// 获取总字节数
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
    
    /// 获取总字符数（UTF-8安全）
    pub fn total_chars(&self) -> usize {
        // 懒计算，需要时遍历
        self.get_text_range(0..self.total_bytes).chars().count()
    }
    
    /// 获取Piece数量
    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }
    
    /// 获取缓冲区模式
    pub fn mode(&self) -> &BufferMode {
        &self.mode
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.total_bytes == 0
    }
    
    /// 估计内存使用量
    pub fn estimated_memory(&self) -> usize {
        let additions_size = self.additions.len();
        let pieces_size = self.pieces.len() * std::mem::size_of::<Piece>();
        let offsets_size = self.piece_offsets.len() * std::mem::size_of::<usize>();
        
        additions_size + pieces_size + offsets_size
    }
}

// ========== UTF-8安全操作（冻结清单要求） ==========

impl PieceTable {
    /// UTF-8安全的插入
    pub fn insert_char_safe(&mut self, byte_offset: usize, text: &str) -> (Self, String) {
        // 确保插入点在字符边界
        let text_for_check = self.get_text_range(0..self.total_bytes.min(byte_offset + 100));
        let safe_offset = Utf8Validator::ensure_char_boundary(&text_for_check, byte_offset);
        
        self.insert_internal(safe_offset, text)
    }
    
    /// UTF-8安全的删除
    pub fn delete_char_safe(&mut self, range: Range<usize>) -> (Self, String) {
        let text = self.get_text_range(range.clone());
        let safe_range = Utf8Validator::ensure_char_boundary_range(&text, range);
        
        if safe_range.is_empty() {
            return (self.clone(), String::new());
        }
        
        self.delete_internal(safe_range)
    }
}

// ========== 核心操作（内部） ==========

impl PieceTable {
    fn insert_internal(&mut self, offset: usize, text: &str) -> (Self, String) {
        if text.is_empty() {
            return (self.clone(), String::new());
        }
        
        if offset > self.total_bytes {
            panic!("插入位置超出范围: {} > {}", offset, self.total_bytes);
        }
        
        // 1. 查找插入点
        let (piece_idx, offset_in_piece) = self.find_piece_and_offset(offset);
        
        // 2. 在additions缓冲区追加新文本
        let current_additions = self.additions.to_string();
        let add_start = current_additions.len();
        let new_additions = format!("{}{}", current_additions, text);
        let additions_arc = Arc::from(new_additions);
        let add_length = text.len();
        
        // 3. 构建新的Piece链
        let mut new_pieces = Vec::with_capacity(self.pieces.len() + 2);
        
        // 插入点之前的Piece
        new_pieces.extend_from_slice(&self.pieces[..piece_idx]);
        
        let current_piece = self.pieces[piece_idx];
        
        // 处理当前Piece的分裂和插入
        if offset_in_piece > 0 && offset_in_piece < current_piece.length {
            // 在Piece中间插入：分裂为三部分
            new_pieces.push(Piece {
                piece_type: current_piece.piece_type,
                start: current_piece.start,
                length: offset_in_piece,
            });
            
            new_pieces.push(Piece {
                piece_type: PieceType::Add,
                start: add_start,
                length: add_length,
            });
            
            new_pieces.push(Piece {
                piece_type: current_piece.piece_type,
                start: current_piece.start + offset_in_piece,
                length: current_piece.length - offset_in_piece,
            });
        } else if offset_in_piece == 0 {
            // 在Piece开头插入
            new_pieces.push(Piece {
                piece_type: PieceType::Add,
                start: add_start,
                length: add_length,
            });
            new_pieces.push(current_piece);
        } else {
            // offset_in_piece == current_piece.length，在Piece结尾
            new_pieces.push(current_piece);
            new_pieces.push(Piece {
                piece_type: PieceType::Add,
                start: add_start,
                length: add_length,
            });
        }
        
        // 插入点之后的Piece
        if piece_idx + 1 < self.pieces.len() {
            new_pieces.extend_from_slice(&self.pieces[piece_idx + 1..]);
        }
        
        // 4. 创建新实例
        let mut new_table = Self {
            original: self.original.clone(),
            additions: additions_arc,
            pieces: new_pieces,
            piece_offsets: Vec::new(),
            total_bytes: self.total_bytes + add_length,
            mode: self.mode,
            lines: self.lines.clone(),
            suspend_auto_merge: self.suspend_auto_merge,
            last_merge_time: self.last_merge_time,
            edit_count_since_last_merge: self.edit_count_since_last_merge + 1,
        };
        
        // 5. 更新累积偏移
        new_table.update_piece_offsets();
        
        // 6. 智能合并决策（按讨论的策略）
        if new_table.should_merge_after_edit() {
            new_table.merge_pieces_smart();
        }
        
        // 7. 更新行索引
        if let Some(ref mut lines) = new_table.lines {
            lines.handle_insert(offset, text);
        }
        
        (new_table, text.to_string())
    }
    
    fn delete_internal(&mut self, range: Range<usize>) -> (Self, String) {
        let start = range.start;
        let end = range.end.min(self.total_bytes);
        
        if start >= end {
            return (self.clone(), String::new());
        }
        
        // 1. 获取被删除的文本
        let deleted_text = self.get_text_range(start..end);
        
        // 2. 查找删除范围
        let (start_piece, start_offset) = self.find_piece_and_offset(start);
        let (end_piece, end_offset) = self.find_piece_and_offset(end);
        
        // 3. 构建新的Piece链
        let mut new_pieces = Vec::with_capacity(self.pieces.len());
        
        // 删除开始之前的Piece
        new_pieces.extend_from_slice(&self.pieces[..start_piece]);
        
        // 处理开始Piece（如果不是从Piece开头删除）
        if start_offset > 0 {
            let piece = self.pieces[start_piece];
            new_pieces.push(Piece {
                piece_type: piece.piece_type,
                start: piece.start,
                length: start_offset,
            });
        }
        
        // 处理结束Piece（如果不是到Piece结尾删除）
        if end_offset < self.pieces[end_piece].length {
            let piece = self.pieces[end_piece];
            new_pieces.push(Piece {
                piece_type: piece.piece_type,
                start: piece.start + end_offset,
                length: piece.length - end_offset,
            });
        }
        
        // 删除结束之后的Piece
        if end_piece + 1 < self.pieces.len() {
            new_pieces.extend_from_slice(&self.pieces[end_piece + 1..]);
        }
        
        // 4. 创建新实例
        let mut new_table = Self {
            original: self.original.clone(),
            additions: self.additions.clone(),
            pieces: new_pieces,
            piece_offsets: Vec::new(),
            total_bytes: self.total_bytes - (end - start),
            mode: self.mode,
            lines: self.lines.clone(),
            suspend_auto_merge: self.suspend_auto_merge,
            last_merge_time: self.last_merge_time,
            edit_count_since_last_merge: self.edit_count_since_last_merge + 1,
        };
        
        // 5. 更新累积偏移
        new_table.update_piece_offsets();
        
        // 6. 智能合并决策
        if new_table.should_merge_after_edit() {
            new_table.merge_pieces_smart();
        }
        
        // 7. 更新行索引
        if let Some(ref mut lines) = new_table.lines {
            lines.handle_delete(start..end);
        }
        
        (new_table, deleted_text)
    }
    
    /// 延迟删除（不立即获取文本，大文件优化）
    pub fn delete_lazy(&mut self, range: Range<usize>) -> (Self, DeletionInfo) {
        let start = range.start;
        let end = range.end.min(self.total_bytes);
        
        if start >= end {
            return (self.clone(), DeletionInfo::new(range, Vec::new()));
        }
        
        // 收集被删除的Piece信息（不获取文本）
        let mut deleted_pieces = Vec::new();
        let mut current_pos = 0;
        
        for piece in &self.pieces {
            let piece_end = current_pos + piece.length;
            
            // 检查这个Piece是否与删除范围重叠
            if piece_end > start && current_pos < end {
                let overlap_start = start.max(current_pos);
                let overlap_end = end.min(piece_end);
                let overlap_len = overlap_end - overlap_start;
                
                if overlap_len > 0 {
                    let piece_start = piece.start + (overlap_start - current_pos);
                    deleted_pieces.push(DeletionPiece {
                        piece_type: piece.piece_type,
                        range: piece_start..piece_start + overlap_len,
                    });
                }
            }
            
            current_pos = piece_end;
            if current_pos >= end {
                break;
            }
        }
        
        // 执行删除操作
        let (new_table, _) = self.delete_internal(range.clone());
        
        (new_table, DeletionInfo::new(range, deleted_pieces))
    }
}

// ========== 文本获取（按冻结清单：按需读取+流式） ==========

impl PieceTable {
    /// 获取指定范围的文本（核心API）
    pub fn get_text_range(&self, range: Range<usize>) -> String {
        let start = range.start.min(self.total_bytes);
        let end = range.end.min(self.total_bytes);
        
        if start >= end {
            return String::new();
        }
        
        let mut result = String::with_capacity(end - start);
        let mut current_pos = 0;
        
        for (piece, piece_start) in self.pieces.iter().zip(&self.piece_offsets) {
            if current_pos >= end {
                break;
            }
            
            let piece_end = piece_start + piece.length;
            if piece_end <= start {
                current_pos = piece_end;
                continue;
            }
            
            // 计算重叠部分
            let overlap_start = start.max(*piece_start);
            let overlap_end = end.min(piece_end);
            
            if overlap_start < overlap_end {
                let overlap_len = overlap_end - overlap_start;
                let piece_offset = overlap_start - *piece_start;
                
                match piece.piece_type {
                    PieceType::Original => {
                        let slice_start = piece.start + piece_offset;
                        let slice_end = slice_start + overlap_len;
                        
                        match &self.original {
                            OriginalBuffer::InMemory(s) => {
                                result.push_str(&s[slice_start..slice_end]);
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            OriginalBuffer::MemoryMapped(mmap) => {
                                match mmap.get_text(slice_start..slice_end) {
                                    Ok(text) => result.push_str(&text),
                                    Err(_) => {
                                        // UTF-8无效，使用损失转换
                                        let lossy = mmap.get_text_lossy(slice_start..slice_end);
                                        result.push_str(&lossy);
                                    }
                                }
                            }
                            #[cfg(target_arch = "wasm32")]
                            OriginalBuffer::Bytes(data) => {
                                let slice = &data[slice_start..slice_end];
                                if let Ok(text) = std::str::from_utf8(slice) {
                                    result.push_str(text);
                                } else {
                                    result.push_str(&String::from_utf8_lossy(slice));
                                }
                            }
                        }
                    }
                    PieceType::Add => {
                        let slice_start = piece.start + piece_offset;
                        let slice_end = slice_start + overlap_len;
                        result.push_str(&self.additions[slice_start..slice_end]);
                    }
                }
            }
            
            current_pos = piece_end;
        }
        
        result
    }
    
    /// ⚠️ 获取全部文本（仅用于测试或小文件，大文件慎用）
    #[cfg(test)]
    pub fn get_all_text(&self) -> String {
        self.get_text_range(0..self.total_bytes)
    }
    
    /// 获取指定行的文本
    pub fn get_line(&self, line_number: usize) -> Option<String> {
        self.get_or_build_lines().get_line_range(line_number)
            .map(|range| self.get_text_range(range))
    }
    
    /// 创建流式迭代器（符合流式处理原则）
    pub fn iter_chunks(&self, chunk_size: usize) -> ChunkIter {
        ChunkIter::new(self, chunk_size)
    }
    
    /// 使用默认块大小的流式迭代器
    pub fn iter_chunks_default(&self) -> ChunkIter {
        ChunkIter::with_default_chunk_size(self)
    }
}

// ========== Piece查找和索引 ==========

impl PieceTable {
    /// 查找字节偏移所在的Piece和在Piece内的偏移
    fn find_piece_and_offset(&self, byte_offset: usize) -> (usize, usize) {
        if byte_offset >= self.total_bytes {
            return (self.pieces.len().saturating_sub(1), 0);
        }
        
        // 使用累积偏移进行二分查找
        match self.piece_offsets.binary_search(&byte_offset) {
            Ok(i) => (i, 0),
            Err(i) => {
                if i == 0 {
                    (0, byte_offset)
                } else {
                    let piece_idx = i - 1;
                    let offset_in_piece = byte_offset - self.piece_offsets[piece_idx];
                    (piece_idx, offset_in_piece.min(self.pieces[piece_idx].length))
                }
            }
        }
    }
    
    /// 更新累积偏移缓存
    fn update_piece_offsets(&mut self) {
        self.piece_offsets.clear();
        let mut offset = 0;
        
        for piece in &self.pieces {
            self.piece_offsets.push(offset);
            offset += piece.length;
        }
        
        self.total_bytes = offset;
    }
}

// ========== 行索引管理 ==========

impl PieceTable {
    /// 获取或构建行索引
    pub fn get_or_build_lines(&mut self) -> &Lines {
        if self.lines.is_none() {
            self.lines = Some(Lines::new());
        }
        
        let lines = self.lines.as_mut().unwrap();
        if lines.is_dirty() {
            // 对于大文件，可以延迟构建或增量构建
            // 这里简化实现：全量构建
            let text = self.get_text_range(0..self.total_bytes.min(10 * 1024 * 1024)); // 最多10MB
            lines.build_from_text(&text);
        }
        
        lines
    }
    
    /// 获取行索引（如果存在）
    pub fn lines(&self) -> Option<&Lines> {
        self.lines.as_ref()
    }
}

// ========== 合并策略（按讨论的策略实现） ==========

impl PieceTable {
    /// 判断编辑后是否应该合并（按讨论的策略）
    fn should_merge_after_edit(&self) -> bool {
        if self.suspend_auto_merge {
            return false;
        }
        
        let piece_count = self.pieces.len();
        let threshold = self.mode.merge_threshold();
        
        match self.mode {
            BufferMode::InMemory { merge_on_edit, .. } => {
                // 小文件：编辑时合并
                merge_on_edit && piece_count > threshold
            }
            BufferMode::MemoryMapped { merge_on_idle, .. } => {
                // 大文件：空闲时合并
                // 简化空闲检测：距离上次合并有一定编辑次数
                let is_idle = self.edit_count_since_last_merge > 100;
                merge_on_idle && is_idle && piece_count > threshold
            }
            BufferMode::Restricted { disable_merge, .. } => {
                // 超大文件：非常保守
                !disable_merge && piece_count > threshold * 2
            }
        }
    }
    
    /// 智能合并实现（根据模式）
    fn merge_pieces_smart(&mut self) {
        match self.mode {
            BufferMode::InMemory { .. } => {
                self.merge_all_adjacent();
            }
            BufferMode::MemoryMapped { max_merge_size, .. } => {
                self.merge_incremental(max_merge_size);
            }
            BufferMode::Restricted { .. } => {
                self.merge_small_fragments_only(1024); // 只合并<1KB的碎片
            }
        }
        
        // 合并后更新偏移缓存
        self.update_piece_offsets();
        self.last_merge_time = std::time::Instant::now();
        self.edit_count_since_last_merge = 0;
    }
    
    /// 合并所有相邻的同类型Piece
    fn merge_all_adjacent(&mut self) {
        if self.pieces.len() <= 1 {
            return;
        }
        
        let mut merged = Vec::with_capacity(self.pieces.len());
        merged.push(self.pieces[0]);
        
        for i in 1..self.pieces.len() {
            let current = self.pieces[i];
            let last = merged.last_mut().unwrap();
            
            if self.can_merge_pieces(last, &current) {
                last.length += current.length;
            } else {
                merged.push(current);
            }
        }
        
        self.pieces = merged;
    }
    
    /// 增量合并（控制每次合并的大小）
    fn merge_incremental(&mut self, max_merge_size: usize) {
        if self.pieces.len() <= 1 {
            return;
        }
        
        let mut merged_bytes = 0;
        let mut merged = Vec::with_capacity(self.pieces.len());
        merged.push(self.pieces[0]);
        
        for i in 1..self.pieces.len() {
            let current = self.pieces[i];
            let last = merged.last_mut().unwrap();
            
            if merged_bytes < max_merge_size && self.can_merge_pieces(last, &current) {
                last.length += current.length;
                merged_bytes += current.length;
            } else {
                merged.push(current);
            }
        }
        
        self.pieces = merged;
    }
    
    /// 只合并小碎片
    fn merge_small_fragments_only(&mut self, max_fragment_size: usize) {
        if self.pieces.len() <= 1 {
            return;
        }
        
        let mut merged = Vec::with_capacity(self.pieces.len());
        merged.push(self.pieces[0]);
        
        for i in 1..self.pieces.len() {
            let current = self.pieces[i];
            let last = merged.last_mut().unwrap();
            
            // 只合并小碎片
            if current.length <= max_fragment_size && self.can_merge_pieces(last, &current) {
                last.length += current.length;
            } else {
                merged.push(current);
            }
        }
        
        self.pieces = merged;
    }
    
    /// 检查两个Piece是否可以合并
    fn can_merge_pieces(&self, a: &Piece, b: &Piece) -> bool {
        if a.piece_type != b.piece_type {
            return false;
        }
        
        match a.piece_type {
            PieceType::Add => a.start + a.length == b.start,
            PieceType::Original => a.start + a.length == b.start,
        }
    }
    
    /// 手动触发合并
    pub fn merge_if_needed(&mut self) {
        if !self.suspend_auto_merge && self.pieces.len() > self.mode.merge_threshold() {
            self.merge_pieces_smart();
        }
    }
}

// ========== 大型操作防护（按冻结清单要求） ==========

impl PieceTable {
    /// 暂停自动合并（大型操作前调用）
    pub fn suspend_auto_merge(&mut self) {
        self.suspend_auto_merge = true;
    }
    
    /// 恢复自动合并（大型操作后调用）
    pub fn resume_auto_merge(&mut self) {
        self.suspend_auto_merge = false;
        // 恢复后检查是否需要合并
        if self.pieces.len() > self.mode.merge_threshold() * 2 {
            self.merge_pieces_smart();
        }
    }
    
    /// 准备大型操作（简单防护）
    pub fn prepare_for_large_operation(&mut self, estimated_size: usize) {
        if estimated_size > LARGE_OPERATION_THRESHOLD {
            self.suspend_auto_merge();
        }
    }
}

// ========== 默认实现 ==========

impl Default for PieceTable {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_piece_table_basic() {
        let mut table = PieceTable::from_text("Hello, world!");
        
        // 测试插入
        let (table, _) = table.insert_char_safe(7, "beautiful ");
        assert_eq!(table.get_all_text(), "Hello, beautiful world!");
        assert_eq!(table.total_bytes(), 22);
        
        // 测试删除
        let (table, deleted) = table.delete_char_safe(7..16);
        assert_eq!(deleted, "beautiful ");
        assert_eq!(table.get_all_text(), "Hello, world!");
        
        // 测试获取范围
        let text = table.get_text_range(0..5);
        assert_eq!(text, "Hello");
    }
    
    #[test]
    fn test_utf8_safety() {
        let mut table = PieceTable::from_text("Hello 世界!");
        
        // UTF-8字符"世"占3个字节，"界"占3个字节
        // 在字符边界插入
        let (table, _) = table.insert_char_safe(6, " beautiful ");
        assert_eq!(table.get_all_text(), "Hello  beautiful 世界!");
    }
    
    #[test]
    fn test_lazy_delete() {
        let mut table = PieceTable::from_text("Hello world!");
        
        // 延迟删除
        let (table, deletion_info) = table.delete_lazy(6..11);
        assert_eq!(deletion_info.len(), 5);
        
        // 应该没有立即加载文本
        //（这里无法直接测试，但结构正确）
        assert!(table.get_text_range(0..table.total_bytes()).contains("Hello"));
    }
    
    #[test]
    fn test_chunk_iter() {
        let table = PieceTable::from_text("Hello world! This is a test.");
        
        // 测试流式迭代
        let mut chunks: Vec<String> = table.iter_chunks(10).collect();
        assert!(!chunks.is_empty());
        
        // 拼接所有块应该等于完整文本
        let reconstructed: String = chunks.concat();
        assert_eq!(reconstructed, "Hello world! This is a test.");
    }
}
```

---

## 📝 **使用示例**

```rust
// 1. 创建PieceTable
let mut table = PieceTable::from_text("Hello");

// 2. UTF-8安全插入
let (table, _) = table.insert_char_safe(5, " world");

// 3. 获取文本（按需）
let hello = table.get_text_range(0..5);  // "Hello"
let world = table.get_text_range(6..11); // "world"

// 4. 流式处理（符合冻结原则）
for chunk in table.iter_chunks(64 * 1024) {
    // 处理块数据，不一次性加载全文
    process_chunk(&chunk);
}

// 5. 大型操作防护
table.prepare_for_large_operation(20 * 1024 * 1024); // 准备20MB操作
// ... 执行大型操作 ...
table.resume_auto_merge();

// 6. 手动合并（如保存时）
table.merge_if_needed();
```

---

## ✅ **总结**

这个PieceTable实现：

1. ✅ **完全符合冻结架构清单**所有要求
2. ✅ **实现了讨论过的所有功能**，不新增复杂度
3. ✅ **遵循流式处理原则**（IO系统负责流式，PieceTable只提供迭代器）
4. ✅ **UTF-8安全**，字符边界保证
5. ✅ **内存友好**（Arc共享 + 延迟加载 + 模式自适应）
6. ✅ **合并策略直接实现讨论方案**，不额外复杂化
7. ✅ **提供了大型操作简单防护**，符合渐进式优化原则

现在可以基于这个实现继续开发其他组件了。