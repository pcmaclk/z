// Piece Table 集成测试

use zedit::core::buffer::PieceTable;

#[test]
fn test_edit_workflow() {
    // 模拟真实编辑场景
    let mut table = PieceTable::from_text("fn main() {\n    println!(\"Hello\");\n}");
    
    // 在函数内添加代码
    let (table, _) = table.insert_char_safe(26, "    println!(\"World\");\n");
    
    let text = table.get_text_range(0..table.total_bytes());
    assert!(text.contains("println!(\"Hello\")"));
    assert!(text.contains("println!(\"World\")"));
}

#[test]
fn test_complex_undo_redo() {
    // 模拟多次编辑和撤销
    let mut table = PieceTable::from_text("Hello");
    
    // 编辑序列
    let (t1, _) = table.insert_char_safe(5, " world");
    let (t2, _) = t1.insert_char_safe(11, "!");
    let (t3, del1) = t2.delete_char_safe(6..11);
    
    assert_eq!(del1, "world");
    assert_eq!(t3.get_text_range(0..7), "Hello !");
    
    // 撤销删除（重新插入）
    let (t4, _) = t3.insert_char_safe(6, "world");
    assert_eq!(t4.get_text_range(0..17), "Hello world!");
}

#[test]
fn test_multiline_edit() {
    let text = "Line 1\nLine 2\nLine 3";
    let mut table = PieceTable::from_text(text);
    
    // 在第二行插入
    let (table, _) = table.insert_char_safe(13, " inserted");
    
    let result = table.get_text_range(0..table.total_bytes());
    assert!(result.contains("Line 2 inserted"));
}

#[test]
fn test_large_text_performance() {
    // 测试大文本性能
    let large_text = "x".repeat(1_000_000); // 1MB
    let table = PieceTable::from_text(&large_text);
    
    // 验证可以正常操作
    assert_eq!(table.total_bytes(), 1_000_000);
    
    let mut table = table;
    let (table, _) = table.insert_char_safe(500_000, "INSERT");
    
    assert_eq!(table.total_bytes(), 1_000_006);
    assert!(table.get_text_range(500_000..500_006).contains("INSERT"));
}
