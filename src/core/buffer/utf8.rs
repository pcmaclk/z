// UTF-8边界安全工具
//
// 职责：确保所有字节操作都在UTF-8字符边界上进行

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_char_boundary() {
        let text = "Hello 世界"; // "世"占3字节

        // ASCII字符边界
        assert_eq!(Utf8Validator::ensure_char_boundary(text, 5), 5);

        // 非字符边界，应该调整
        assert_eq!(Utf8Validator::ensure_char_boundary(text, 6), 6); // "世"的开头
        assert_eq!(Utf8Validator::ensure_char_boundary(text, 7), 6); // 在"世"中间，调整到开头
    }

    #[test]
    fn test_ensure_char_boundary_range() {
        let text = "Hello 世界";

        let range = Utf8Validator::ensure_char_boundary_range(text, 6..9);
        assert_eq!(range, 6..9); // "世"的完整范围
    }

    #[test]
    fn test_safe_substr() {
        let text = "Hello 世界";

        assert_eq!(Utf8Validator::safe_substr(text, 0, 5), "Hello");
        assert_eq!(Utf8Validator::safe_substr(text, 6, 9), "世");
    }
}
