// 行索引管理
//
// 职责：维护文本行号与字节偏移的映射关系，支持快速行查找

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
    pub fn handle_insert(&mut self, _offset: usize, text: &str) {
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
