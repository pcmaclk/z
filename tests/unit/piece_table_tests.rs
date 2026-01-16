// Piece Table 单元测试

use zedit::core::buffer::PieceTable;

#[test]
fn test_empty_table() {
    let table = PieceTable::new();
    assert!(table.is_empty());
    assert_eq!(table.total_bytes(), 0);
    assert_eq!(table.piece_count(), 0);
}

#[test]
fn test_from_text() {
    let table = PieceTable::from_text("Hello, world!");
    assert!(!table.is_empty());
    assert_eq!(table.total_bytes(), 13);
    assert_eq!(table.piece_count(), 1);
}

#[test]
fn test_basic_insert() {
    let mut table = PieceTable::from_text("Hello");
    let (table, _) = table.insert_char_safe(5, " world");
    assert_eq!(table.get_text_range(0..11), "Hello world");
    assert_eq!(table.total_bytes(), 11);
}

#[test]
fn test_basic_delete() {
    let mut table = PieceTable::from_text("Hello world");
    let (table, deleted) = table.delete_char_safe(5..6);
    assert_eq!(deleted, " ");
    assert_eq!(table.get_text_range(0..10), "Helloworld");
}

#[test]
fn test_insert_at_beginning() {
    let mut table = PieceTable::from_text("world");
    let (table, _) = table.insert_char_safe(0, "Hello ");
    assert_eq!(table.get_text_range(0..11), "Hello world");
}

#[test]
fn test_insert_at_end() {
    let mut table = PieceTable::from_text("Hello");
    let (table, _) = table.insert_char_safe(5, " world");
    assert_eq!(table.get_text_range(0..11), "Hello world");
}

#[test]
fn test_multiple_inserts() {
    let mut table = PieceTable::new();
    let (table, _) = table.insert_char_safe(0, "Hello");
    let (table, _) = table.insert_char_safe(5, " ");
    let (table, _) = table.insert_char_safe(6, "world");
    assert_eq!(table.get_text_range(0..11), "Hello world");
}

#[test]
fn test_undo_simulation() {
    let mut table = PieceTable::from_text("Hello world");
    
    // 删除 " world"
    let (table1, deleted1) = table.delete_char_safe(5..11);
    assert_eq!(deleted1, " world");
    assert_eq!(table1.get_text_range(0..5), "Hello");
    
    // 重新插入（模拟撤销）
    let (table2, _) = table1.insert_char_safe(5, " world");
    assert_eq!(table2.get_text_range(0..11), "Hello world");
}

#[test]
fn test_utf8_multibyte() {
    let mut table = PieceTable::from_text("Hello 世界");
    
    // 在UTF-8字符边界插入
    let (table, _) = table.insert_char_safe(6, " beautiful");
    assert_eq!(table.get_text_range(0..16), "Hello beautiful");
}

#[test]
fn test_get_text_range() {
    let table = PieceTable::from_text("Hello, world!");
    
    assert_eq!(table.get_text_range(0..5), "Hello");
    assert_eq!(table.get_text_range(7..12), "world");
    assert_eq!(table.get_text_range(0..13), "Hello, world!");
}

#[test]
fn test_piece_count_after_merge() {
    let mut table = PieceTable::from_text("Hello");
    
    // 多次插入会增加piece数量
    let (table, _) = table.insert_char_safe(5, " ");
    let (table, _) = table.insert_char_safe(6, "w");
    let (table, _) = table.insert_char_safe(7, "o");
    let (table, _) = table.insert_char_safe(8, "r");
    let (table, _) = table.insert_char_safe(9, "l");
    let (table, _) = table.insert_char_safe(10, "d");
    
    // 合并前piece数量应该较多
    let piece_count_before = table.piece_count();
    
    // 手动触发合并
    let mut table = table;
    table.merge_if_needed();
    
    // 合并后piece数量应该减少
    assert!(table.piece_count() <= piece_count_before);
}

#[test]
fn test_large_operation_protection() {
    let mut table = PieceTable::from_text("Hello");
    
    // 准备大型操作
    table.prepare_for_large_operation(20 * 1024 * 1024);
    
    // 执行多次插入
    for i in 0..100 {
        let (new_table, _) = table.insert_char_safe(table.total_bytes(), &format!(" {}", i));
        table = new_table;
    }
    
    // 恢复自动合并
    table.resume_auto_merge();
    
    // 验证内容正确
    let text = table.get_text_range(0..table.total_bytes());
    assert!(text.starts_with("Hello"));
}

#[test]
fn test_lazy_delete() {
    let mut table = PieceTable::from_text("Hello world!");
    
    // 延迟删除
    let (table, deletion_info) = table.delete_lazy(6..11);
    
    assert_eq!(deletion_info.len(), 5);
    assert_eq!(table.get_text_range(0..5), "Hello");
    assert_eq!(table.get_text_range(5..6), "!");
}

#[test]
fn test_chunk_iteration() {
    let table = PieceTable::from_text("Hello world! This is a test.");
    
    let mut chunks: Vec<String> = table.iter_chunks(10).collect();
    assert!(!chunks.is_empty());
    
    // 拼接所有块应该等于完整文本
    let reconstructed: String = chunks.concat();
    assert_eq!(reconstructed, "Hello world! This is a test.");
}

#[test]
fn test_buffer_mode_selection() {
    let small_table = PieceTable::from_text("small");
    assert!(!small_table.mode().is_large_file());
    
    let large_text = "x".repeat(20 * 1024 * 1024);
    let large_table = PieceTable::from_text(&large_text);
    assert!(large_table.mode().is_large_file());
}

#[test]
fn test_memory_estimation() {
    let table = PieceTable::from_text("Hello world!");
    let memory = table.estimated_memory();
    
    // 内存估算应该大于0
    assert!(memory > 0);
}
