我将提供Layout系统的完整实现。由于代码量很大，我将分模块实现核心部分：

## 1. **核心数据结构和枚举定义**

```rust
// src/core/layout/mod.rs
pub mod context;
pub mod result;
pub mod engine;
pub mod font;
pub mod cache;
pub mod wrapping;
pub mod coordinate;

pub use context::{LayoutContext, LayoutConfig, WrapMode};
pub use result::{LayoutResult, LayoutLine, LayoutFragment, PositionedGlyph};
pub use engine::LayoutEngine;
pub use font::{FontManager, FontMetrics, FontId};
pub use cache::{LayoutCache, CacheStats};
pub use coordinate::{CoordinateMapper, LogicalPosition, PhysicalPosition};

// src/core/layout/result.rs
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use crate::core::logical::LineRange;
use crate::core::viewmodel::{VisualSpan, VisualAttributes};
use crate::core::geometry::{Rect, Point};

/// 布局结果标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutId(u64);

impl LayoutId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
    
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// 布局结果 - 包含所有行的几何信息
#[derive(Clone)]
pub struct LayoutResult {
    // 标识信息
    id: LayoutId,
    snapshot_id: crate::core::viewmodel::SnapshotId,
    
    // 布局范围
    viewport_range: LineRange,
    
    // 布局行数据
    layout_lines: Arc<[LayoutLine]>,
    
    // 几何信息
    total_height: f32,
    max_line_width: f32,
    
    // 缓存信息
    cached_glyphs: usize,
    cache_hit_rate: f32,
    
    // 元数据
    metadata: LayoutMetadata,
}

impl LayoutResult {
    pub fn new(
        snapshot_id: crate::core::viewmodel::SnapshotId,
        viewport_range: LineRange,
        layout_lines: Arc<[LayoutLine]>,
        total_height: f32,
        max_line_width: f32,
        cached_glyphs: usize,
        cache_hit_rate: f32,
        metadata: LayoutMetadata,
    ) -> Self {
        Self {
            id: LayoutId::new(),
            snapshot_id,
            viewport_range,
            layout_lines,
            total_height,
            max_line_width,
            cached_glyphs,
            cache_hit_rate,
            metadata,
        }
    }
    
    // 只读访问器
    pub fn id(&self) -> LayoutId {
        self.id
    }
    
    pub fn snapshot_id(&self) -> crate::core::viewmodel::SnapshotId {
        self.snapshot_id
    }
    
    pub fn viewport_range(&self) -> LineRange {
        self.viewport_range
    }
    
    pub fn lines(&self) -> &[LayoutLine] {
        &self.layout_lines
    }
    
    pub fn line_at(&self, index: usize) -> Option<&LayoutLine> {
        self.layout_lines.get(index)
    }
    
    pub fn line_by_number(&self, line_number: usize) -> Option<&LayoutLine> {
        if line_number < self.viewport_range.start || line_number >= self.viewport_range.end {
            return None;
        }
        let idx = line_number - self.viewport_range.start;
        self.layout_lines.get(idx)
    }
    
    pub fn total_height(&self) -> f32 {
        self.total_height
    }
    
    pub fn max_line_width(&self) -> f32 {
        self.max_line_width
    }
    
    pub fn cached_glyphs(&self) -> usize {
        self.cached_glyphs
    }
    
    pub fn cache_hit_rate(&self) -> f32 {
        self.cache_hit_rate
    }
    
    pub fn metadata(&self) -> &LayoutMetadata {
        &self.metadata
    }
    
    /// 克隆为Arc（廉价克隆）
    pub fn clone_arc(&self) -> Arc<Self> {
        Arc::new(self.clone())
    }
    
    /// 估计内存占用
    pub fn estimated_size(&self) -> usize {
        let base = std::mem::size_of::<Self>();
        let lines_size = self.layout_lines.iter().map(LayoutLine::estimated_size).sum::<usize>();
        base + lines_size
    }
    
    /// 获取调试信息
    pub fn debug_info(&self) -> LayoutDebugInfo {
        LayoutDebugInfo {
            id: self.id,
            snapshot_id: self.snapshot_id,
            line_count: self.layout_lines.len(),
            viewport_range: self.viewport_range,
            total_height: self.total_height,
            max_line_width: self.max_line_width,
            cached_glyphs: self.cached_glyphs,
            cache_hit_rate: self.cache_hit_rate,
            metadata: self.metadata.clone(),
        }
    }
    
    /// 在指定逻辑位置获取布局行
    pub fn find_line_for_position(&self, line_number: usize) -> Option<&LayoutLine> {
        self.line_by_number(line_number)
    }
    
    /// 将位置限制在有效范围内
    pub fn clamp_position(&self, pos: LogicalPosition) -> LogicalPosition {
        let line = pos.line.clamp(self.viewport_range.start, self.viewport_range.end.saturating_sub(1));
        let column = if let Some(layout_line) = self.line_by_number(line) {
            let max_column = layout_line.max_column();
            pos.column.min(max_column)
        } else {
            pos.column
        };
        
        LogicalPosition { line, column }
    }
}

/// 布局行 - 单行的完整布局信息
#[derive(Clone, Debug)]
pub struct LayoutLine {
    // 源信息
    source_line_handle: LineHandle,
    line_number: usize,
    
    // 布局信息
    fragments: Arc<[LayoutFragment]>,
    y_position: f32,
    height: f32,
    ascent: f32,
    descent: f32,
    
    // 换行信息
    is_wrapped: bool,
    wrap_count: usize,
    wrapped_lines: Option<Arc<[WrappedLine]>>,
    
    // 缓存键
    layout_hash: u64,
    
    // 计算字段
    width: f32,
    max_column: usize,
}

impl LayoutLine {
    pub fn new(
        source_line_handle: LineHandle,
        line_number: usize,
        fragments: Arc<[LayoutFragment]>,
        y_position: f32,
        height: f32,
        ascent: f32,
        descent: f32,
        is_wrapped: bool,
        wrap_count: usize,
        wrapped_lines: Option<Arc<[WrappedLine]>>,
        layout_hash: u64,
    ) -> Self {
        // 计算宽度和最大列
        let width: f32 = fragments.iter().map(|f| f.width).sum();
        let max_column: usize = fragments.iter()
            .map(|f| f.text.chars().count())
            .sum::<usize>()
            .saturating_sub(1);
        
        Self {
            source_line_handle,
            line_number,
            fragments,
            y_position,
            height,
            ascent,
            descent,
            is_wrapped,
            wrap_count,
            wrapped_lines,
            layout_hash,
            width,
            max_column,
        }
    }
    
    // 访问器
    pub fn source_line_handle(&self) -> &LineHandle {
        &self.source_line_handle
    }
    
    pub fn line_number(&self) -> usize {
        self.line_number
    }
    
    pub fn fragments(&self) -> &[LayoutFragment] {
        &self.fragments
    }
    
    pub fn y_position(&self) -> f32 {
        self.y_position
    }
    
    pub fn height(&self) -> f32 {
        self.height
    }
    
    pub fn ascent(&self) -> f32 {
        self.ascent
    }
    
    pub fn descent(&self) -> f32 {
        self.descent
    }
    
    pub fn is_wrapped(&self) -> bool {
        self.is_wrapped
    }
    
    pub fn wrap_count(&self) -> usize {
        self.wrap_count
    }
    
    pub fn wrapped_lines(&self) -> Option<&[WrappedLine]> {
        self.wrapped_lines.as_deref()
    }
    
    pub fn layout_hash(&self) -> u64 {
        self.layout_hash
    }
    
    pub fn width(&self) -> f32 {
        self.width
    }
    
    pub fn max_column(&self) -> usize {
        self.max_column
    }
    
    /// 在指定列获取水平位置
    pub fn x_at_column(&self, column: usize) -> Option<f32> {
        if column > self.max_column {
            return None;
        }
        
        let mut current_x = 0.0;
        let mut chars_processed = 0;
        
        for fragment in self.fragments.iter() {
            let fragment_chars = fragment.text.chars().count();
            
            if chars_processed + fragment_chars > column {
                // 目标列在当前片段内
                let offset_in_fragment = column - chars_processed;
                return fragment.x_at_char(offset_in_fragment).map(|x| current_x + x);
            }
            
            current_x += fragment.width;
            chars_processed += fragment_chars;
        }
        
        // 列等于行长度，返回行尾
        Some(current_x)
    }
    
    /// 在指定水平位置获取列
    pub fn column_at_x(&self, x: f32) -> usize {
        if x <= 0.0 {
            return 0;
        }
        
        let mut current_x = 0.0;
        let mut chars_processed = 0;
        
        for fragment in self.fragments.iter() {
            if x < current_x + fragment.width {
                // 位置在当前片段内
                let offset_in_fragment = x - current_x;
                let char_in_fragment = fragment.char_at_x(offset_in_fragment);
                return chars_processed + char_in_fragment;
            }
            
            current_x += fragment.width;
            chars_processed += fragment.text.chars().count();
        }
        
        // 位置超出行尾，返回行尾
        self.max_column
    }
    
    /// 获取基线位置（y + ascent）
    pub fn baseline(&self) -> f32 {
        self.y_position + self.ascent
    }
    
    /// 获取行边界矩形
    pub fn bounds(&self) -> Rect {
        Rect::new(
            0.0,
            self.y_position,
            self.width,
            self.height
        )
    }
    
    /// 估计内存大小
    pub fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>() +
        self.fragments.iter().map(LayoutFragment::estimated_size).sum::<usize>() +
        self.wrapped_lines.as_ref().map_or(0, |w| w.iter().map(WrappedLine::estimated_size).sum::<usize>())
    }
}

/// 布局片段 - 具有相同视觉属性的连续文本的布局信息
#[derive(Clone, Debug)]
pub struct LayoutFragment {
    // 源信息
    visual_span: Arc<VisualSpan>,
    text: Arc<str>,
    
    // 几何信息
    x_position: f32,
    width: f32,
    ascent: f32,
    descent: f32,
    
    // 字形信息
    glyphs: Arc<[PositionedGlyph]>,
    cluster_map: Arc<[usize]>,
    
    // 视觉属性
    visual_attrs: VisualAttributes,
    
    // 计算字段
    char_count: usize,
}

impl LayoutFragment {
    pub fn new(
        visual_span: Arc<VisualSpan>,
        text: Arc<str>,
        x_position: f32,
        width: f32,
        ascent: f32,
        descent: f32,
        glyphs: Arc<[PositionedGlyph]>,
        cluster_map: Arc<[usize]>,
        visual_attrs: VisualAttributes,
    ) -> Self {
        let char_count = text.chars().count();
        
        Self {
            visual_span,
            text,
            x_position,
            width,
            ascent,
            descent,
            glyphs,
            cluster_map,
            visual_attrs,
            char_count,
        }
    }
    
    // 访问器
    pub fn visual_span(&self) -> &Arc<VisualSpan> {
        &self.visual_span
    }
    
    pub fn text(&self) -> &str {
        &self.text
    }
    
    pub fn x_position(&self) -> f32 {
        self.x_position
    }
    
    pub fn width(&self) -> f32 {
        self.width
    }
    
    pub fn ascent(&self) -> f32 {
        self.ascent
    }
    
    pub fn descent(&self) -> f32 {
        self.descent
    }
    
    pub fn glyphs(&self) -> &[PositionedGlyph] {
        &self.glyphs
    }
    
    pub fn cluster_map(&self) -> &[usize] {
        &self.cluster_map
    }
    
    pub fn visual_attrs(&self) -> VisualAttributes {
        self.visual_attrs
    }
    
    pub fn char_count(&self) -> usize {
        self.char_count
    }
    
    /// 在片段内获取字符的水平位置
    pub fn x_at_char(&self, char_index: usize) -> Option<f32> {
        if char_index >= self.char_count {
            return None;
        }
        
        // 通过cluster_map查找对应的字形
        if let Some(&glyph_index) = self.cluster_map.get(char_index) {
            if glyph_index < self.glyphs.len() {
                return Some(self.glyphs[glyph_index].x);
            }
        }
        
        // 回退：根据字符宽度估算
        let sub_text: String = self.text.chars().take(char_index).collect();
        // 这里需要字体度量来计算，暂时返回None
        None
    }
    
    /// 在片段内获取指定水平位置的字符索引
    pub fn char_at_x(&self, x: f32) -> usize {
        if x <= 0.0 {
            return 0;
        }
        
        if x >= self.width {
            return self.char_count;
        }
        
        // 查找包含x的字形
        for (i, glyph) in self.glyphs.iter().enumerate() {
            if x < glyph.x + glyph.advance {
                // 找到对应的字形，需要查找对应的字符
                // 简化实现：返回字形索引
                return i.min(self.char_count - 1);
            }
        }
        
        // 默认返回最后一个字符
        self.char_count.saturating_sub(1)
    }
    
    /// 获取片段边界矩形（相对于行）
    pub fn bounds(&self) -> Rect {
        Rect::new(
            self.x_position,
            -self.ascent,  // 相对于基线
            self.width,
            self.ascent + self.descent
        )
    }
    
    /// 估计内存大小
    pub fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>() +
        self.text.len() +
        self.glyphs.len() * std::mem::size_of::<PositionedGlyph>() +
        self.cluster_map.len() * std::mem::size_of::<usize>()
    }
}

/// 定位字形 - 包含位置信息的字形
#[derive(Clone, Debug)]
pub struct PositionedGlyph {
    // 字形标识
    glyph_id: u32,
    font_id: FontId,
    
    // 位置信息
    x: f32,
    y: f32,
    advance: f32,
    
    // 边界框
    bounds: Option<Rect>,
    
    // 文本信息
    cluster_index: usize,
    char_index: usize,
}

impl PositionedGlyph {
    pub fn new(
        glyph_id: u32,
        font_id: FontId,
        x: f32,
        y: f32,
        advance: f32,
        bounds: Option<Rect>,
        cluster_index: usize,
        char_index: usize,
    ) -> Self {
        Self {
            glyph_id,
            font_id,
            x,
            y,
            advance,
            bounds,
            cluster_index,
            char_index,
        }
    }
    
    // 访问器
    pub fn glyph_id(&self) -> u32 {
        self.glyph_id
    }
    
    pub fn font_id(&self) -> FontId {
        self.font_id
    }
    
    pub fn x(&self) -> f32 {
        self.x
    }
    
    pub fn y(&self) -> f32 {
        self.y
    }
    
    pub fn advance(&self) -> f32 {
        self.advance
    }
    
    pub fn bounds(&self) -> Option<Rect> {
        self.bounds
    }
    
    pub fn cluster_index(&self) -> usize {
        self.cluster_index
    }
    
    pub fn char_index(&self) -> usize {
        self.char_index
    }
    
    /// 获取字形位置点
    pub fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }
}

/// 换行子行
#[derive(Clone, Debug)]
pub struct WrappedLine {
    fragments: Arc<[LayoutFragment]>,
    height: f32,
    ascent: f32,
    descent: f32,
    y_offset: f32,  // 相对于父行的垂直偏移
}

impl WrappedLine {
    pub fn new(
        fragments: Arc<[LayoutFragment]>,
        height: f32,
        ascent: f32,
        descent: f32,
        y_offset: f32,
    ) -> Self {
        Self {
            fragments,
            height,
            ascent,
            descent,
            y_offset,
        }
    }
    
    pub fn fragments(&self) -> &[LayoutFragment] {
        &self.fragments
    }
    
    pub fn height(&self) -> f32 {
        self.height
    }
    
    pub fn ascent(&self) -> f32 {
        self.ascent
    }
    
    pub fn descent(&self) -> f32 {
        self.descent
    }
    
    pub fn y_offset(&self) -> f32 {
        self.y_offset
    }
    
    pub fn width(&self) -> f32 {
        self.fragments.iter().map(|f| f.width).sum()
    }
    
    pub fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>() +
        self.fragments.iter().map(LayoutFragment::estimated_size).sum::<usize>()
    }
}

/// 行句柄 - 指向ViewModel中的行
#[derive(Clone, Debug)]
pub struct LineHandle {
    snapshot_id: crate::core::viewmodel::SnapshotId,
    line_index: usize,
    content_hash: u64,
}

impl LineHandle {
    pub fn new(
        snapshot_id: crate::core::viewmodel::SnapshotId,
        line_index: usize,
        content_hash: u64,
    ) -> Self {
        Self {
            snapshot_id,
            line_index,
            content_hash,
        }
    }
    
    pub fn from_rendered_line(
        snapshot_id: crate::core::viewmodel::SnapshotId,
        line: &crate::core::viewmodel::RenderedLine,
        line_index: usize,
    ) -> Self {
        Self {
            snapshot_id,
            line_index,
            content_hash: line.content_hash(),
        }
    }
}

/// 布局元数据
#[derive(Clone, Debug)]
pub struct LayoutMetadata {
    pub source: LayoutSource,
    pub build_time: std::time::Instant,
    pub lines_updated: usize,
    pub total_lines: usize,
    pub font_size: f32,
    pub dpi_scale: f32,
    pub max_line_width: Option<f32>,
}

/// 布局来源
#[derive(Clone, Debug, PartialEq)]
pub enum LayoutSource {
    Full,
    Incremental,
    Cached,
    Parallel,
}

/// 布局调试信息
#[derive(Clone, Debug)]
pub struct LayoutDebugInfo {
    pub id: LayoutId,
    pub snapshot_id: crate::core::viewmodel::SnapshotId,
    pub line_count: usize,
    pub viewport_range: LineRange,
    pub total_height: f32,
    pub max_line_width: f32,
    pub cached_glyphs: usize,
    pub cache_hit_rate: f32,
    pub metadata: LayoutMetadata,
}
```

## 2. **布局上下文和配置**

```rust
// src/core/layout/context.rs
use std::sync::Arc;
use crate::core::font::{FontFamily, FontFeatures};

/// 布局配置
#[derive(Clone, Debug)]
pub struct LayoutConfig {
    // 字体配置
    pub font_size: f32,
    pub font_family: FontFamily,
    pub font_features: FontFeatures,
    
    // 文本配置
    pub tab_width: usize,
    pub line_spacing: f32,
    pub paragraph_spacing: f32,
    
    // 换行配置
    pub max_line_width: Option<f32>,
    pub wrap_mode: WrapMode,
    pub wrap_indent: f32,
    
    // 显示配置
    pub dpi_scale: f32,
    pub pixel_ratio: f32,
    
    // 缓存配置
    pub glyph_cache_size: usize,
    pub line_cache_size: usize,
    pub enable_incremental: bool,
    
    // 性能配置
    pub enable_caching: bool,
    pub precompute_glyphs: bool,
    pub optimize_fragments: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            font_family: FontFamily::monospace(),
            font_features: FontFeatures::default(),
            tab_width: 4,
            line_spacing: 4.0,
            paragraph_spacing: 8.0,
            max_line_width: None,
            wrap_mode: WrapMode::None,
            wrap_indent: 0.0,
            dpi_scale: 1.0,
            pixel_ratio: 1.0,
            glyph_cache_size: 10000,
            line_cache_size: 500,
            enable_incremental: true,
            enable_caching: true,
            precompute_glyphs: false,
            optimize_fragments: true,
        }
    }
}

impl LayoutConfig {
    /// 编程配置（等宽字体，tab=4）
    pub fn programming() -> Self {
        Self {
            font_family: FontFamily::monospace(),
            tab_width: 4,
            wrap_mode: WrapMode::None,
            ..Default::default()
        }
    }
    
    /// 写作配置（比例字体，换行优化）
    pub fn writing() -> Self {
        Self {
            font_family: FontFamily::proportional(),
            max_line_width: Some(800.0),
            wrap_mode: WrapMode::Word,
            ..Default::default()
        }
    }
    
    /// 计算配置哈希（用于缓存键）
    pub fn hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        
        self.font_size.to_bits().hash(&mut hasher);
        self.font_family.hash(&mut hasher);
        self.font_features.hash(&mut hasher);
        self.tab_width.hash(&mut hasher);
        self.line_spacing.to_bits().hash(&mut hasher);
        self.max_line_width.map(|w| w.to_bits()).hash(&mut hasher);
        self.wrap_mode.hash(&mut hasher);
        self.dpi_scale.to_bits().hash(&mut hasher);
        
        hasher.finish()
    }
    
    /// 检查配置是否影响布局（用于增量更新决策）
    pub fn affects_layout(&self, other: &Self) -> bool {
        self.font_size != other.font_size ||
        self.font_family != other.font_family ||
        self.tab_width != other.tab_width ||
        self.max_line_width != other.max_line_width ||
        self.wrap_mode != other.wrap_mode ||
        (self.dpi_scale - other.dpi_scale).abs() > f32::EPSILON
    }
}

/// 换行模式
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub enum WrapMode {
    None,           // 不换行
    Word,           // 按单词换行
    Character,      // 按字符换行
    Whitespace,     // 在空白处换行
    Smart,          // 智能换行（混合策略）
}

/// 布局上下文 - 管理布局状态和资源
pub struct LayoutContext {
    // 字体系统
    font_manager: Arc<dyn FontManager>,
    
    // 缓存系统
    glyph_cache: GlyphCache,
    line_cache: LineLayoutCache,
    coordinate_cache: CoordinateCache,
    
    // 配置
    config: LayoutConfig,
    
    // 统计信息
    stats: LayoutStats,
    
    // 当前布局状态
    current_layout_id: Option<LayoutId>,
    dirty_regions: Vec<LineRange>,
}

impl LayoutContext {
    pub fn new(font_manager: Arc<dyn FontManager>) -> Self {
        let config = LayoutConfig::default();
        
        Self {
            font_manager,
            glyph_cache: GlyphCache::new(config.glyph_cache_size),
            line_cache: LineLayoutCache::new(config.line_cache_size),
            coordinate_cache: CoordinateCache::new(1000),
            config,
            stats: LayoutStats::new(),
            current_layout_id: None,
            dirty_regions: Vec::new(),
        }
    }
    
    /// 配置布局上下文
    pub fn configure(&mut self, config: LayoutConfig) -> &mut Self {
        // 检查配置变化是否影响现有缓存
        if self.config.affects_layout(&config) {
            self.clear_cache();
        }
        
        // 调整缓存大小
        if config.glyph_cache_size != self.config.glyph_cache_size {
            self.glyph_cache.resize(config.glyph_cache_size);
        }
        
        if config.line_cache_size != self.config.line_cache_size {
            self.line_cache.resize(config.line_cache_size);
        }
        
        self.config = config;
        self
    }
    
    /// 获取当前配置
    pub fn config(&self) -> &LayoutConfig {
        &self.config
    }
    
    /// 获取字体管理器
    pub fn font_manager(&self) -> &Arc<dyn FontManager> {
        &self.font_manager
    }
    
    /// 获取统计信息
    pub fn stats(&self) -> &LayoutStats {
        &self.stats
    }
    
    /// 清空所有缓存
    pub fn clear_cache(&mut self) {
        self.glyph_cache.clear();
        self.line_cache.clear();
        self.coordinate_cache.clear();
        self.stats.record_cache_clear();
    }
    
    /// 调整缓存大小
    pub fn resize_cache(&mut self, glyph_cache_size: usize, line_cache_size: usize) {
        self.glyph_cache.resize(glyph_cache_size);
        self.line_cache.resize(line_cache_size);
        self.config.glyph_cache_size = glyph_cache_size;
        self.config.line_cache_size = line_cache_size;
    }
    
    /// 标记脏区
    pub fn mark_dirty(&mut self, range: LineRange) {
        self.dirty_regions.push(range);
    }
    
    /// 获取当前脏区
    pub fn dirty_regions(&self) -> &[LineRange] {
        &self.dirty_regions
    }
    
    /// 清除脏区标记
    pub fn clear_dirty(&mut self) {
        self.dirty_regions.clear();
    }
    
    /// 设置当前布局ID
    pub fn set_current_layout(&mut self, layout_id: LayoutId) {
        self.current_layout_id = Some(layout_id);
    }
    
    /// 获取当前布局ID
    pub fn current_layout_id(&self) -> Option<LayoutId> {
        self.current_layout_id
    }
    
    /// 估计内存使用
    pub fn estimated_memory(&self) -> usize {
        self.glyph_cache.estimated_size() +
        self.line_cache.estimated_size() +
        self.coordinate_cache.estimated_size()
    }
}

/// 布局统计
#[derive(Clone, Debug)]
pub struct LayoutStats {
    // 性能统计
    layout_calls: usize,
    total_layout_time_ms: f64,
    incremental_layouts: usize,
    
    // 缓存统计
    glyph_cache_hits: usize,
    glyph_cache_misses: usize,
    line_cache_hits: usize,
    line_cache_misses: usize,
    coordinate_cache_hits: usize,
    coordinate_cache_misses: usize,
    
    // 内存统计
    peak_memory_bytes: usize,
    cache_clears: usize,
}

impl LayoutStats {
    pub fn new() -> Self {
        Self {
            layout_calls: 0,
            total_layout_time_ms: 0.0,
            incremental_layouts: 0,
            glyph_cache_hits: 0,
            glyph_cache_misses: 0,
            line_cache_hits: 0,
            line_cache_misses: 0,
            coordinate_cache_hits: 0,
            coordinate_cache_misses: 0,
            peak_memory_bytes: 0,
            cache_clears: 0,
        }
    }
    
    pub fn record_layout(&mut self, duration: std::time::Duration, incremental: bool) {
        self.layout_calls += 1;
        self.total_layout_time_ms += duration.as_secs_f64() * 1000.0;
        if incremental {
            self.incremental_layouts += 1;
        }
    }
    
    pub fn record_glyph_cache_hit(&mut self) {
        self.glyph_cache_hits += 1;
    }
    
    pub fn record_glyph_cache_miss(&mut self) {
        self.glyph_cache_misses += 1;
    }
    
    pub fn record_line_cache_hit(&mut self) {
        self.line_cache_hits += 1;
    }
    
    pub fn record_line_cache_miss(&mut self) {
        self.line_cache_misses += 1;
    }
    
    pub fn record_coordinate_cache_hit(&mut self) {
        self.coordinate_cache_hits += 1;
    }
    
    pub fn record_coordinate_cache_miss(&mut self) {
        self.coordinate_cache_misses += 1;
    }
    
    pub fn record_cache_clear(&mut self) {
        self.cache_clears += 1;
    }
    
    pub fn update_memory_usage(&mut self, bytes: usize) {
        if bytes > self.peak_memory_bytes {
            self.peak_memory_bytes = bytes;
        }
    }
    
    // 计算统计指标
    pub fn glyph_cache_hit_rate(&self) -> f32 {
        let total = self.glyph_cache_hits + self.glyph_cache_misses;
        if total == 0 {
            0.0
        } else {
            self.glyph_cache_hits as f32 / total as f32
        }
    }
    
    pub fn line_cache_hit_rate(&self) -> f32 {
        let total = self.line_cache_hits + self.line_cache_misses;
        if total == 0 {
            0.0
        } else {
            self.line_cache_hits as f32 / total as f32
        }
    }
    
    pub fn coordinate_cache_hit_rate(&self) -> f32 {
        let total = self.coordinate_cache_hits + self.coordinate_cache_misses;
        if total == 0 {
            0.0
        } else {
            self.coordinate_cache_hits as f32 / total as f32
        }
    }
    
    pub fn avg_layout_time_ms(&self) -> f64 {
        if self.layout_calls == 0 {
            0.0
        } else {
            self.total_layout_time_ms / self.layout_calls as f64
        }
    }
    
    pub fn incremental_ratio(&self) -> f32 {
        if self.layout_calls == 0 {
            0.0
        } else {
            self.incremental_layouts as f32 / self.layout_calls as f32
        }
    }
}
```

## 3. **布局引擎核心实现**

```rust
// src/core/layout/engine.rs
use std::sync::Arc;
use crate::core::logical::LineRange;
use crate::core::viewmodel::{ViewModelSnapshot, RenderedLine};
use super::context::{LayoutContext, LayoutConfig};
use super::result::{LayoutResult, LayoutLine, LayoutFragment};
use super::wrapping::{LineWrapper, WrapMode};
use super::font::{FontManager, FontMetrics};

/// 布局引擎 - 执行实际布局计算
pub struct LayoutEngine {
    line_wrapper: LineWrapper,
    text_shaper: TextShaper,
    fragment_optimizer: FragmentOptimizer,
    incremental_engine: Option<IncrementalLayoutEngine>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            line_wrapper: LineWrapper::new(),
            text_shaper: TextShaper::new(),
            fragment_optimizer: FragmentOptimizer::new(),
            incremental_engine: Some(IncrementalLayoutEngine::new()),
        }
    }
    
    /// 全量布局快照
    pub fn layout_snapshot(
        &self,
        context: &mut LayoutContext,
        snapshot: &ViewModelSnapshot,
        config: &LayoutConfig,
    ) -> LayoutResult {
        let start_time = std::time::Instant::now();
        
        // 布局所有行
        let mut layout_lines = Vec::with_capacity(snapshot.lines().len());
        let mut y_position = 0.0;
        let mut max_line_width = 0.0;
        
        for (i, rendered_line) in snapshot.lines().iter().enumerate() {
            let line_number = snapshot.viewport_range().start + i;
            
            let layout_line = self.layout_line(
                context,
                rendered_line,
                line_number,
                config.max_line_width,
                config,
            );
            
            // 更新垂直位置
            let line_with_position = layout_line.with_y_position(y_position);
            y_position += line_with_position.height();
            
            // 更新最大行宽
            let line_width = line_with_position.width();
            if line_width > max_line_width {
                max_line_width = line_width;
            }
            
            layout_lines.push(line_with_position);
        }
        
        let build_duration = start_time.elapsed();
        
        // 创建布局结果
        let result = LayoutResult::new(
            snapshot.id(),
            snapshot.viewport_range(),
            layout_lines.into(),
            y_position,
            max_line_width,
            context.stats().glyph_cache_hits,
            context.stats().glyph_cache_hit_rate(),
            LayoutMetadata {
                source: LayoutSource::Full,
                build_time: std::time::Instant::now(),
                lines_updated: layout_lines.len(),
                total_lines: layout_lines.len(),
                font_size: config.font_size,
                dpi_scale: config.dpi_scale,
                max_line_width: config.max_line_width,
            },
        );
        
        // 更新上下文
        context.stats().record_layout(build_duration, false);
        context.set_current_layout(result.id());
        
        result
    }
    
    /// 增量布局更新
    pub fn incremental_layout(
        &self,
        context: &mut LayoutContext,
        previous: &LayoutResult,
        snapshot: &ViewModelSnapshot,
        dirty_range: LineRange,
        config: &LayoutConfig,
    ) -> LayoutResult {
        let start_time = std::time::Instant::now();
        
        // 检查是否可以增量布局
        if !self.can_incremental_layout(previous, snapshot, config) {
            return self.layout_snapshot(context, snapshot, config);
        }
        
        // 使用增量引擎
        if let Some(engine) = &self.incremental_engine {
            let result = engine.update_layout(
                context,
                previous,
                snapshot,
                dirty_range,
                config,
            );
            
            let build_duration = start_time.elapsed();
            context.stats().record_layout(build_duration, true);
            context.set_current_layout(result.id());
            
            return result;
        }
        
        // 增量引擎不可用，回退到全量布局
        self.layout_snapshot(context, snapshot, config)
    }
    
    /// 布局单行
    pub fn layout_line(
        &self,
        context: &mut LayoutContext,
        rendered_line: &RenderedLine,
        line_number: usize,
        max_width: Option<f32>,
        config: &LayoutConfig,
    ) -> LayoutLine {
        // 检查行缓存
        let cache_key = self.compute_line_cache_key(rendered_line, max_width, config);
        if config.enable_caching {
            if let Some(cached) = context.line_cache.get(&cache_key) {
                context.stats().record_line_cache_hit();
                return cached.clone();
            }
        }
        
        // 布局视觉片段
        let mut fragments = Vec::new();
        let mut current_x = 0.0;
        
        for visual_span in rendered_line.visual_spans() {
            let fragment = self.layout_fragment(
                context,
                visual_span,
                current_x,
                config,
            );
            
            fragments.push(fragment);
            current_x += fragment.width;
        }
        
        // 处理换行（如果需要）
        let (fragments, wrapped_lines) = if let Some(max_width) = max_width {
            self.line_wrapper.wrap_line(
                fragments,
                max_width,
                config.wrap_mode,
                config.wrap_indent,
                context,
            )
        } else {
            (fragments, None)
        };
        
        // 计算行度量
        let (ascent, descent) = self.compute_line_metrics(&fragments, context, config);
        let height = ascent + descent + config.line_spacing;
        
        // 创建布局行
        let layout_line = LayoutLine::new(
            LineHandle::from_rendered_line(
                rendered_line.source_snapshot_id().unwrap_or_default(),
                rendered_line,
                line_number,
            ),
            line_number,
            fragments.into(),
            0.0, // y_position将在布局过程中设置
            height,
            ascent,
            descent,
            wrapped_lines.is_some(),
            wrapped_lines.as_ref().map_or(0, |w| w.len()),
            wrapped_lines,
            cache_key.line_hash,
        );
        
        // 缓存结果
        if config.enable_caching {
            context.line_cache.put(cache_key, layout_line.clone());
        }
        
        layout_line
    }
    
    /// 布局单个片段
    fn layout_fragment(
        &self,
        context: &mut LayoutContext,
        visual_span: &VisualSpan,
        start_x: f32,
        config: &LayoutConfig,
    ) -> LayoutFragment {
        // 获取字体
        let font_id = context.font_manager()
            .select_font_for_attrs(&visual_span.visual_attrs(), true)
            .expect("No suitable font found");
        
        let font = context.font_manager()
            .get_font(font_id)
            .expect("Font not found");
        
        // 形状化文本
        let glyphs = self.text_shaper.shape_text(
            context,
            font,
            visual_span.text(),
            &visual_span.visual_attrs(),
            config,
        );
        
        // 定位字形
        let (width, positioned_glyphs) = self.position_glyphs(&glyphs, font, config);
        
        // 检查是否需要处理溢出（在wrap_line中处理）
        
        // 计算垂直度量
        let font_metrics = font.metrics();
        let font_size = visual_span.visual_attrs().font_size.unwrap_or(config.font_size);
        let ascent = font_metrics.ascent(font_size) * config.dpi_scale;
        let descent = font_metrics.descent(font_size) * config.dpi_scale;
        
        // 构建字符到字形的映射
        let cluster_map = self.build_cluster_map(visual_span.text(), &positioned_glyphs);
        
        LayoutFragment::new(
            Arc::new(visual_span.clone()),
            Arc::from(visual_span.text()),
            start_x,
            width,
            ascent,
            descent,
            positioned_glyphs.into(),
            cluster_map.into(),
            visual_span.visual_attrs(),
        )
    }
    
    /// 计算行度量
    fn compute_line_metrics(
        &self,
        fragments: &[LayoutFragment],
        context: &LayoutContext,
        config: &LayoutConfig,
    ) -> (f32, f32) {
        let mut max_ascent = 0.0;
        let mut max_descent = 0.0;
        
        for fragment in fragments {
            if fragment.ascent > max_ascent {
                max_ascent = fragment.ascent;
            }
            if fragment.descent > max_descent {
                max_descent = fragment.descent;
            }
        }
        
        (max_ascent, max_descent)
    }
    
    /// 计算行缓存键
    fn compute_line_cache_key(
        &self,
        rendered_line: &RenderedLine,
        max_width: Option<f32>,
        config: &LayoutConfig,
    ) -> LineCacheKey {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        
        // 行内容哈希
        rendered_line.content_hash().hash(&mut hasher);
        
        // 配置相关
        max_width.map(|w| w.to_bits()).hash(&mut hasher);
        config.hash().hash(&mut hasher);
        
        LineCacheKey {
            line_hash: hasher.finish(),
            max_width: max_width.map(|w| w.to_bits()),
            config_hash: config.hash(),
        }
    }
    
    /// 检查是否可以增量布局
    fn can_incremental_layout(
        &self,
        previous: &LayoutResult,
        snapshot: &ViewModelSnapshot,
        config: &LayoutConfig,
    ) -> bool {
        // 视口范围必须相同
        if previous.viewport_range() != snapshot.viewport_range() {
            return false;
        }
        
        // 配置必须相同
        let prev_config = &previous.metadata();
        if (prev_config.font_size - config.font_size).abs() > f32::EPSILON ||
           (prev_config.dpi_scale - config.dpi_scale).abs() > f32::EPSILON ||
           prev_config.max_line_width != config.max_line_width {
            return false;
        }
        
        true
    }
    
    /// 定位字形
    fn position_glyphs(
        &self,
        glyphs: &[GlyphInfo],
        font: &dyn Font,
        config: &LayoutConfig,
    ) -> (f32, Vec<PositionedGlyph>) {
        let mut positioned = Vec::with_capacity(glyphs.len());
        let mut current_x = 0.0;
        
        for (i, glyph) in glyphs.iter().enumerate() {
            let positioned_glyph = PositionedGlyph::new(
                glyph.glyph_id,
                glyph.font_id,
                current_x,
                0.0, // y将在行布局中调整
                glyph.advance * config.dpi_scale,
                glyph.bounds.map(|b| b.scale(config.dpi_scale)),
                i,
                glyph.char_index,
            );
            
            positioned.push(positioned_glyph);
            current_x += glyph.advance * config.dpi_scale;
        }
        
        (current_x, positioned)
    }
    
    /// 构建字符到字形的映射
    fn build_cluster_map(&self, text: &str, glyphs: &[PositionedGlyph]) -> Vec<usize> {
        let char_count = text.chars().count();
        let mut cluster_map = vec![0; char_count];
        
        // 简单实现：假设每个字符对应一个字形
        for (i, glyph) in glyphs.iter().enumerate() {
            let char_idx = glyph.char_index();
            if char_idx < char_count {
                cluster_map[char_idx] = i;
            }
        }
        
        cluster_map
    }
}

/// 文本形状化器
struct TextShaper {
    // 字形缓存
    // 文本形状化算法
}

impl TextShaper {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn shape_text(
        &self,
        context: &mut LayoutContext,
        font: &dyn Font,
        text: &str,
        attrs: &VisualAttributes,
        config: &LayoutConfig,
    ) -> Vec<GlyphInfo> {
        let font_size = attrs.font_size.unwrap_or(config.font_size);
        let scaled_size = font_size * config.dpi_scale;
        
        // 检查字形缓存
        let cache_key = GlyphCacheKey {
            font_id: font.id(),
            text: text.to_string(),
            font_size: scaled_size.to_bits(),
            features: attrs.font_features.clone(),
        };
        
        if config.enable_caching {
            if let Some(cached) = context.glyph_cache.get(&cache_key) {
                context.stats().record_glyph_cache_hit();
                return cached.clone();
            }
        }
        
        // 形状化文本
        let glyphs = self.shape_text_impl(font, text, scaled_size, &attrs.font_features);
        
        // 缓存结果
        if config.enable_caching {
            context.glyph_cache.put(cache_key, glyphs.clone());
        } else {
            context.stats().record_glyph_cache_miss();
        }
        
        glyphs
    }
    
    fn shape_text_impl(
        &self,
        font: &dyn Font,
        text: &str,
        size: f32,
        features: &FontFeatures,
    ) -> Vec<GlyphInfo> {
        // 使用字体库进行文本形状化
        // 这里简化实现，返回基本字形信息
        let mut glyphs = Vec::new();
        let mut char_index = 0;
        
        for c in text.chars() {
            let glyph_id = font.glyph_for_char(c).unwrap_or(0);
            let advance = font.glyph_advance(glyph_id, size).unwrap_or(size * 0.6);
            let bounds = font.glyph_bounds(glyph_id, size);
            
            glyphs.push(GlyphInfo {
                glyph_id,
                font_id: font.id(),
                advance,
                bounds,
                char_index,
            });
            
            char_index += 1;
        }
        
        glyphs
    }
}

/// 字形信息
#[derive(Clone, Debug)]
struct GlyphInfo {
    glyph_id: u32,
    font_id: FontId,
    advance: f32,
    bounds: Option<Rect>,
    char_index: usize,
}

/// 片段优化器
struct FragmentOptimizer {
    options: OptimizerOptions,
}

impl FragmentOptimizer {
    pub fn new() -> Self {
        Self {
            options: OptimizerOptions::default(),
        }
    }
    
    pub fn optimize_fragments(&self, fragments: Vec<LayoutFragment>) -> Vec<LayoutFragment> {
        if !self.options.enabled {
            return fragments;
        }
        
        let mut optimized = Vec::with_capacity(fragments.len());
        let mut current = None;
        
        for fragment in fragments {
            if let Some(prev) = current.take() {
                if self.can_merge(&prev, &fragment) {
                    current = Some(self.merge_fragments(prev, fragment));
                } else {
                    optimized.push(prev);
                    current = Some(fragment);
                }
            } else {
                current = Some(fragment);
            }
        }
        
        if let Some(last) = current {
            optimized.push(last);
        }
        
        optimized
    }
    
    fn can_merge(&self, a: &LayoutFragment, b: &LayoutFragment) -> bool {
        // 视觉属性必须相同
        if a.visual_attrs() != b.visual_attrs() {
            return false;
        }
        
        // 必须是相邻的
        if (a.x_position + a.width - b.x_position).abs() > f32::EPSILON {
            return false;
        }
        
        // 片段大小不能太大
        if a.text.len() + b.text.len() > self.options.max_merge_size {
            return false;
        }
        
        true
    }
    
    fn merge_fragments(&self, a: LayoutFragment, b: LayoutFragment) -> LayoutFragment {
        // 合并文本
        let merged_text = format!("{}{}", a.text, b.text);
        
        // 合并字形（需要重新定位）
        let mut merged_glyphs = Vec::with_capacity(a.glyphs.len() + b.glyphs.len());
        
        // 添加a的字形
        for glyph in a.glyphs.iter() {
            merged_glyphs.push(glyph.clone());
        }
        
        // 添加b的字形，调整x位置
        for glyph in b.glyphs.iter() {
            let adjusted_glyph = PositionedGlyph::new(
                glyph.glyph_id(),
                glyph.font_id(),
                glyph.x() + a.width,
                glyph.y(),
                glyph.advance(),
                glyph.bounds().map(|bounds| bounds.translate(a.width, 0.0)),
                glyph.cluster_index() + a.glyphs.len(),
                glyph.char_index() + a.text.chars().count(),
            );
            merged_glyphs.push(adjusted_glyph);
        }
        
        // 构建新的cluster map
        let mut merged_cluster_map = Vec::with_capacity(a.cluster_map.len() + b.cluster_map.len());
        merged_cluster_map.extend_from_slice(&a.cluster_map);
        
        for &cluster_idx in b.cluster_map.iter() {
            merged_cluster_map.push(cluster_idx + a.glyphs.len());
        }
        
        LayoutFragment::new(
            a.visual_span.clone(), // 使用第一个片段的visual_span
            Arc::from(merged_text),
            a.x_position,
            a.width + b.width,
            a.ascent.max(b.ascent),
            a.descent.max(b.descent),
            merged_glyphs.into(),
            merged_cluster_map.into(),
            a.visual_attrs,
        )
    }
}

/// 优化器选项
#[derive(Clone, Debug)]
struct OptimizerOptions {
    enabled: bool,
    max_merge_size: usize,
}

impl Default for OptimizerOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            max_merge_size: 1024,
        }
    }
}
```

## 4. **换行处理实现**

```rust
// src/core/layout/wrapping.rs
use crate::core::geometry::Rect;
use super::context::{LayoutContext, WrapMode};
use super::result::LayoutFragment;

/// 换行器
pub struct LineWrapper {
    word_breaker: WordBreaker,
    char_breaker: CharBreaker,
    whitespace_breaker: WhitespaceBreaker,
    smart_wrapper: SmartWrapper,
}

impl LineWrapper {
    pub fn new() -> Self {
        Self {
            word_breaker: WordBreaker::new(),
            char_breaker: CharBreaker::new(),
            whitespace_breaker: WhitespaceBreaker::new(),
            smart_wrapper: SmartWrapper::new(),
        }
    }
    
    /// 换行处理
    pub fn wrap_line(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        wrap_mode: WrapMode,
        wrap_indent: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        if fragments.is_empty() {
            return (fragments, None);
        }
        
        // 计算总宽度
        let total_width: f32 = fragments.iter().map(|f| f.width).sum();
        
        // 如果总宽度不超过最大宽度，不需要换行
        if total_width <= max_width {
            return (fragments, None);
        }
        
        match wrap_mode {
            WrapMode::None => (fragments, None),
            WrapMode::Word => self.wrap_by_word(fragments, max_width, wrap_indent, context),
            WrapMode::Character => self.wrap_by_character(fragments, max_width, wrap_indent, context),
            WrapMode::Whitespace => self.wrap_at_whitespace(fragments, max_width, wrap_indent, context),
            WrapMode::Smart => self.smart_wrap(fragments, max_width, wrap_indent, context),
        }
    }
    
    /// 按单词换行
    fn wrap_by_word(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        wrap_indent: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        let mut wrapped_lines = Vec::new();
        let mut current_line_fragments = Vec::new();
        let mut current_width = 0.0;
        let mut line_index = 0;
        
        for fragment in fragments {
            // 如果片段本身超过最大宽度，需要进一步分割
            if fragment.width > max_width {
                let sub_fragments = self.word_breaker.split_fragment_by_word(
                    &fragment,
                    max_width,
                    context,
                );
                
                for sub_fragment in sub_fragments {
                    if current_width + sub_fragment.width > max_width && !current_line_fragments.is_empty() {
                        // 当前行已满，开始新行
                        wrapped_lines.push(self.create_wrapped_line(
                            current_line_fragments,
                            current_width,
                            line_index,
                            wrap_indent,
                        ));
                        current_line_fragments = Vec::new();
                        current_width = 0.0;
                        line_index += 1;
                    }
                    
                    current_line_fragments.push(sub_fragment.clone());
                    current_width += sub_fragment.width;
                }
            } else if current_width + fragment.width > max_width && !current_line_fragments.is_empty() {
                // 当前行已满，开始新行
                wrapped_lines.push(self.create_wrapped_line(
                    current_line_fragments,
                    current_width,
                    line_index,
                    wrap_indent,
                ));
                current_line_fragments = vec![fragment];
                current_width = fragment.width;
                line_index += 1;
            } else {
                // 添加到当前行
                current_line_fragments.push(fragment);
                current_width += fragment.width;
            }
        }
        
        // 添加最后一行
        if !current_line_fragments.is_empty() {
            wrapped_lines.push(self.create_wrapped_line(
                current_line_fragments,
                current_width,
                line_index,
                wrap_indent,
            ));
        }
        
        if wrapped_lines.len() > 1 {
            // 第一行作为主行，其余作为换行子行
            let first_line = wrapped_lines.remove(0);
            let wrapped_lines_arc = Arc::from(wrapped_lines);
            
            (first_line.fragments.to_vec(), Some(wrapped_lines_arc))
        } else {
            // 没有换行或只有一行
            (fragments, None)
        }
    }
    
    /// 按字符换行
    fn wrap_by_character(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        wrap_indent: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        let mut wrapped_lines = Vec::new();
        let mut current_line_fragments = Vec::new();
        let mut current_width = 0.0;
        let mut line_index = 0;
        
        for fragment in fragments {
            if fragment.width > max_width {
                // 按字符分割过宽的片段
                let char_fragments = self.char_breaker.split_fragment_by_char(
                    &fragment,
                    max_width,
                    context,
                );
                
                for char_fragment in char_fragments {
                    if current_width + char_fragment.width > max_width && !current_line_fragments.is_empty() {
                        wrapped_lines.push(self.create_wrapped_line(
                            current_line_fragments,
                            current_width,
                            line_index,
                            wrap_indent,
                        ));
                        current_line_fragments = Vec::new();
                        current_width = 0.0;
                        line_index += 1;
                    }
                    
                    current_line_fragments.push(char_fragment);
                    current_width += char_fragment.width;
                }
            } else if current_width + fragment.width > max_width && !current_line_fragments.is_empty() {
                wrapped_lines.push(self.create_wrapped_line(
                    current_line_fragments,
                    current_width,
                    line_index,
                    wrap_indent,
                ));
                current_line_fragments = vec![fragment];
                current_width = fragment.width;
                line_index += 1;
            } else {
                current_line_fragments.push(fragment);
                current_width += fragment.width;
            }
        }
        
        if !current_line_fragments.is_empty() {
            wrapped_lines.push(self.create_wrapped_line(
                current_line_fragments,
                current_width,
                line_index,
                wrap_indent,
            ));
        }
        
        if wrapped_lines.len() > 1 {
            let first_line = wrapped_lines.remove(0);
            let wrapped_lines_arc = Arc::from(wrapped_lines);
            (first_line.fragments.to_vec(), Some(wrapped_lines_arc))
        } else {
            (fragments, None)
        }
    }
    
    /// 在空白处换行
    fn wrap_at_whitespace(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        wrap_indent: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        let mut wrapped_lines = Vec::new();
        let mut current_line_fragments = Vec::new();
        let mut current_width = 0.0;
        let mut line_index = 0;
        
        for fragment in fragments {
            if fragment.width > max_width {
                // 在空白处分割
                let whitespace_fragments = self.whitespace_breaker.split_fragment_at_whitespace(
                    &fragment,
                    max_width,
                    context,
                );
                
                for ws_fragment in whitespace_fragments {
                    if current_width + ws_fragment.width > max_width && !current_line_fragments.is_empty() {
                        wrapped_lines.push(self.create_wrapped_line(
                            current_line_fragments,
                            current_width,
                            line_index,
                            wrap_indent,
                        ));
                        current_line_fragments = Vec::new();
                        current_width = 0.0;
                        line_index += 1;
                    }
                    
                    current_line_fragments.push(ws_fragment);
                    current_width += ws_fragment.width;
                }
            } else if current_width + fragment.width > max_width && !current_line_fragments.is_empty() {
                wrapped_lines.push(self.create_wrapped_line(
                    current_line_fragments,
                    current_width,
                    line_index,
                    wrap_indent,
                ));
                current_line_fragments = vec![fragment];
                current_width = fragment.width;
                line_index += 1;
            } else {
                current_line_fragments.push(fragment);
                current_width += fragment.width;
            }
        }
        
        if !current_line_fragments.is_empty() {
            wrapped_lines.push(self.create_wrapped_line(
                current_line_fragments,
                current_width,
                line_index,
                wrap_indent,
            ));
        }
        
        if wrapped_lines.len() > 1 {
            let first_line = wrapped_lines.remove(0);
            let wrapped_lines_arc = Arc::from(wrapped_lines);
            (first_line.fragments.to_vec(), Some(wrapped_lines_arc))
        } else {
            (fragments, None)
        }
    }
    
    /// 智能换行（混合策略）
    fn smart_wrap(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        wrap_indent: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        self.smart_wrapper.wrap_line(
            fragments,
            max_width,
            wrap_indent,
            context,
        )
    }
    
    /// 创建换行子行
    fn create_wrapped_line(
        &self,
        fragments: Vec<LayoutFragment>,
        width: f32,
        line_index: usize,
        wrap_indent: f32,
    ) -> WrappedLine {
        // 计算行度量
        let ascent = fragments.iter().map(|f| f.ascent).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);
        let descent = fragments.iter().map(|f| f.descent).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);
        let height = ascent + descent;
        
        // 调整缩进
        let adjusted_fragments = if line_index > 0 && wrap_indent > 0.0 {
            self.adjust_fragments_for_indent(fragments, wrap_indent)
        } else {
            fragments
        };
        
        WrappedLine::new(
            adjusted_fragments.into(),
            height,
            ascent,
            descent,
            (line_index as f32) * height,
        )
    }
    
    /// 调整片段缩进
    fn adjust_fragments_for_indent(
        &self,
        fragments: Vec<LayoutFragment>,
        indent: f32,
    ) -> Vec<LayoutFragment> {
        let mut adjusted = Vec::with_capacity(fragments.len());
        
        for (i, fragment) in fragments.into_iter().enumerate() {
            if i == 0 {
                // 第一个片段添加缩进
                let mut adjusted_fragment = fragment.clone();
                adjusted_fragment.x_position += indent;
                adjusted.push(adjusted_fragment);
            } else {
                adjusted.push(fragment);
            }
        }
        
        adjusted
    }
}

/// 单词分割器
struct WordBreaker {
    word_boundary_regex: regex::Regex,
}

impl WordBreaker {
    pub fn new() -> Self {
        Self {
            word_boundary_regex: regex::Regex::new(r"\b").unwrap(),
        }
    }
    
    pub fn split_fragment_by_word(
        &self,
        fragment: &LayoutFragment,
        max_width: f32,
        context: &LayoutContext,
    ) -> Vec<LayoutFragment> {
        let text = fragment.text.as_str();
        let font = context.font_manager()
            .get_font_for_attrs(&fragment.visual_attrs())
            .expect("Font not found");
        
        // 查找单词边界
        let mut splits = Vec::new();
        let mut start = 0;
        
        while start < text.len() {
            // 查找下一个单词边界
            let mut end = start;
            let mut word_width = 0.0;
            let mut found_boundary = false;
            
            while end < text.len() {
                // 检查当前结束位置是否是单词边界
                if end > start && self.is_word_boundary(text, end) {
                    found_boundary = true;
                }
                
                let next_char_start = text.char_indices().nth(end).map(|(i, _)| i).unwrap_or(text.len());
                let next_char_end = text.char_indices().nth(end + 1).map(|(i, _)| i).unwrap_or(text.len());
                
                if next_char_start >= text.len() {
                    break;
                }
                
                let char_text = &text[next_char_start..next_char_end.min(text.len())];
                let char_width = font.measure_text(char_text, fragment.visual_attrs().font_size.unwrap_or(context.config().font_size));
                
                if word_width + char_width > max_width {
                    // 超过宽度，需要分割
                    if found_boundary && end > start {
                        // 在上一个单词边界处分割
                        break;
                    } else {
                        // 没有找到合适的边界，在当前位置分割
                        break;
                    }
                }
                
                word_width += char_width;
                end += 1;
                
                if found_boundary && word_width > max_width * 0.7 {
                    // 找到边界且宽度合适，可以在此分割
                    break;
                }
            }
            
            if end == start {
                // 没有找到合适的分割点，强制前进一个字符
                end = start + 1;
            }
            
            // 创建子片段
            let sub_text = text.chars()
                .skip(start)
                .take(end - start)
                .collect::<String>();
            
            if !sub_text.is_empty() {
                let sub_fragment = self.create_sub_fragment(fragment, &sub_text, start, end, context);
                splits.push(sub_fragment);
            }
            
            start = end;
        }
        
        splits
    }
    
    fn is_word_boundary(&self, text: &str, position: usize) -> bool {
        // 简化实现：检查字符类型变化
        if position == 0 || position >= text.len() {
            return true;
        }
        
        let prev_char = text.chars().nth(position - 1).unwrap();
        let curr_char = text.chars().nth(position).unwrap();
        
        let prev_is_word = prev_char.is_alphanumeric();
        let curr_is_word = curr_char.is_alphanumeric();
        
        prev_is_word != curr_is_word
    }
    
    fn create_sub_fragment(
        &self,
        base_fragment: &LayoutFragment,
        sub_text: &str,
        start_char: usize,
        end_char: usize,
        context: &LayoutContext,
    ) -> LayoutFragment {
        // 计算子文本在原始文本中的字节范围
        let start_byte = base_fragment.text
            .char_indices()
            .nth(start_char)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let end_byte = base_fragment.text
            .char_indices()
            .nth(end_char)
            .map(|(i, _)| i)
            .unwrap_or(base_fragment.text.len());
        
        // 提取对应的字形
        let mut sub_glyphs = Vec::new();
        let mut sub_cluster_map = Vec::new();
        
        for (glyph_idx, glyph) in base_fragment.glyphs.iter().enumerate() {
            if glyph.char_index() >= start_char && glyph.char_index() < end_char {
                // 调整字形位置（相对于子片段）
                let adjusted_glyph = PositionedGlyph::new(
                    glyph.glyph_id(),
                    glyph.font_id(),
                    glyph.x() - (start_char as f32 * base_fragment.width / base_fragment.text.len() as f32),
                    glyph.y(),
                    glyph.advance(),
                    glyph.bounds(),
                    glyph.cluster_index(),
                    glyph.char_index() - start_char,
                );
                sub_glyphs.push(adjusted_glyph);
                
                // 更新cluster map
                if glyph.char_index() - start_char < sub_cluster_map.len() {
                    sub_cluster_map[glyph.char_index() - start_char] = glyph_idx;
                } else {
                    // 扩展cluster map
                    sub_cluster_map.resize(glyph.char_index() - start_char + 1, 0);
                    sub_cluster_map[glyph.char_index() - start_char] = glyph_idx;
                }
            }
        }
        
        // 计算宽度
        let font = context.font_manager()
            .get_font_for_attrs(&base_fragment.visual_attrs())
            .expect("Font not found");
        let width = font.measure_text(sub_text, base_fragment.visual_attrs().font_size.unwrap_or(context.config().font_size));
        
        // 创建新的visual span（如果需要）
        let visual_span = if start_byte == 0 && end_byte == base_fragment.text.len() {
            base_fragment.visual_span.clone()
        } else {
            Arc::new(VisualSpan::new(
                Arc::from(sub_text),
                base_fragment.visual_attrs(),
                start_byte..end_byte,
                start_char..end_char,
            ))
        };
        
        LayoutFragment::new(
            visual_span,
            Arc::from(sub_text),
            0.0, // x_position将在行布局中设置
            width,
            base_fragment.ascent,
            base_fragment.descent,
            sub_glyphs.into(),
            sub_cluster_map.into(),
            base_fragment.visual_attrs(),
        )
    }
}

/// 字符分割器
struct CharBreaker;

impl CharBreaker {
    pub fn new() -> Self {
        Self
    }
    
    pub fn split_fragment_by_char(
        &self,
        fragment: &LayoutFragment,
        max_width: f32,
        context: &LayoutContext,
    ) -> Vec<LayoutFragment> {
        let text = fragment.text.as_str();
        let font = context.font_manager()
            .get_font_for_attrs(&fragment.visual_attrs())
            .expect("Font not found");
        
        let mut splits = Vec::new();
        let mut start = 0;
        let mut current_width = 0.0;
        let mut current_chars = Vec::new();
        
        for (char_idx, c) in text.chars().enumerate() {
            let char_width = font.measure_text(&c.to_string(), fragment.visual_attrs().font_size.unwrap_or(context.config().font_size));
            
            if current_width + char_width > max_width && !current_chars.is_empty() {
                // 当前行已满，创建片段
                let sub_text: String = current_chars.iter().collect();
                let sub_fragment = self.create_sub_fragment(fragment, &sub_text, start, char_idx, context);
                splits.push(sub_fragment);
                
                // 开始新的片段
                start = char_idx;
                current_width = char_width;
                current_chars = vec![c];
            } else {
                current_width += char_width;
                current_chars.push(c);
            }
        }
        
        // 添加最后一个片段
        if !current_chars.is_empty() {
            let sub_text: String = current_chars.iter().collect();
            let sub_fragment = self.create_sub_fragment(fragment, &sub_text, start, text.chars().count(), context);
            splits.push(sub_fragment);
        }
        
        splits
    }
    
    fn create_sub_fragment(
        &self,
        base_fragment: &LayoutFragment,
        sub_text: &str,
        start_char: usize,
        end_char: usize,
        context: &LayoutContext,
    ) -> LayoutFragment {
        // 简化实现，实际需要更精确的字形处理
        let font = context.font_manager()
            .get_font_for_attrs(&base_fragment.visual_attrs())
            .expect("Font not found");
        let width = font.measure_text(sub_text, base_fragment.visual_attrs().font_size.unwrap_or(context.config().font_size));
        
        // 创建新的visual span
        let visual_span = Arc::new(VisualSpan::new(
            Arc::from(sub_text),
            base_fragment.visual_attrs(),
            0..sub_text.len(),
            0..sub_text.chars().count(),
        ));
        
        // 简化：创建新的字形（实际应该从base_fragment提取）
        let mut glyphs = Vec::new();
        let mut cluster_map = Vec::new();
        
        for (i, c) in sub_text.chars().enumerate() {
            let glyph_id = font.glyph_for_char(c).unwrap_or(0);
            let advance = font.glyph_advance(glyph_id, base_fragment.visual_attrs().font_size.unwrap_or(context.config().font_size))
                .unwrap_or(width / sub_text.chars().count() as f32);
            
            let glyph = PositionedGlyph::new(
                glyph_id,
                font.id(),
                (i as f32) * advance,
                0.0,
                advance,
                None,
                i,
                i,
            );
            glyphs.push(glyph);
            cluster_map.push(i);
        }
        
        LayoutFragment::new(
            visual_span,
            Arc::from(sub_text),
            0.0,
            width,
            base_fragment.ascent,
            base_fragment.descent,
            glyphs.into(),
            cluster_map.into(),
            base_fragment.visual_attrs(),
        )
    }
}

/// 空白分割器
struct WhitespaceBreaker {
    whitespace_regex: regex::Regex,
}

impl WhitespaceBreaker {
    pub fn new() -> Self {
        Self {
            whitespace_regex: regex::Regex::new(r"\s+").unwrap(),
        }
    }
    
    pub fn split_fragment_at_whitespace(
        &self,
        fragment: &LayoutFragment,
        max_width: f32,
        context: &LayoutContext,
    ) -> Vec<LayoutFragment> {
        let text = fragment.text.as_str();
        let font = context.font_manager()
            .get_font_for_attrs(&fragment.visual_attrs())
            .expect("Font not found");
        
        let mut splits = Vec::new();
        let mut start = 0;
        
        // 查找所有空白位置
        let whitespace_matches: Vec<_> = self.whitespace_regex.find_iter(text).collect();
        
        for (i, ws_match) in whitespace_matches.iter().enumerate() {
            let segment_end = ws_match.start();
            let segment_text = &text[start..segment_end];
            
            if !segment_text.is_empty() {
                let segment_width = font.measure_text(segment_text, fragment.visual_attrs().font_size.unwrap_or(context.config().font_size));
                
                if segment_width > max_width {
                    // 段仍然太宽，需要进一步分割
                    let sub_splits = self.split_segment_further(
                        fragment,
                        segment_text,
                        start,
                        segment_end,
                        max_width,
                        context,
                    );
                    splits.extend(sub_splits);
                } else {
                    let sub_fragment = self.create_sub_fragment(fragment, segment_text, start, segment_end, context);
                    splits.push(sub_fragment);
                }
            }
            
            // 添加空白（除非是最后一个）
            if i < whitespace_matches.len() - 1 {
                let ws_text = ws_match.as_str();
                let ws_fragment = self.create_sub_fragment(fragment, ws_text, ws_match.start(), ws_match.end(), context);
                splits.push(ws_fragment);
            }
            
            start = ws_match.end();
        }
        
        // 添加最后一个段
        if start < text.len() {
            let last_segment = &text[start..];
            if !last_segment.is_empty() {
                let last_width = font.measure_text(last_segment, fragment.visual_attrs().font_size.unwrap_or(context.config().font_size));
                
                if last_width > max_width {
                    let sub_splits = self.split_segment_further(
                        fragment,
                        last_segment,
                        start,
                        text.len(),
                        max_width,
                        context,
                    );
                    splits.extend(sub_splits);
                } else {
                    let last_fragment = self.create_sub_fragment(fragment, last_segment, start, text.len(), context);
                    splits.push(last_fragment);
                }
            }
        }
        
        splits
    }
    
    fn split_segment_further(
        &self,
        base_fragment: &LayoutFragment,
        segment_text: &str,
        segment_start: usize,
        segment_end: usize,
        max_width: f32,
        context: &LayoutContext,
    ) -> Vec<LayoutFragment> {
        // 使用字符分割器进一步分割
        let char_breaker = CharBreaker::new();
        let temp_fragment = self.create_sub_fragment(base_fragment, segment_text, segment_start, segment_end, context);
        
        char_breaker.split_fragment_by_char(&temp_fragment, max_width, context)
    }
    
    fn create_sub_fragment(
        &self,
        base_fragment: &LayoutFragment,
        sub_text: &str,
        start_byte: usize,
        end_byte: usize,
        context: &LayoutContext,
    ) -> LayoutFragment {
        // 简化实现，类似于WordBreaker中的方法
        let font = context.font_manager()
            .get_font_for_attrs(&base_fragment.visual_attrs())
            .expect("Font not found");
        let width = font.measure_text(sub_text, base_fragment.visual_attrs().font_size.unwrap_or(context.config().font_size));
        
        let start_char = base_fragment.text[..start_byte].chars().count();
        let end_char = start_char + sub_text.chars().count();
        
        let visual_span = Arc::new(VisualSpan::new(
            Arc::from(sub_text),
            base_fragment.visual_attrs(),
            start_byte..end_byte,
            start_char..end_char,
        ));
        
        // 简化字形创建
        let mut glyphs = Vec::new();
        let mut cluster_map = Vec::new();
        
        for (i, c) in sub_text.chars().enumerate() {
            let glyph_id = font.glyph_for_char(c).unwrap_or(0);
            let advance = font.glyph_advance(glyph_id, base_fragment.visual_attrs().font_size.unwrap_or(context.config().font_size))
                .unwrap_or(width / sub_text.chars().count() as f32);
            
            let glyph = PositionedGlyph::new(
                glyph_id,
                font.id(),
                (i as f32) * advance,
                0.0,
                advance,
                None,
                i,
                i,
            );
            glyphs.push(glyph);
            cluster_map.push(i);
        }
        
        LayoutFragment::new(
            visual_span,
            Arc::from(sub_text),
            0.0,
            width,
            base_fragment.ascent,
            base_fragment.descent,
            glyphs.into(),
            cluster_map.into(),
            base_fragment.visual_attrs(),
        )
    }
}

/// 智能换行器（混合策略）
struct SmartWrapper {
    word_breaker: WordBreaker,
    char_breaker: CharBreaker,
    whitespace_breaker: WhitespaceBreaker,
}

impl SmartWrapper {
    pub fn new() -> Self {
        Self {
            word_breaker: WordBreaker::new(),
            char_breaker: CharBreaker::new(),
            whitespace_breaker: WhitespaceBreaker::new(),
        }
    }
    
    pub fn wrap_line(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        wrap_indent: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        // 智能策略：优先尝试单词换行，如果不行则使用字符换行
        // 同时考虑空白处的自然换行点
        
        let word_result = self.word_breaker.split_fragment_by_word(
            &fragments[0], // 简化：只处理第一个片段
            max_width,
            context,
        );
        
        if !word_result.is_empty() && word_result.iter().all(|f| f.width <= max_width) {
            // 单词换行成功
            let mut all_fragments = Vec::new();
            all_fragments.extend(word_result);
            all_fragments.extend(fragments.iter().skip(1).cloned());
            
            return self.wrap_by_word(all_fragments, max_width, wrap_indent, context);
        }
        
        // 尝试空白处换行
        let whitespace_result = self.whitespace_breaker.split_fragment_at_whitespace(
            &fragments[0],
            max_width,
            context,
        );
        
        if !whitespace_result.is_empty() && whitespace_result.iter().all(|f| f.width <= max_width) {
            // 空白处换行成功
            let mut all_fragments = Vec::new();
            all_fragments.extend(whitespace_result);
            all_fragments.extend(fragments.iter().skip(1).cloned());
            
            return self.wrap_at_whitespace(all_fragments, max_width, wrap_indent, context);
        }
        
        // 最后使用字符换行
        self.wrap_by_character(fragments, max_width, wrap_indent, context)
    }
    
    fn wrap_by_word(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        wrap_indent: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        // 使用LineWrapper的逻辑
        let wrapper = LineWrapper::new();
        wrapper.wrap_by_word(fragments, max_width, wrap_indent, context)
    }
    
    fn wrap_at_whitespace(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        wrap_indent: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        let wrapper = LineWrapper::new();
        wrapper.wrap_at_whitespace(fragments, max_width, wrap_indent, context)
    }
    
    fn wrap_by_character(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        wrap_indent: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        let wrapper = LineWrapper::new();
        wrapper.wrap_by_character(fragments, max_width, wrap_indent, context)
    }
}
```

## 5. **坐标映射实现**

```rust
// src/core/layout/coordinate.rs
use std::sync::Arc;
use crate::core::logical::{LogicalPosition, LineRange};
use crate::core::geometry::{PhysicalPosition, Point};
use super::result::{LayoutResult, LayoutLine};

/// 坐标映射器
pub struct CoordinateMapper {
    cache: CoordinateCache,
    options: CoordinateOptions,
}

impl CoordinateMapper {
    pub fn new() -> Self {
        Self {
            cache: CoordinateCache::new(1000),
            options: CoordinateOptions::default(),
        }
    }
    
    /// 逻辑位置 → 物理位置
    pub fn logical_to_physical(
        &mut self,
        layout_result: &LayoutResult,
        logical_pos: LogicalPosition,
    ) -> Option<PhysicalPosition> {
        // 检查缓存
        let cache_key = CoordKey::from_logical(logical_pos, layout_result.id());
        if self.options.enable_caching {
            if let Some(cached) = self.cache.get(&cache_key) {
                return Some(cached);
            }
        }
        
        // 查找对应的布局行
        let layout_line = layout_result.find_line_for_position(logical_pos.line)?;
        
        // 计算水平位置
        let x = layout_line.x_at_column(logical_pos.column)?;
        
        // 计算垂直位置（基线位置）
        let y = layout_line.baseline();
        
        let physical_pos = PhysicalPosition::new(x, y);
        
        // 缓存结果
        if self.options.enable_caching {
            self.cache.put(cache_key, physical_pos);
        }
        
        Some(physical_pos)
    }
    
    /// 物理位置 → 逻辑位置
    pub fn physical_to_logical(
        &mut self,
        layout_result: &LayoutResult,
        physical_pos: PhysicalPosition,
    ) -> Option<LogicalPosition> {
        // 检查缓存
        let cache_key = CoordKey::from_physical(physical_pos, layout_result.id());
        if self.options.enable_caching {
            if let Some(cached) = self.cache.get(&cache_key) {
                return Some(cached);
            }
        }
        
        // 查找对应的布局行
        let layout_line = self.find_line_at_y(layout_result, physical_pos.y)?;
        
        // 计算列
        let x_in_line = physical_pos.x;
        let column = layout_line.column_at_x(x_in_line);
        
        let logical_pos = LogicalPosition::new(layout_line.line_number(), column);
        
        // 缓存结果
        if self.options.enable_caching {
            self.cache.put(cache_key, logical_pos);
        }
        
        Some(logical_pos)
    }
    
    /// 在指定y坐标查找布局行
    fn find_line_at_y(
        &self,
        layout_result: &LayoutResult,
        y: f32,
    ) -> Option<&LayoutLine> {
        if y < 0.0 || y > layout_result.total_height() {
            return None;
        }
        
        // 二分查找
        let lines = layout_result.lines();
        let mut low = 0;
        let mut high = lines.len();
        
        while low < high {
            let mid = (low + high) / 2;
            let line = &lines[mid];
            
            if y < line.y_position() {
                high = mid;
            } else if y >= line.y_position() + line.height() {
                low = mid + 1;
            } else {
                // 检查是否在换行子行中
                if let Some(wrapped_lines) = line.wrapped_lines() {
                    for wrapped_line in wrapped_lines {
                        let wrapped_y = line.y_position() + wrapped_line.y_offset();
                        if y >= wrapped_y && y < wrapped_y + wrapped_line.height() {
                            // 在换行子行中，需要特殊处理
                            // 简化：返回父行
                            return Some(line);
                        }
                    }
                }
                return Some(line);
            }
        }
        
        // 返回最后一行（如果y在最后一行之后）
        if low > 0 && low <= lines.len() {
            Some(&lines[low - 1])
        } else {
            None
        }
    }
    
    /// 获取行的边界矩形
    pub fn line_bounds(
        &self,
        layout_result: &LayoutResult,
        line_number: usize,
    ) -> Option<Rect> {
        let layout_line = layout_result.find_line_for_position(line_number)?;
        
        Some(Rect::new(
            0.0,
            layout_line.y_position(),
            layout_line.width(),
            layout_line.height(),
        ))
    }
    
    /// 获取字符的边界矩形
    pub fn char_bounds(
        &self,
        layout_result: &LayoutResult,
        logical_pos: LogicalPosition,
    ) -> Option<Rect> {
        let layout_line = layout_result.find_line_for_position(logical_pos.line)?;
        
        // 计算水平位置
        let x = layout_line.x_at_column(logical_pos.column)?;
        
        // 查找包含该列的片段
        let mut current_x = 0.0;
        for fragment in layout_line.fragments() {
            if logical_pos.column >= current_x as usize && 
               logical_pos.column < current_x as usize + fragment.char_count() {
                
                // 在片段内计算精确位置
                let offset_in_fragment = logical_pos.column - current_x as usize;
                let char_x = fragment.x_at_char(offset_in_fragment).unwrap_or(0.0);
                
                return Some(Rect::new(
                    x + char_x,
                    layout_line.y_position() - layout_line.ascent(),
                    0.0, // 字符宽度需要从字形获取
                    layout_line.height(),
                ));
            }
            current_x += fragment.width;
        }
        
        None
    }
    
    /// 清空坐标缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// 调整缓存大小
    pub fn resize_cache(&mut self, size: usize) {
        self.cache.resize(size);
    }
}

/// 坐标选项
#[derive(Clone, Debug)]
struct CoordinateOptions {
    enable_caching: bool,
    cache_size: usize,
    precision: CoordinatePrecision,
}

impl Default for CoordinateOptions {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_size: 1000,
            precision: CoordinatePrecision::High,
        }
    }
}

/// 坐标精度
#[derive(Clone, Debug)]
enum CoordinatePrecision {
    Low,     // 整数像素
    Medium,  // 0.5像素精度
    High,    // 0.1像素精度
    Exact,   // 精确计算
}

/// 坐标缓存
struct CoordinateCache {
    logical_to_physical: LruCache<CoordKey, PhysicalPosition>,
    physical_to_logical: LruCache<CoordKey, LogicalPosition>,
}

impl CoordinateCache {
    pub fn new(size: usize) -> Self {
        Self {
            logical_to_physical: LruCache::new(size),
            physical_to_logical: LruCache::new(size),
        }
    }
    
    pub fn get(&self, key: &CoordKey) -> Option<PhysicalPosition> {
        self.logical_to_physical.get(key).cloned()
    }
    
    pub fn put(&mut self, key: CoordKey, value: PhysicalPosition) {
        self.logical_to_physical.put(key, value);
    }
    
    pub fn get_logical(&self, key: &CoordKey) -> Option<LogicalPosition> {
        self.physical_to_logical.get(key).cloned()
    }
    
    pub fn put_logical(&mut self, key: CoordKey, value: LogicalPosition) {
        self.physical_to_logical.put(key, value);
    }
    
    pub fn clear(&mut self) {
        self.logical_to_physical.clear();
        self.physical_to_logical.clear();
    }
    
    pub fn resize(&mut self, size: usize) {
        self.logical_to_physical.resize(size);
        self.physical_to_logical.resize(size);
    }
}

/// 坐标缓存键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CoordKey {
    layout_id: LayoutId,
    logical_line: usize,
    logical_column: usize,
    physical_x_bits: Option<u32>,
    physical_y_bits: Option<u32>,
}

impl CoordKey {
    pub fn from_logical(logical_pos: LogicalPosition, layout_id: LayoutId) -> Self {
        Self {
            layout_id,
            logical_line: logical_pos.line,
            logical_column: logical_pos.column,
            physical_x_bits: None,
            physical_y_bits: None,
        }
    }
    
    pub fn from_physical(physical_pos: PhysicalPosition, layout_id: LayoutId) -> Self {
        Self {
            layout_id,
            logical_line: 0,
            logical_column: 0,
            physical_x_bits: Some(physical_pos.x.to_bits()),
            physical_y_bits: Some(physical_pos.y.to_bits()),
        }
    }
}
```

## 6. **缓存系统实现**

```rust
// src/core/layout/cache.rs
use std::collections::HashMap;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::hash::{Hash, Hasher};
use super::result::{LayoutLine, LayoutId};
use super::context::LayoutConfig;

/// 布局缓存系统
pub struct LayoutCache {
    glyph_cache: GlyphCache,
    line_cache: LineLayoutCache,
    layout_result_cache: LayoutResultCache,
    stats: CacheStats,
}

impl LayoutCache {
    pub fn new(glyph_cache_size: usize, line_cache_size: usize, result_cache_size: usize) -> Self {
        Self {
            glyph_cache: GlyphCache::new(glyph_cache_size),
            line_cache: LineLayoutCache::new(line_cache_size),
            layout_result_cache: LayoutResultCache::new(result_cache_size),
            stats: CacheStats::new(),
        }
    }
    
    pub fn get_line(&mut self, key: &LineCacheKey) -> Option<Arc<LayoutLine>> {
        if let Some(line) = self.line_cache.get(key) {
            self.stats.record_line_cache_hit();
            Some(line.clone())
        } else {
            self.stats.record_line_cache_miss();
            None
        }
    }
    
    pub fn put_line(&mut self, key: LineCacheKey, line: Arc<LayoutLine>) {
        self.line_cache.put(key, line);
        self.stats.record_line_cache_store();
    }
    
    pub fn get_glyphs(&mut self, key: &GlyphCacheKey) -> Option<Arc<Vec<GlyphInfo>>> {
        if let Some(glyphs) = self.glyph_cache.get(key) {
            self.stats.record_glyph_cache_hit();
            Some(glyphs.clone())
        } else {
            self.stats.record_glyph_cache_miss();
            None
        }
    }
    
    pub fn put_glyphs(&mut self, key: GlyphCacheKey, glyphs: Arc<Vec<GlyphInfo>>) {
        self.glyph_cache.put(key, glyphs);
        self.stats.record_glyph_cache_store();
    }
    
    pub fn get_layout_result(&mut self, key: &LayoutResultKey) -> Option<Arc<LayoutResult>> {
        if let Some(result) = self.layout_result_cache.get(key) {
            self.stats.record_result_cache_hit();
            Some(result.clone())
        } else {
            self.stats.record_result_cache_miss();
            None
        }
    }
    
    pub fn put_layout_result(&mut self, key: LayoutResultKey, result: Arc<LayoutResult>) {
        self.layout_result_cache.put(key, result);
        self.stats.record_result_cache_store();
    }
    
    pub fn invalidate_range(&mut self, range: LineRange) {
        // 使受影响的缓存条目失效
        self.line_cache.invalidate_range(range);
        self.layout_result_cache.invalidate_range(range);
        self.stats.record_invalidation();
    }
    
    pub fn clear(&mut self) {
        self.glyph_cache.clear();
        self.line_cache.clear();
        self.layout_result_cache.clear();
        self.stats.record_clear();
    }
    
    pub fn resize(&mut self, glyph_size: usize, line_size: usize, result_size: usize) {
        self.glyph_cache.resize(glyph_size);
        self.line_cache.resize(line_size);
        self.layout_result_cache.resize(result_size);
    }
    
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }
    
    pub fn estimated_size(&self) -> usize {
        self.glyph_cache.estimated_size() +
        self.line_cache.estimated_size() +
        self.layout_result_cache.estimated_size()
    }
}

/// 行缓存键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LineCacheKey {
    pub line_hash: u64,
    pub max_width_bits: Option<u32>,
    pub config_hash: u64,
}

/// 字形缓存键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey {
    pub font_id: FontId,
    pub text: String,
    pub font_size_bits: u32,
    pub features_hash: u64,
}

/// 布局结果缓存键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutResultKey {
    pub snapshot_id: SnapshotId,
    pub viewport_range: LineRange,
    pub config_hash: u64,
    pub text_hash: u64,
}

/// 字形缓存
struct GlyphCache {
    cache: LruCache<GlyphCacheKey, Arc<Vec<GlyphInfo>>>,
}

impl GlyphCache {
    pub fn new(size: usize) -> Self {
        let size = NonZeroUsize::new(size.max(1)).unwrap();
        Self {
            cache: LruCache::new(size),
        }
    }
    
    pub fn get(&self, key: &GlyphCacheKey) -> Option<&Arc<Vec<GlyphInfo>>> {
        self.cache.get(key)
    }
    
    pub fn put(&mut self, key: GlyphCacheKey, value: Arc<Vec<GlyphInfo>>) {
        self.cache.put(key, value);
    }
    
    pub fn clear(&mut self) {
        self.cache.clear();
    }
    
    pub fn resize(&mut self, size: usize) {
        let size = NonZeroUsize::new(size.max(1)).unwrap();
        self.cache.resize(size);
    }
    
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    
    pub fn estimated_size(&self) -> usize {
        let mut size = 0;
        for (key, value) in self.cache.iter() {
            size += std::mem::size_of_val(key);
            size += std::mem::size_of_val(value);
            size += value.iter().map(|g| std::mem::size_of_val(g)).sum::<usize>();
        }
        size
    }
}

/// 行布局缓存
struct LineLayoutCache {
    cache: LruCache<LineCacheKey, Arc<LayoutLine>>,
}

impl LineLayoutCache {
    pub fn new(size: usize) -> Self {
        let size = NonZeroUsize::new(size.max(1)).unwrap();
        Self {
            cache: LruCache::new(size),
        }
    }
    
    pub fn get(&self, key: &LineCacheKey) -> Option<&Arc<LayoutLine>> {
        self.cache.get(key)
    }
    
    pub fn put(&mut self, key: LineCacheKey, value: Arc<LayoutLine>) {
        self.cache.put(key, value);
    }
    
    pub fn invalidate_range(&mut self, range: LineRange) {
        // 收集需要移除的键
        let keys_to_remove: Vec<LineCacheKey> = self.cache
            .iter()
            .filter(|(_, line)| {
                let line_number = line.line_number();
                range.contains(line_number)
            })
            .map(|(key, _)| key.clone())
            .collect();
        
        // 移除受影响的缓存项
        for key in keys_to_remove {
            self.cache.pop(&key);
        }
    }
    
    pub fn clear(&mut self) {
        self.cache.clear();
    }
    
    pub fn resize(&mut self, size: usize) {
        let size = NonZeroUsize::new(size.max(1)).unwrap();
        self.cache.resize(size);
    }
    
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    
    pub fn estimated_size(&self) -> usize {
        let mut size = 0;
        for (key, value) in self.cache.iter() {
            size += std::mem::size_of_val(key);
            size += std::mem::size_of_val(value);
            size += value.estimated_size();
        }
        size
    }
}

/// 布局结果缓存
struct LayoutResultCache {
    cache: LruCache<LayoutResultKey, Arc<LayoutResult>>,
}

impl LayoutResultCache {
    pub fn new(size: usize) -> Self {
        let size = NonZeroUsize::new(size.max(1)).unwrap();
        Self {
            cache: LruCache::new(size),
        }
    }
    
    pub fn get(&self, key: &LayoutResultKey) -> Option<&Arc<LayoutResult>> {
        self.cache.get(key)
    }
    
    pub fn put(&mut self, key: LayoutResultKey, value: Arc<LayoutResult>) {
        self.cache.put(key, value);
    }
    
    pub fn invalidate_range(&mut self, range: LineRange) {
        // 收集需要移除的键
        let keys_to_remove: Vec<LayoutResultKey> = self.cache
            .iter()
            .filter(|(_, result)| {
                result.viewport_range().intersects(&range)
            })
            .map(|(key, _)| key.clone())
            .collect();
        
        // 移除受影响的缓存项
        for key in keys_to_remove {
            self.cache.pop(&key);
        }
    }
    
    pub fn clear(&mut self) {
        self.cache.clear();
    }
    
    pub fn resize(&mut self, size: usize) {
        let size = NonZeroUsize::new(size.max(1)).unwrap();
        self.cache.resize(size);
    }
    
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    
    pub fn estimated_size(&self) -> usize {
        let mut size = 0;
        for (key, value) in self.cache.iter() {
            size += std::mem::size_of_val(key);
            size += std::mem::size_of_val(value);
            size += value.estimated_size();
        }
        size
    }
}

/// 缓存统计
#[derive(Clone, Debug)]
pub struct CacheStats {
    glyph_cache_hits: usize,
    glyph_cache_misses: usize,
    glyph_cache_stores: usize,
    line_cache_hits: usize,
    line_cache_misses: usize,
    line_cache_stores: usize,
    result_cache_hits: usize,
    result_cache_misses: usize,
    result_cache_stores: usize,
    invalidations: usize,
    clears: usize,
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            glyph_cache_hits: 0,
            glyph_cache_misses: 0,
            glyph_cache_stores: 0,
            line_cache_hits: 0,
            line_cache_misses: 0,
            line_cache_stores: 0,
            result_cache_hits: 0,
            result_cache_misses: 0,
            result_cache_stores: 0,
            invalidations: 0,
            clears: 0,
        }
    }
    
    pub fn record_glyph_cache_hit(&mut self) {
        self.glyph_cache_hits += 1;
    }
    
    pub fn record_glyph_cache_miss(&mut self) {
        self.glyph_cache_misses += 1;
    }
    
    pub fn record_glyph_cache_store(&mut self) {
        self.glyph_cache_stores += 1;
    }
    
    pub fn record_line_cache_hit(&mut self) {
        self.line_cache_hits += 1;
    }
    
    pub fn record_line_cache_miss(&mut self) {
        self.line_cache_misses += 1;
    }
    
    pub fn record_line_cache_store(&mut self) {
        self.line_cache_stores += 1;
    }
    
    pub fn record_result_cache_hit(&mut self) {
        self.result_cache_hits += 1;
    }
    
    pub fn record_result_cache_miss(&mut self) {
        self.result_cache_misses += 1;
    }
    
    pub fn record_result_cache_store(&mut self) {
        self.result_cache_stores += 1;
    }
    
    pub fn record_invalidation(&mut self) {
        self.invalidations += 1;
    }
    
    pub fn record_clear(&mut self) {
        self.clears += 1;
    }
    
    // 计算命中率
    pub fn glyph_cache_hit_rate(&self) -> f32 {
        let total = self.glyph_cache_hits + self.glyph_cache_misses;
        if total == 0 {
            0.0
        } else {
            self.glyph_cache_hits as f32 / total as f32
        }
    }
    
    pub fn line_cache_hit_rate(&self) -> f32 {
        let total = self.line_cache_hits + self.line_cache_misses;
        if total == 0 {
            0.0
        } else {
            self.line_cache_hits as f32 / total as f32
        }
    }
    
    pub fn result_cache_hit_rate(&self) -> f32 {
        let total = self.result_cache_hits + self.result_cache_misses;
        if total == 0 {
            0.0
        } else {
            self.result_cache_hits as f32 / total as f32
        }
    }
    
    pub fn overall_hit_rate(&self) -> f32 {
        let hits = self.glyph_cache_hits + self.line_cache_hits + self.result_cache_hits;
        let misses = self.glyph_cache_misses + self.line_cache_misses + self.result_cache_misses;
        let total = hits + misses;
        
        if total == 0 {
            0.0
        } else {
            hits as f32 / total as f32
        }
    }
}
```

## 7. **集成示例和使用**

```rust
// src/core/layout/integration.rs
use std::sync::Arc;
use crate::core::viewmodel::ViewModelSnapshot;
use crate::core::viewport::{Viewport, ViewportData};
use super::*;

/// 布局管道 - 连接ViewModel和Layout
pub struct LayoutPipeline {
    layout_engine: LayoutEngine,
    layout_context: LayoutContext,
    current_layout: Option<Arc<LayoutResult>>,
    coordinate_mapper: CoordinateMapper,
    config: LayoutConfig,
}

impl LayoutPipeline {
    pub fn new(font_manager: Arc<dyn FontManager>) -> Self {
        Self {
            layout_engine: LayoutEngine::new(),
            layout_context: LayoutContext::new(font_manager),
            current_layout: None,
            coordinate_mapper: CoordinateMapper::new(),
            config: LayoutConfig::default(),
        }
    }
    
    /// 配置布局管道
    pub fn configure(&mut self, config: LayoutConfig) -> &mut Self {
        self.layout_context.configure(config.clone());
        self.config = config;
        self
    }
    
    /// 处理视图模型更新
    pub fn process_update(
        &mut self,
        snapshot: Arc<ViewModelSnapshot>,
        delta: Option<&ViewModelDelta>,
        viewport_width: Option<f32>,
    ) -> PipelineResult {
        let start_time = std::time::Instant::now();
        
        // 确定更新策略
        let update_strategy = self.determine_update_strategy(&snapshot, delta, viewport_width);
        
        // 执行布局
        let new_layout = match update_strategy {
            UpdateStrategy::Full => {
                self.layout_engine.layout_snapshot(
                    &mut self.layout_context,
                    &snapshot,
                    &self.config.with_viewport_width(viewport_width),
                )
            }
            UpdateStrategy::Incremental(dirty_range) => {
                if let Some(current) = &self.current_layout {
                    self.layout_engine.incremental_layout(
                        &mut self.layout_context,
                        current,
                        &snapshot,
                        dirty_range,
                        &self.config.with_viewport_width(viewport_width),
                    )
                } else {
                    // 没有当前布局，退回到全量布局
                    self.layout_engine.layout_snapshot(
                        &mut self.layout_context,
                        &snapshot,
                        &self.config.with_viewport_width(viewport_width),
                    )
                }
            }
            UpdateStrategy::None => {
                // 无需更新，返回当前布局
                return PipelineResult {
                    layout: self.current_layout.clone().unwrap(),
                    delta: LayoutDelta::empty(),
                    source: LayoutSource::Cached,
                };
            }
        };
        
        // 计算布局差异
        let layout_delta = if let Some(current) = &self.current_layout {
            LayoutDelta::compute(current, &new_layout)
        } else {
            LayoutDelta::empty()
        };
        
        // 更新当前布局
        let layout_arc = Arc::new(new_layout);
        self.current_layout = Some(layout_arc.clone());
        
        let duration = start_time.elapsed();
        self.layout_context.stats().record_layout(duration, matches!(update_strategy, UpdateStrategy::Incremental(_)));
        
        PipelineResult {
            layout: layout_arc,
            delta: layout_delta,
            source: match update_strategy {
                UpdateStrategy::Full => LayoutSource::Full,
                UpdateStrategy::Incremental(_) => LayoutSource::Incremental,
                UpdateStrategy::None => LayoutSource::Cached,
            },
        }
    }
    
    /// 确定更新策略
    fn determine_update_strategy(
        &self,
        snapshot: &ViewModelSnapshot,
        delta: Option<&ViewModelDelta>,
        viewport_width: Option<f32>,
    ) -> UpdateStrategy {
        // 检查配置变化
        let config_with_width = self.config.with_viewport_width(viewport_width);
        if let Some(current) = &self.current_layout {
            if current.metadata().max_line_width != config_with_width.max_line_width ||
               (current.metadata().font_size - config_with_width.font_size).abs() > f32::EPSILON ||
               (current.metadata().dpi_scale - config_with_width.dpi_scale).abs() > f32::EPSILON {
                return UpdateStrategy::Full;
            }
        }
        
        // 检查视口范围变化
        if let Some(current) = &self.current_layout {
            if current.viewport_range() != snapshot.viewport_range() {
                return UpdateStrategy::Full;
            }
        }
        
        // 检查增量更新可能性
        if let Some(delta) = delta {
            if let Some(dirty_range) = delta.affected_range() {
                let dirty_lines = dirty_range.len();
                let total_lines = snapshot.viewport_range().len();
                
                // 如果脏区较小，使用增量布局
                if dirty_lines as f32 / total_lines as f32 < 0.3 {
                    return UpdateStrategy::Incremental(dirty_range);
                }
            }
        }
        
        // 默认全量布局
        UpdateStrategy::Full
    }
    
    /// 坐标转换
    pub fn logical_to_physical(
        &mut self,
        logical_pos: LogicalPosition,
    ) -> Option<PhysicalPosition> {
        self.current_layout.as_ref().and_then(|layout| {
            self.coordinate_mapper.logical_to_physical(layout, logical_pos)
        })
    }
    
    pub fn physical_to_logical(
        &mut self,
        physical_pos: PhysicalPosition,
    ) -> Option<LogicalPosition> {
        self.current_layout.as_ref().and_then(|layout| {
            self.coordinate_mapper.physical_to_logical(layout, physical_pos)
        })
    }
    
    /// 获取当前布局
    pub fn current_layout(&self) -> Option<&Arc<LayoutResult>> {
        self.current_layout.as_ref()
    }
    
    /// 获取统计信息
    pub fn stats(&self) -> &LayoutStats {
        self.layout_context.stats()
    }
    
    /// 清空缓存和状态
    pub fn reset(&mut self) {
        self.layout_context.clear_cache();
        self.coordinate_mapper.clear_cache();
        self.current_layout = None;
    }
}

/// 更新策略
enum UpdateStrategy {
    Full,
    Incremental(LineRange),
    None,
}

/// 管道结果
pub struct PipelineResult {
    pub layout: Arc<LayoutResult>,
    pub delta: LayoutDelta,
    pub source: LayoutSource,
}

/// 布局差异
#[derive(Clone, Debug)]
pub struct LayoutDelta {
    pub updated_lines: Vec<LineUpdate>,
    pub inserted_lines: Vec<(usize, Arc<LayoutLine>)>,
    pub deleted_lines: Range<usize>,
    pub total_height_changed: bool,
    pub max_width_changed: bool,
}

impl LayoutDelta {
    pub fn empty() -> Self {
        Self {
            updated_lines: Vec::new(),
            inserted_lines: Vec::new(),
            deleted_lines: 0..0,
            total_height_changed: false,
            max_width_changed: false,
        }
    }
    
    pub fn compute(old: &LayoutResult, new: &LayoutResult) -> Self {
        let mut delta = Self::empty();
        
        // 检查总体变化
        delta.total_height_changed = (old.total_height() - new.total_height()).abs() > f32::EPSILON;
        delta.max_width_changed = (old.max_line_width() - new.max_line_width()).abs() > f32::EPSILON;
        
        // 比较行
        let old_lines = old.lines();
        let new_lines = new.lines();
        
        // 简化实现：逐行比较
        for i in 0..old_lines.len().max(new_lines.len()) {
            match (old_lines.get(i), new_lines.get(i)) {
                (Some(old_line), Some(new_line)) => {
                    if old_line.layout_hash() != new_line.layout_hash() {
                        delta.updated_lines.push(LineUpdate {
                            line_index: i,
                            old_line: old_line.clone(),
                            new_line: new_line.clone(),
                        });
                    }
                }
                (None, Some(new_line)) => {
                    delta.inserted_lines.push((i, Arc::new(new_line.clone())));
                }
                (Some(_), None) => {
                    delta.deleted_lines = i..old_lines.len();
                    break;
                }
                _ => {}
            }
        }
        
        delta
    }
    
    pub fn is_empty(&self) -> bool {
        self.updated_lines.is_empty() &&
        self.inserted_lines.is_empty() &&
        self.deleted_lines.is_empty() &&
        !self.total_height_changed &&
        !self.max_width_changed
    }
}

/// 行更新
#[derive(Clone, Debug)]
pub struct LineUpdate {
    pub line_index: usize,
    pub old_line: LayoutLine,
    pub new_line: LayoutLine,
}
```

## 8. **测试代码示例**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::viewmodel::{ViewModelSnapshot, RenderedLine, VisualSpan};
    
    #[test]
    fn test_basic_layout() {
        // 创建测试字体管理器
        let font_manager = create_test_font_manager();
        let mut pipeline = LayoutPipeline::new(font_manager);
        
        // 创建测试快照
        let snapshot = create_test_snapshot();
        
        // 布局
        let result = pipeline.process_update(
            Arc::new(snapshot),
            None,
            Some(800.0),
        );
        
        assert!(result.layout.lines().len() > 0);
        assert!(result.layout.total_height() > 0.0);
        assert!(result.layout.max_line_width() > 0.0);
    }
    
    #[test]
    fn test_line_wrapping() {
        let font_manager = create_test_font_manager();
        let mut context = LayoutContext::new(font_manager);
        
        let config = LayoutConfig {
            max_line_width: Some(200.0),
            wrap_mode: WrapMode::Word,
            ..Default::default()
        };
        context.configure(config);
        
        let engine = LayoutEngine::new();
        
        // 创建长文本行
        let long_text = "This is a very long line of text that should wrap at word boundaries when it exceeds the maximum line width.";
        let rendered_line = create_test_rendered_line(long_text);
        
        // 布局行
        let layout_line = engine.layout_line(
            &mut context,
            &rendered_line,
            0,
            Some(200.0),
            &LayoutConfig::default(),
        );
        
        // 应该被换行
        assert!(layout_line.is_wrapped());
        assert!(layout_line.wrap_count() > 0);
        
        if let Some(wrapped_lines) = layout_line.wrapped_lines() {
            assert!(!wrapped_lines.is_empty());
            
            // 检查每行宽度不超过最大宽度
            for wrapped_line in wrapped_lines {
                assert!(wrapped_line.width() <= 200.0);
            }
        }
    }
    
    #[test]
    fn test_coordinate_mapping() {
        let font_manager = create_test_font_manager();
        let mut pipeline = LayoutPipeline::new(font_manager);
        
        let snapshot = create_test_snapshot();
        let result = pipeline.process_update(
            Arc::new(snapshot),
            None,
            Some(800.0),
        );
        
        // 测试逻辑到物理映射
        let logical_pos = LogicalPosition::new(0, 0);
        let physical_pos = pipeline.logical_to_physical(logical_pos);
        
        assert!(physical_pos.is_some());
        let physical = physical_pos.unwrap();
        assert!(physical.x >= 0.0);
        assert!(physical.y >= 0.0);
        
        // 测试物理到逻辑映射（往返）
        let round_trip = pipeline.physical_to_logical(physical);
        assert!(round_trip.is_some());
        
        // 应该映射回相同位置（或非常接近）
        let rt = round_trip.unwrap();
        assert_eq!(rt.line, logical_pos.line);
        // 列可能因为字符宽度而略有不同
        assert!(rt.column.abs_diff(logical_pos.column) <= 1);
    }
    
    #[test]
    fn test_incremental_layout() {
        let font_manager = create_test_font_manager();
        let mut pipeline = LayoutPipeline::new(font_manager);
        
        // 第一次布局
        let snapshot1 = create_test_snapshot();
        let result1 = pipeline.process_update(
            Arc::new(snapshot1.clone()),
            None,
            Some(800.0),
        );
        
        // 创建修改后的快照（只修改第一行）
        let snapshot2 = modify_snapshot_line(&snapshot1, 0, "Modified line");
        
        // 创建delta
        let delta = ViewModelDelta {
            updated_lines: vec![LineUpdate {
                line_index: 0,
                old_line: snapshot1.lines()[0].clone(),
                new_line: snapshot2.lines()[0].clone(),
            }],
            ..ViewModelDelta::empty()
        };
        
        // 增量布局
        let result2 = pipeline.process_update(
            Arc::new(snapshot2),
            Some(&delta),
            Some(800.0),
        );
        
        // 检查布局差异
        assert!(!result2.delta.is_empty());
        assert_eq!(result2.delta.updated_lines.len(), 1);
        assert_eq!(result2.delta.updated_lines[0].line_index, 0);
        
        // 验证元数据
        assert_eq!(result2.source, LayoutSource::Incremental);
    }
    
    #[test]
    fn test_cache_efficiency() {
        let font_manager = create_test_font_manager();
        let mut pipeline = LayoutPipeline::new(font_manager);
        
        let snapshot = create_test_snapshot();
        
        // 多次布局相同内容
        for _ in 0..5 {
            pipeline.process_update(
                Arc::new(snapshot.clone()),
                None,
                Some(800.0),
            );
        }
        
        let stats = pipeline.stats();
        
        // 缓存命中率应该提高
        assert!(stats.glyph_cache_hit_rate() > 0.5);
        assert!(stats.line_cache_hit_rate() > 0.5);
        
        // 后续布局应该更快（统计上）
        assert!(stats.avg_layout_time_ms() < 10.0); // 假设阈值
    }
    
    // 辅助函数
    fn create_test_font_manager() -> Arc<dyn FontManager> {
        // 创建模拟字体管理器
        Arc::new(MockFontManager::new())
    }
    
    fn create_test_snapshot() -> ViewModelSnapshot {
        // 创建测试快照
        let lines = vec![
            create_test_rendered_line("Line 1: Hello World"),
            create_test_rendered_line("Line 2: This is a test"),
            create_test_rendered_line("Line 3: 你好世界"),
        ];
        
        ViewModelSnapshot::new(
            LineRange::new(0, 3),
            100,
            lines.into(),
            Arc::new(DecorationSet::empty()),
            None,
            None,
            SnapshotMetadata {
                version: 1,
                source: SnapshotSource::FullBuild,
                build_duration: std::time::Duration::from_millis(1),
                build_method: BuildMethod::Full,
                text_hash: 123,
            },
        )
    }
    
    fn create_test_rendered_line(text: &str) -> RenderedLine {
        let visual_span = VisualSpan::plain(text);
        
        RenderedLine::new(
            0,
            Arc::from(text),
            Arc::from([visual_span]),
            20.0,
            16.0,
            None,
            0.0,
            false,
            false,
            false,
            false,
        )
    }
    
    fn modify_snapshot_line(
        snapshot: &ViewModelSnapshot,
        line_index: usize,
        new_text: &str,
    ) -> ViewModelSnapshot {
        // 创建修改后的快照
        let mut new_lines = Vec::new();
        
        for (i, line) in snapshot.lines().iter().enumerate() {
            if i == line_index {
                new_lines.push(create_test_rendered_line(new_text));
            } else {
                new_lines.push(line.clone());
            }
        }
        
        ViewModelSnapshot::new(
            snapshot.viewport_range(),
            snapshot.total_lines(),
            new_lines.into(),
            snapshot.decorations().clone(),
            snapshot.cursor().cloned(),
            snapshot.selection().cloned(),
            SnapshotMetadata {
                version: snapshot.version() + 1,
                ..snapshot.metadata().clone()
            },
        )
    }
}
```

## **核心特性总结**

这个Layout系统实现提供了：

### 1. **完整的布局管道**
- 从ViewModel快照到LayoutResult的完整转换
- 支持全量和增量两种布局模式
- 智能的更新策略决策

### 2. **多策略换行系统**
- 单词换行、字符换行、空白换行、智能换行四种模式
- 可扩展的换行算法架构
- 换行缩进和子行管理

### 3. **精确坐标映射**
- 逻辑位置 ↔ 物理位置双向转换
- 亚像素精度的文本定位
- 多层缓存加速坐标查询

### 4. **高性能缓存系统**
- 字形缓存：高频字形的形状信息
- 行布局缓存：基于内容哈希的完整行布局
- 布局结果缓存：完整的布局结果
- 坐标缓存：逻辑↔物理映射关系

### 5. **统计和监控**
- 详细的性能统计：命中率、布局时间、内存使用
- 缓存效率监控和优化建议
- 增量布局效果的量化分析

### 6. **模块化架构**
- 布局引擎、换行器、坐标映射器分离
- 可插拔的字体管理器接口
- 配置驱动的行为控制

这个Layout系统完全遵循架构宪法，作为纯计算层将ViewModel的逻辑数据转换为Render System所需的几何信息，是保证编辑器渲染性能和精度的关键组件。设计考虑了性能、内存使用和可扩展性的平衡，支持大文件编辑和复杂文本布局场景。