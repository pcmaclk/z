// Piece Table 核心实现
//
// 职责：使用 Piece Table 数据结构管理文本内容，
//       支持高效插入、删除、撤销重做操作

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

// ========== UTF-8安全操作 ==========

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

        // 6. 智能合并决策
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

// ========== 文本获取 ==========

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

    /// 获取全部文本（仅用于测试或小文件）
    #[cfg(test)]
    pub fn get_all_text(&self) -> String {
        self.get_text_range(0..self.total_bytes)
    }

    /// 获取指定行的文本
    pub fn get_line(&self, line_number: usize) -> Option<String> {
        self.lines.as_ref()?.get_line_range(line_number)
            .map(|range| self.get_text_range(range))
    }

    /// 创建流式迭代器
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

        let is_dirty = self.lines.as_ref().map(|l| l.is_dirty()).unwrap_or(false);

        if is_dirty {
            // 对于大文件，可以延迟构建或增量构建
            // 这里简化实现：全量构建
            let text = self.get_text_range(0..self.total_bytes.min(10 * 1024 * 1024)); // 最多10MB
            if let Some(ref mut lines) = self.lines {
                lines.build_from_text(&text);
            }
        }

        self.lines.as_ref().unwrap()
    }

    /// 获取行索引（如果存在）
    pub fn lines(&self) -> Option<&Lines> {
        self.lines.as_ref()
    }
}

// ========== 合并策略 ==========

impl PieceTable {
    /// 判断编辑后是否应该合并
    fn should_merge_after_edit(&self) -> bool {
        if self.suspend_auto_merge {
            return false;
        }

        let piece_count = self.pieces.len();
        let threshold = self.mode.merge_threshold();

        match self.mode {
            BufferMode::InMemory { merge_on_edit, .. } => {
                merge_on_edit && piece_count > threshold
            }
            BufferMode::MemoryMapped { merge_on_idle, .. } => {
                let is_idle = self.edit_count_since_last_merge > 100;
                merge_on_idle && is_idle && piece_count > threshold
            }
            BufferMode::Restricted { disable_merge, .. } => {
                !disable_merge && piece_count > threshold * 2
            }
        }
    }

    /// 智能合并实现
    fn merge_pieces_smart(&mut self) {
        match self.mode {
            BufferMode::InMemory { .. } => {
                self.merge_all_adjacent();
            }
            BufferMode::MemoryMapped { max_merge_size, .. } => {
                self.merge_incremental(max_merge_size);
            }
            BufferMode::Restricted { .. } => {
                self.merge_small_fragments_only(1024);
            }
        }

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

// ========== 大型操作防护 ==========

impl PieceTable {
    /// 暂停自动合并
    pub fn suspend_auto_merge(&mut self) {
        self.suspend_auto_merge = true;
    }

    /// 恢复自动合并
    pub fn resume_auto_merge(&mut self) {
        self.suspend_auto_merge = false;
        if self.pieces.len() > self.mode.merge_threshold() * 2 {
            self.merge_pieces_smart();
        }
    }

    /// 准备大型操作
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
        let (new_table, _) = table.insert_char_safe(7, "beautiful ");
        table = new_table;
        assert_eq!(table.get_all_text(), "Hello, beautiful world!");
        // "Hello, world!"(13) + "beautiful "(10) = 23
        assert_eq!(table.total_bytes(), 23);

        // 测试删除
        let (new_table, deleted) = table.delete_char_safe(7..17);
        table = new_table;
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
        let (new_table, _) = table.insert_char_safe(6, " beautiful ");
        table = new_table;
        assert_eq!(table.get_all_text(), "Hello  beautiful 世界!");
    }

    #[test]
    fn test_lazy_delete() {
        let mut table = PieceTable::from_text("Hello world!");

        // 延迟删除
        let (new_table, deletion_info) = table.delete_lazy(6..11);
        table = new_table;
        assert_eq!(deletion_info.len(), 5);
        assert!(table.get_text_range(0..table.total_bytes()).contains("Hello"));
    }

    #[test]
    fn test_chunk_iter() {
        let table = PieceTable::from_text("Hello world! This is a test.");

        // 测试流式迭代
        let chunks: Vec<String> = table.iter_chunks(10).collect();
        assert!(!chunks.is_empty());

        // 拼接所有块应该等于完整文本
        let reconstructed: String = chunks.concat();
        assert_eq!(reconstructed, "Hello world! This is a test.");
    }
}
