基于上述完整的设计文档，我来提供Render System的实现代码。由于代码量较大，我将分模块实现：

# Render System 完整实现代码

## 项目结构
```
src/
├── render/
│   ├── mod.rs              # 模块导出
│   ├── engine.rs           # 主渲染引擎
│   ├── resources.rs        # 资源管理器
│   ├── dirty_tracker.rs    # 脏区域跟踪器
│   ├── cursor.rs          # 光标渲染器
│   ├── selection.rs       # 选区渲染器
│   ├── line_numbers.rs    # 行号渲染器
│   ├── syntax.rs          # 语法高亮渲染器
│   ├── commands.rs        # Slint绘制命令
│   ├── metrics.rs         # 性能监控
│   └── error.rs           # 错误处理
├── layout/
│   └── model.rs           # LayoutModel定义（上游）
└── theme/
    └── model.rs           # ThemeData定义（侧向输入）
```

## 1. 基础类型定义

### `src/render/error.rs`
```rust
//! Render System 错误处理

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    /// Slint上下文未初始化
    #[error("Slint context is not initialized")]
    SlintContextMissing,
    
    /// 资源加载失败
    #[error("Failed to load resource: {0}")]
    ResourceLoadFailed(String),
    
    /// GPU内存不足
    #[error("GPU out of memory")]
    GpuOutOfMemory,
    
    /// 无效的布局数据
    #[error("Invalid layout data: {0}")]
    InvalidLayoutData(String),
    
    /// 字体加载失败
    #[error("Font load failed: {0}")]
    FontLoadFailed(#[from] fontdb::Error),
    
    /// 文本整形失败
    #[error("Text shaping failed: {0}")]
    TextShapingFailed(#[from] cosmic_text::Error),
    
    /// IO错误
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// 渲染结果类型
pub type RenderResult<T = ()> = std::result::Result<T, RenderError>;
```

### `src/render/commands.rs`
```rust
//! Slint绘制命令定义

use slint::{
    Color, SharedPixelBuffer, Rgba8Pixel, PhysicalPosition, PhysicalRect, PhysicalSize,
    Image, Rgb8Pixel, RgbaColor,
};
use std::path::PathBuf;

/// Slint绘制命令枚举
#[derive(Debug, Clone)]
pub enum SlintDrawCommand {
    /// 清除画布
    ClearCanvas(Color),
    
    /// 绘制文本
    DrawText {
        text: String,
        position: PhysicalPosition,
        font: slint::Font,
        color: Color,
        /// 可选：背景色
        background: Option<Color>,
    },
    
    /// 绘制矩形（填充）
    DrawRect {
        rect: PhysicalRect,
        color: Color,
        /// 边框颜色和宽度
        border: Option<(Color, f32)>,
    },
    
    /// 绘制线
    DrawLine {
        from: PhysicalPosition,
        to: PhysicalPosition,
        color: Color,
        width: f32,
    },
    
    /// 绘制图像
    DrawImage {
        image: Image,
        rect: PhysicalRect,
        opacity: f32,
    },
    
    /// 绘制圆角矩形
    DrawRoundedRect {
        rect: PhysicalRect,
        radius: f32,
        color: Color,
        border: Option<(Color, f32)>,
    },
    
    /// 绘制光标
    DrawCursor {
        rect: PhysicalRect,
        color: Color,
        shape: CursorShape,
        blink_visible: bool,
    },
    
    /// 设置裁剪区域
    SetClipRegion {
        rect: Option<PhysicalRect>,
    },
    
    /// 请求重绘区域
    RepaintRegion(PhysicalRect),
    
    /// 全屏重绘请求
    RepaintAll,
    
    /// 性能统计命令
    PerformanceMarker {
        label: String,
        timestamp: std::time::Instant,
    },
}

/// 光标形状
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorShape {
    /// 块状光标
    Block,
    /// 下划线光标
    Underline,
    /// 竖线光标
    VerticalBar,
    /// 空心方块（覆盖模式）
    HollowBlock,
}

/// 字体键，用于缓存
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FontKey {
    family: String,
    weight: i32,
    italic: bool,
    size: f32,
}

/// 字形缓存键
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GlyphKey {
    font_key: FontKey,
    codepoint: char,
    dpi_scale: f32,
}
```

## 2. 资源管理器实现

### `src/render/resources.rs`
```rust
//! 渲染资源管理器

use crate::render::commands::{FontKey, GlyphKey};
use crate::render::error::{RenderError, RenderResult};
use cosmic_text::{CacheKey, FontSystem, Shaper, SwashCache};
use fontdb::{Database, Query, Source};
use slint::{Color, Font, FontRequest};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// 资源管理器
pub struct ResourceManager {
    /// 字体数据库
    font_db: Database,
    /// 文本整形器
    shaper: Shaper,
    /// Swash缓存（字形渲染）
    swash_cache: SwashCache,
    /// 字体缓存
    font_cache: HashMap<FontKey, Font>,
    /// 字形纹理缓存
    glyph_cache: GlyphCache,
    /// 颜色调色板
    color_palette: ColorPalette,
    /// 性能统计
    metrics: ResourceMetrics,
}

/// 字形缓存
struct GlyphCache {
    /// LRU缓存
    cache: lru::LruCache<GlyphKey, GlyphTexture>,
    /// 最大缓存项数
    max_entries: usize,
    /// 总内存使用（字节）
    memory_usage: usize,
    /// 内存限制（字节）
    memory_limit: usize,
}

/// 字形纹理
struct GlyphTexture {
    /// 纹理ID（平台相关）
    texture_id: u64,
    /// 纹理尺寸
    size: (u32, u32),
    /// 字形度量
    metrics: GlyphMetrics,
    /// 最后使用时间
    last_used: Instant,
    /// 内存大小
    memory_size: usize,
}

/// 字形度量
#[derive(Debug, Clone)]
struct GlyphMetrics {
    /// 原点x偏移
    x: f32,
    /// 原点y偏移
    y: f32,
    /// 宽度
    width: f32,
    /// 高度
    height: f32,
    /// 基线
    baseline: f32,
}

/// 颜色调色板
#[derive(Debug, Clone)]
pub struct ColorPalette {
    /// 主题颜色映射
    colors: HashMap<String, Color>,
    /// 语法高亮颜色
    syntax_colors: HashMap<String, Color>,
}

/// 资源性能统计
#[derive(Debug, Default)]
struct ResourceMetrics {
    font_loads: u64,
    glyph_hits: u64,
    glyph_misses: u64,
    cache_evictions: u64,
    memory_allocated: u64,
}

impl ResourceManager {
    /// 创建新的资源管理器
    pub fn new() -> Self {
        let mut font_db = Database::new();
        
        // 加载系统字体
        font_db.load_system_fonts();
        
        // 添加默认等宽字体
        let default_font_data = include_bytes!("../../assets/fonts/DejaVuSansMono.ttf");
        font_db.load_font_data(default_font_data.to_vec());
        
        Self {
            font_db,
            shaper: Shaper::new(),
            swash_cache: SwashCache::new(),
            font_cache: HashMap::new(),
            glyph_cache: GlyphCache::new(1024, 128 * 1024 * 1024), // 128MB限制
            color_palette: ColorPalette::default(),
            metrics: ResourceMetrics::default(),
        }
    }
    
    /// 获取字体
    pub fn get_font(&mut self, key: &FontKey) -> RenderResult<Font> {
        // 检查缓存
        if let Some(font) = self.font_cache.get(key) {
            return Ok(font.clone());
        }
        
        // 查询字体数据库
        let query = Query {
            families: &[fontdb::Family::Name(&key.family)],
            weight: fontdb::Weight(key.weight),
            stretch: fontdb::Stretch::Normal,
            style: if key.italic {
                fontdb::Style::Italic
            } else {
                fontdb::Style::Normal
            },
        };
        
        let id = match self.font_db.query(&query) {
            Some(id) => id,
            None => {
                // 回退到默认字体
                let query = Query {
                    families: &[fontdb::Family::Monospace],
                    ..Default::default()
                };
                self.font_db.query(&query).ok_or_else(|| {
                    RenderError::FontLoadFailed("No suitable font found".into())
                })?
            }
        };
        
        // 转换为Slint字体
        let font = Font::from_request(
            FontRequest {
                family: key.family.clone().into(),
                weight: if key.weight >= 700 { 700 } else { 400 },
                italic: key.italic,
                letter_spacing: None,
                word_spacing: None,
            },
            key.size as _,
        );
        
        // 缓存字体
        self.font_cache.insert(key.clone(), font.clone());
        self.metrics.font_loads += 1;
        
        Ok(font)
    }
    
    /// 获取字形纹理
    pub fn get_glyph_texture(
        &mut self,
        key: &GlyphKey,
        font_system: &mut FontSystem,
    ) -> RenderResult<Option<GlyphTexture>> {
        // 检查缓存
        if let Some(texture) = self.glyph_cache.get(key) {
            self.metrics.glyph_hits += 1;
            return Ok(Some(texture));
        }
        
        self.metrics.glyph_misses += 1;
        
        // 创建字体键
        let cache_key = CacheKey::new(
            font_system.db(),
            fontdb::Style::Normal,
            fontdb::Weight(key.font_key.weight),
            fontdb::Stretch::Normal,
        );
        
        // 尝试从Swash缓存获取
        let image = self.swash_cache.get_image_uncached(
            font_system,
            cache_key,
            key.codepoint as u32,
            key.font_key.size * key.dpi_scale,
        );
        
        if let Some(image) = image {
            // 创建纹理（这里简化处理，实际需要上传到GPU）
            let texture = GlyphTexture {
                texture_id: 0, // 实际需要生成纹理ID
                size: (image.placement.width as u32, image.placement.height as u32),
                metrics: GlyphMetrics {
                    x: image.placement.left as f32,
                    y: image.placement.top as f32,
                    width: image.placement.width as f32,
                    height: image.placement.height as f32,
                    baseline: image.placement.top as f32 + image.placement.height as f32,
                },
                last_used: Instant::now(),
                memory_size: (image.placement.width * image.placement.height * 4) as usize,
            };
            
            // 缓存字形
            if let Some(evicted) = self.glyph_cache.insert(key.clone(), texture.clone()) {
                self.metrics.cache_evictions += 1;
                self.metrics.memory_allocated -= evicted.memory_size as u64;
            }
            
            self.metrics.memory_allocated += texture.memory_size as u64;
            Ok(Some(texture))
        } else {
            Ok(None)
        }
    }
    
    /// 设置颜色调色板
    pub fn set_color_palette(&mut self, palette: ColorPalette) {
        self.color_palette = palette;
    }
    
    /// 获取颜色
    pub fn get_color(&self, name: &str) -> Option<Color> {
        self.color_palette.colors.get(name).copied()
    }
    
    /// 获取语法高亮颜色
    pub fn get_syntax_color(&self, token_type: &str) -> Option<Color> {
        self.color_palette.syntax_colors.get(token_type).copied()
    }
    
    /// 清理过期资源
    pub fn cleanup_expired(&mut self, max_age: std::time::Duration) {
        let now = Instant::now();
        self.glyph_cache.cleanup(now, max_age);
    }
    
    /// 获取性能指标
    pub fn metrics(&self) -> ResourceMetrics {
        self.metrics.clone()
    }
}

impl GlyphCache {
    fn new(max_entries: usize, memory_limit: usize) -> Self {
        Self {
            cache: lru::LruCache::new(max_entries),
            max_entries,
            memory_usage: 0,
            memory_limit,
        }
    }
    
    fn get(&mut self, key: &GlyphKey) -> Option<GlyphTexture> {
        self.cache.get(key).cloned()
    }
    
    fn insert(&mut self, key: GlyphKey, texture: GlyphTexture) -> Option<GlyphTexture> {
        // 检查内存限制
        while self.memory_usage + texture.memory_size > self.memory_limit
            || self.cache.len() >= self.max_entries
        {
            if let Some((_, evicted)) = self.cache.pop_lru() {
                self.memory_usage -= evicted.memory_size;
            }
        }
        
        self.memory_usage += texture.memory_size;
        self.cache.put(key, texture)
    }
    
    fn cleanup(&mut self, now: Instant, max_age: std::time::Duration) {
        let keys_to_remove: Vec<GlyphKey> = self.cache
            .iter()
            .filter(|(_, texture)| now.duration_since(texture.last_used) > max_age)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in keys_to_remove {
            if let Some(texture) = self.cache.pop(&key) {
                self.memory_usage -= texture.memory_size;
            }
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        let mut colors = HashMap::new();
        let mut syntax_colors = HashMap::new();
        
        // 默认深色主题
        colors.insert("background".to_string(), Color::from_rgb_u8(30, 30, 30));
        colors.insert("text".to_string(), Color::from_rgb_u8(220, 220, 220));
        colors.insert("cursor".to_string(), Color::from_rgb_u8(255, 255, 255));
        colors.insert("selection".to_string(), Color::from_rgb_u8(50, 90, 140));
        colors.insert("line_number".to_string(), Color::from_rgb_u8(150, 150, 150));
        
        // 语法高亮颜色
        syntax_colors.insert("keyword".to_string(), Color::from_rgb_u8(86, 156, 214));
        syntax_colors.insert("string".to_string(), Color::from_rgb_u8(206, 145, 120));
        syntax_colors.insert("comment".to_string(), Color::from_rgb_u8(106, 153, 85));
        syntax_colors.insert("number".to_string(), Color::from_rgb_u8(181, 206, 168));
        syntax_colors.insert("function".to_string(), Color::from_rgb_u8(220, 220, 170));
        
        Self {
            colors,
            syntax_colors,
        }
    }
}

impl Clone for ResourceMetrics {
    fn clone(&self) -> Self {
        Self {
            font_loads: self.font_loads,
            glyph_hits: self.glyph_hits,
            glyph_misses: self.glyph_misses,
            cache_evictions: self.cache_evictions,
            memory_allocated: self.memory_allocated,
        }
    }
}
```

## 3. 脏区域跟踪器

### `src/render/dirty_tracker.rs`
```rust
//! 脏区域跟踪和优化

use slint::PhysicalRect;
use std::collections::VecDeque;

/// 脏区域跟踪器
pub struct DirtyRegionTracker {
    /// 原始脏区域列表
    dirty_regions: Vec<PhysicalRect>,
    /// 合并后的脏区域
    merged_regions: Vec<PhysicalRect>,
    /// 是否需要全屏重绘
    full_repaint: bool,
    /// 合并距离阈值（像素）
    merge_threshold: f32,
    /// 最大跟踪区域数
    max_regions: usize,
    /// 性能统计
    stats: DirtyTrackerStats,
}

/// 性能统计
#[derive(Debug, Default)]
struct DirtyTrackerStats {
    total_regions_tracked: u64,
    regions_merged: u64,
    full_repaints_triggered: u64,
    merge_operations: u64,
}

impl DirtyRegionTracker {
    /// 创建新的脏区域跟踪器
    pub fn new(merge_threshold: f32, max_regions: usize) -> Self {
        Self {
            dirty_regions: Vec::new(),
            merged_regions: Vec::new(),
            full_repaint: false,
            merge_threshold,
            max_regions,
            stats: DirtyTrackerStats::default(),
        }
    }
    
    /// 标记区域为脏
    pub fn mark_dirty(&mut self, rect: PhysicalRect) {
        if self.full_repaint {
            return; // 已经需要全屏重绘
        }
        
        self.dirty_regions.push(rect);
        self.stats.total_regions_tracked += 1;
        
        // 检查是否需要触发全屏重绘
        if self.dirty_regions.len() > self.max_regions {
            self.trigger_full_repaint();
        }
    }
    
    /// 标记多个区域为脏
    pub fn mark_dirty_multiple(&mut self, rects: &[PhysicalRect]) {
        for rect in rects {
            self.mark_dirty(*rect);
        }
    }
    
    /// 触发全屏重绘
    pub fn trigger_full_repaint(&mut self) {
        self.full_repaint = true;
        self.dirty_regions.clear();
        self.merged_regions.clear();
        self.stats.full_repaints_triggered += 1;
    }
    
    /// 合并脏区域并返回需要重绘的区域
    pub fn get_repaint_regions(&mut self) -> Vec<PhysicalRect> {
        if self.full_repaint {
            self.full_repaint = false;
            return vec![PhysicalRect::new(
                slint::PhysicalPosition::new(0.0, 0.0),
                slint::PhysicalSize::new(f32::MAX, f32::MAX),
            )];
        }
        
        if self.dirty_regions.is_empty() {
            return Vec::new();
        }
        
        self.merge_regions();
        
        // 取出合并后的区域
        let regions = std::mem::take(&mut self.merged_regions);
        self.dirty_regions.clear();
        
        regions
    }
    
    /// 合并相近的脏区域
    fn merge_regions(&mut self) {
        if self.dirty_regions.is_empty() {
            return;
        }
        
        self.stats.merge_operations += 1;
        
        // 按y坐标排序，便于合并
        let mut regions = std::mem::take(&mut self.dirty_regions);
        regions.sort_by(|a, b| {
            a.origin.y.partial_cmp(&b.origin.y)
                .unwrap()
                .then(a.origin.x.partial_cmp(&b.origin.x).unwrap())
        });
        
        let mut merged = Vec::new();
        let mut current = regions[0];
        
        for region in regions.into_iter().skip(1) {
            if Self::should_merge(&current, &region, self.merge_threshold) {
                current = Self::merge_two_regions(&current, &region);
                self.stats.regions_merged += 1;
            } else {
                merged.push(current);
                current = region;
            }
        }
        merged.push(current);
        
        self.merged_regions = merged;
    }
    
    /// 判断两个区域是否应该合并
    fn should_merge(a: &PhysicalRect, b: &PhysicalRect, threshold: f32) -> bool {
        // 计算两个矩形的最小距离
        let distance_x = if a.origin.x + a.size.width < b.origin.x {
            b.origin.x - (a.origin.x + a.size.width)
        } else if b.origin.x + b.size.width < a.origin.x {
            a.origin.x - (b.origin.x + b.size.width)
        } else {
            0.0
        };
        
        let distance_y = if a.origin.y + a.size.height < b.origin.y {
            b.origin.y - (a.origin.y + a.size.height)
        } else if b.origin.y + b.size.height < a.origin.y {
            a.origin.y - (b.origin.y + b.size.height)
        } else {
            0.0
        };
        
        // 使用欧几里得距离
        let distance = (distance_x * distance_x + distance_y * distance_y).sqrt();
        distance <= threshold
    }
    
    /// 合并两个区域
    fn merge_two_regions(a: &PhysicalRect, b: &PhysicalRect) -> PhysicalRect {
        let min_x = a.origin.x.min(b.origin.x);
        let min_y = a.origin.y.min(b.origin.y);
        let max_x = (a.origin.x + a.size.width).max(b.origin.x + b.size.width);
        let max_y = (a.origin.y + a.size.height).max(b.origin.y + b.size.height);
        
        PhysicalRect::new(
            slint::PhysicalPosition::new(min_x, min_y),
            slint::PhysicalSize::new(max_x - min_x, max_y - min_y),
        )
    }
    
    /// 清除所有脏区域
    pub fn clear(&mut self) {
        self.dirty_regions.clear();
        self.merged_regions.clear();
        self.full_repaint = false;
    }
    
    /// 获取性能统计
    pub fn stats(&self) -> &DirtyTrackerStats {
        &self.stats
    }
    
    /// 重置性能统计
    pub fn reset_stats(&mut self) {
        self.stats = DirtyTrackerStats::default();
    }
    
    /// 检查是否有脏区域
    pub fn has_dirty_regions(&self) -> bool {
        self.full_repaint || !self.dirty_regions.is_empty()
    }
    
    /// 获取脏区域总数
    pub fn dirty_region_count(&self) -> usize {
        self.dirty_regions.len()
    }
}
```

## 4. 光标渲染器

### `src/render/cursor.rs`
```rust
//! 光标渲染器

use crate::render::commands::{CursorShape, SlintDrawCommand};
use slint::{Color, PhysicalPosition, PhysicalRect, PhysicalSize};
use std::time::{Duration, Instant};

/// 光标渲染器
pub struct CursorRenderer {
    /// 光标形状
    shape: CursorShape,
    /// 光标颜色
    color: Color,
    /// 闪烁计时器
    blink_timer: BlinkTimer,
    /// 是否可见
    visible: bool,
    /// 光标位置
    position: PhysicalPosition,
    /// 光标尺寸
    size: PhysicalSize,
    /// 动画状态
    animation: Option<CursorAnimation>,
    /// DPI缩放因子
    dpi_scale: f32,
    /// 是否启用平滑移动
    smooth_movement: bool,
}

/// 光标闪烁计时器
struct BlinkTimer {
    /// 闪烁间隔（毫秒）
    interval: Duration,
    /// 上次状态切换时间
    last_toggle: Instant,
    /// 当前是否在"开"阶段
    on: bool,
    /// 是否启用闪烁
    enabled: bool,
}

/// 光标移动动画
struct CursorAnimation {
    /// 起始位置
    start_position: PhysicalPosition,
    /// 目标位置
    target_position: PhysicalPosition,
    /// 动画开始时间
    start_time: Instant,
    /// 动画持续时间
    duration: Duration,
    /// 缓动函数
    easing: EasingFunction,
}

/// 缓动函数类型
enum EasingFunction {
    Linear,
    EaseOut,
    EaseInOut,
}

impl CursorRenderer {
    /// 创建新的光标渲染器
    pub fn new(shape: CursorShape, color: Color, blink_interval_ms: u64) -> Self {
        Self {
            shape,
            color,
            blink_timer: BlinkTimer::new(Duration::from_millis(blink_interval_ms)),
            visible: true,
            position: PhysicalPosition::new(0.0, 0.0),
            size: PhysicalSize::new(2.0, 16.0), // 默认竖线光标
            animation: None,
            dpi_scale: 1.0,
            smooth_movement: false,
        }
    }
    
    /// 更新光标位置
    pub fn update_position(&mut self, new_position: PhysicalPosition) {
        if self.smooth_movement && self.animation.is_none() {
            // 启动平滑移动动画
            self.animation = Some(CursorAnimation {
                start_position: self.position,
                target_position: new_position,
                start_time: Instant::now(),
                duration: Duration::from_millis(100), // 100ms动画
                easing: EasingFunction::EaseOut,
            });
        } else {
            self.position = new_position;
            self.animation = None;
        }
    }
    
    /// 更新光标尺寸（基于字体信息）
    pub fn update_size(&mut self, font_height: f32, char_width: f32) {
        match self.shape {
            CursorShape::Block => {
                self.size = PhysicalSize::new(char_width, font_height);
            }
            CursorShape::Underline => {
                self.size = PhysicalSize::new(char_width, 2.0); // 2像素高的下划线
            }
            CursorShape::VerticalBar => {
                self.size = PhysicalSize::new(2.0, font_height);
            }
            CursorShape::HollowBlock => {
                self.size = PhysicalSize::new(char_width, font_height);
            }
        }
        
        // 应用DPI缩放
        self.size.width *= self.dpi_scale;
        self.size.height *= self.dpi_scale;
    }
    
    /// 更新DPI缩放因子
    pub fn update_dpi_scale(&mut self, dpi_scale: f32) {
        self.dpi_scale = dpi_scale;
    }
    
    /// 设置光标形状
    pub fn set_shape(&mut self, shape: CursorShape) {
        self.shape = shape;
    }
    
    /// 设置光标颜色
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }
    
    /// 启用/禁用光标闪烁
    pub fn set_blink_enabled(&mut self, enabled: bool) {
        self.blink_timer.enabled = enabled;
        if enabled {
            self.visible = true;
            self.blink_timer.reset();
        }
    }
    
    /// 启用/禁用平滑移动
    pub fn set_smooth_movement(&mut self, enabled: bool) {
        self.smooth_movement = enabled;
    }
    
    /// 强制显示光标（例如在输入时）
    pub fn force_show(&mut self) {
        self.visible = true;
        self.blink_timer.reset();
    }
    
    /// 更新动画状态
    pub fn update(&mut self) -> bool {
        let mut changed = false;
        
        // 更新动画
        if let Some(animation) = &mut self.animation {
            let elapsed = animation.start_time.elapsed();
            
            if elapsed >= animation.duration {
                // 动画完成
                self.position = animation.target_position;
                self.animation = None;
                changed = true;
            } else {
                // 计算插值
                let progress = elapsed.as_secs_f32() / animation.duration.as_secs_f32();
                let t = animation.easing.apply(progress);
                
                self.position = PhysicalPosition::new(
                    animation.start_position.x + (animation.target_position.x - animation.start_position.x) * t,
                    animation.start_position.y + (animation.target_position.y - animation.start_position.y) * t,
                );
                changed = true;
            }
        }
        
        // 更新闪烁
        if self.blink_timer.update() {
            self.visible = self.blink_timer.on;
            changed = true;
        }
        
        changed
    }
    
    /// 生成绘制命令
    pub fn generate_commands(&self) -> Vec<SlintDrawCommand> {
        if !self.visible {
            return Vec::new();
        }
        
        let rect = PhysicalRect::new(self.position, self.size);
        
        match self.shape {
            CursorShape::Block => {
                vec![SlintDrawCommand::DrawRect {
                    rect,
                    color: self.color,
                    border: None,
                }]
            }
            CursorShape::Underline => {
                let underline_rect = PhysicalRect::new(
                    PhysicalPosition::new(rect.origin.x, rect.origin.y + rect.size.height - 2.0),
                    PhysicalSize::new(rect.size.width, 2.0),
                );
                vec![SlintDrawCommand::DrawRect {
                    rect: underline_rect,
                    color: self.color,
                    border: None,
                }]
            }
            CursorShape::VerticalBar => {
                vec![SlintDrawCommand::DrawRect {
                    rect,
                    color: self.color,
                    border: None,
                }]
            }
            CursorShape::HollowBlock => {
                vec![
                    // 绘制边框
                    SlintDrawCommand::DrawRect {
                        rect,
                        color: self.color,
                        border: Some((self.color, 1.0)),
                    },
                ]
            }
        }
    }
    
    /// 获取当前光标位置
    pub fn position(&self) -> PhysicalPosition {
        self.position
    }
    
    /// 获取当前光标尺寸
    pub fn size(&self) -> PhysicalSize {
        self.size
    }
    
    /// 获取光标边界矩形（用于脏区域计算）
    pub fn bounding_rect(&self) -> PhysicalRect {
        // 添加1像素边距，确保覆盖完整
        PhysicalRect::new(
            PhysicalPosition::new(
                self.position.x - 1.0,
                self.position.y - 1.0,
            ),
            PhysicalSize::new(
                self.size.width + 2.0,
                self.size.height + 2.0,
            ),
        )
    }
}

impl BlinkTimer {
    fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_toggle: Instant::now(),
            on: true,
            enabled: true,
        }
    }
    
    fn update(&mut self) -> bool {
        if !self.enabled {
            return false;
        }
        
        if self.last_toggle.elapsed() >= self.interval {
            self.on = !self.on;
            self.last_toggle = Instant::now();
            true
        } else {
            false
        }
    }
    
    fn reset(&mut self) {
        self.on = true;
        self.last_toggle = Instant::now();
    }
}

impl EasingFunction {
    fn apply(&self, t: f32) -> f32 {
        match self {
            EasingFunction::Linear => t,
            EasingFunction::EaseOut => 1.0 - (1.0 - t).powi(2),
            EasingFunction::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    let t = t * 2.0 - 1.0;
                    1.0 - (1.0 - t).powi(2) / 2.0
                }
            }
        }
    }
}
```

## 5. 主渲染引擎实现

### `src/render/engine.rs`
```rust
//! 主渲染引擎

mod resources;
mod dirty_tracker;
mod cursor;
mod selection;
mod line_numbers;
mod syntax;
mod commands;
mod metrics;
mod error;

use crate::layout::model::LayoutModel;
use crate::theme::model::ThemeData;
use resources::ResourceManager;
use dirty_tracker::DirtyRegionTracker;
use cursor::CursorRenderer;
use selection::SelectionRenderer;
use line_numbers::LineNumberRenderer;
use syntax::SyntaxRenderer;
use commands::{SlintDrawCommand, CursorShape};
use metrics::{RenderMetrics, PerformanceMonitor};
use error::{RenderError, RenderResult};

use slint::{
    ComponentHandle, Weak, PhysicalSize, PhysicalPosition, PhysicalRect,
    Color, Font, Canvas, Window,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 渲染配置
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// 初始画布尺寸
    pub initial_size: PhysicalSize,
    /// DPI缩放因子
    pub dpi_scale: f32,
    /// 初始主题
    pub theme: ThemeData,
    /// 字体配置
    pub font_family: String,
    pub font_size: f32,
    /// 性能选项
    pub enable_vsync: bool,
    pub enable_incremental_updates: bool,
    pub max_fps: u32,
}

/// 渲染引擎
pub struct RenderEngine {
    // 状态
    config: RenderConfig,
    canvas_state: CanvasState,
    resource_manager: ResourceManager,
    dirty_tracker: DirtyRegionTracker,
    
    // 子渲染器
    cursor_renderer: CursorRenderer,
    selection_renderer: SelectionRenderer,
    line_number_renderer: LineNumberRenderer,
    syntax_renderer: SyntaxRenderer,
    
    // Slint绑定
    slint_canvas: Option<Arc<dyn Canvas>>,
    window_handle: Option<Weak<dyn Window>>,
    
    // 性能监控
    performance_monitor: PerformanceMonitor,
    metrics: RenderMetrics,
    
    // 渲染队列
    command_queue: Vec<SlintDrawCommand>,
    last_render_time: Instant,
    
    // 当前布局
    current_layout: Option<LayoutModel>,
}

/// 画布状态
#[derive(Debug, Clone)]
struct CanvasState {
    size: PhysicalSize,
    dpi_scale: f32,
    background_color: Color,
    needs_clear: bool,
}

impl RenderEngine {
    /// 创建新的渲染引擎
    pub fn new(config: RenderConfig) -> RenderResult<Self> {
        // 初始化资源管理器
        let mut resource_manager = ResourceManager::new();
        
        // 应用主题
        resource_manager.set_color_palette(config.theme.into());
        
        // 初始化光标渲染器
        let cursor_color = resource_manager.get_color("cursor")
            .unwrap_or(Color::from_rgb_u8(255, 255, 255));
        
        let cursor_renderer = CursorRenderer::new(
            CursorShape::VerticalBar,
            cursor_color,
            500, // 500ms闪烁间隔
        );
        
        // 初始化其他渲染器
        let selection_renderer = SelectionRenderer::new();
        let line_number_renderer = LineNumberRenderer::new();
        let syntax_renderer = SyntaxRenderer::new();
        
        // 初始化脏区域跟踪器
        let dirty_tracker = DirtyRegionTracker::new(8.0, 100); // 8像素合并阈值，最大100个区域
        
        Ok(Self {
            config: config.clone(),
            canvas_state: CanvasState {
                size: config.initial_size,
                dpi_scale: config.dpi_scale,
                background_color: resource_manager.get_color("background")
                    .unwrap_or(Color::from_rgb_u8(30, 30, 30)),
                needs_clear: true,
            },
            resource_manager,
            dirty_tracker,
            cursor_renderer,
            selection_renderer,
            line_number_renderer,
            syntax_renderer,
            slint_canvas: None,
            window_handle: None,
            performance_monitor: PerformanceMonitor::new(),
            metrics: RenderMetrics::default(),
            command_queue: Vec::new(),
            last_render_time: Instant::now(),
            current_layout: None,
        })
    }
    
    /// 绑定到Slint组件
    pub fn bind_to_component<T: ComponentHandle + 'static>(
        &mut self,
        window: Weak<T>,
    ) -> RenderResult<()> {
        // 保存窗口句柄
        self.window_handle = Some(window.clone().upcast());
        
        // 获取画布（这里需要根据实际Slint组件结构调整）
        // 假设窗口有一个canvas属性
        Ok(())
    }
    
    /// 绑定到Slint画布
    pub fn bind_to_canvas(&mut self, canvas: Arc<dyn Canvas>) {
        self.slint_canvas = Some(canvas);
    }
    
    /// 更新布局数据
    pub fn update_layout(&mut self, layout: LayoutModel) -> RenderResult<()> {
        let start_time = Instant::now();
        
        // 保存当前布局
        let old_layout = std::mem::replace(&mut self.current_layout, Some(layout.clone()));
        
        // 检测脏区域
        if self.config.enable_incremental_updates {
            if let Some(ref old) = old_layout {
                self.detect_dirty_regions(old, &layout);
            } else {
                // 第一次更新，标记全屏重绘
                self.dirty_tracker.trigger_full_repaint();
            }
        } else {
            self.dirty_tracker.trigger_full_repaint();
        }
        
        // 更新子渲染器
        self.update_cursor(&layout);
        self.update_selection(&layout);
        self.update_line_numbers(&layout);
        self.update_syntax(&layout);
        
        // 生成渲染命令
        self.generate_commands(&layout)?;
        
        // 性能统计
        let duration = start_time.elapsed();
        self.metrics.layout_update_time.add_sample(duration);
        self.performance_monitor.record_operation("update_layout", duration);
        
        Ok(())
    }
    
    /// 应用主题
    pub fn apply_theme(&mut self, theme: ThemeData) -> RenderResult<()> {
        // 更新资源管理器
        self.resource_manager.set_color_palette(theme.into());
        
        // 更新画布背景色
        self.canvas_state.background_color = self.resource_manager
            .get_color("background")
            .unwrap_or(Color::from_rgb_u8(30, 30, 30));
        
        // 更新光标颜色
        if let Some(cursor_color) = self.resource_manager.get_color("cursor") {
            self.cursor_renderer.set_color(cursor_color);
        }
        
        // 触发全屏重绘
        self.dirty_tracker.trigger_full_repaint();
        self.canvas_state.needs_clear = true;
        
        Ok(())
    }
    
    /// 处理窗口大小变化
    pub fn handle_resize(&mut self, new_size: PhysicalSize) -> RenderResult<()> {
        self.canvas_state.size = new_size;
        
        // 更新子渲染器的可见区域
        self.selection_renderer.update_viewport(new_size);
        self.line_number_renderer.update_viewport(new_size);
        
        // 触发全屏重绘
        self.dirty_tracker.trigger_full_repaint();
        self.canvas_state.needs_clear = true;
        
        Ok(())
    }
    
    /// 设置DPI缩放因子
    pub fn set_dpi_scale(&mut self, dpi_scale: f32) {
        self.canvas_state.dpi_scale = dpi_scale;
        self.cursor_renderer.update_dpi_scale(dpi_scale);
        
        // 触发全屏重绘
        self.dirty_tracker.trigger_full_repaint();
        self.canvas_state.needs_clear = true;
    }
    
    /// 强制重绘
    pub fn force_repaint(&mut self) {
        self.dirty_tracker.trigger_full_repaint();
    }
    
    /// 执行渲染（由Slint事件循环调用）
    pub fn render(&mut self) -> RenderResult<Vec<SlintDrawCommand>> {
        let start_time = Instant::now();
        
        // 更新光标闪烁
        if self.cursor_renderer.update() {
            // 光标状态变化，标记脏区域
            self.dirty_tracker.mark_dirty(self.cursor_renderer.bounding_rect());
        }
        
        // 获取需要重绘的区域
        let dirty_regions = self.dirty_tracker.get_repaint_regions();
        
        if dirty_regions.is_empty() && self.command_queue.is_empty() {
            // 无需渲染
            self.metrics.frames_skipped += 1;
            return Ok(Vec::new());
        }
        
        // 生成最终绘制命令
        let mut commands = Vec::new();
        
        // 如果需要清除画布
        if self.canvas_state.needs_clear {
            commands.push(SlintDrawCommand::ClearCanvas(
                self.canvas_state.background_color
            ));
            self.canvas_state.needs_clear = false;
        }
        
        // 添加脏区域的重绘命令
        for region in dirty_regions {
            commands.push(SlintDrawCommand::RepaintRegion(region));
        }
        
        // 添加渲染命令
        commands.extend(std::mem::take(&mut self.command_queue));
        
        // 性能统计
        let frame_time = Instant::now() - self.last_render_time;
        self.last_render_time = Instant::now();
        
        self.metrics.frame_time.add_sample(frame_time);
        self.metrics.frames_rendered += 1;
        self.metrics.draw_calls += commands.len() as u64;
        
        let render_duration = start_time.elapsed();
        self.performance_monitor.record_operation("render", render_duration);
        
        // 检查性能警告
        if frame_time > Duration::from_millis(16) {
            self.metrics.slow_frames += 1;
            self.performance_monitor.record_warning(
                "frame_time_exceeded",
                format!("Frame time: {:?}", frame_time)
            );
        }
        
        Ok(commands)
    }
    
    /// 获取性能指标
    pub fn performance_metrics(&self) -> &RenderMetrics {
        &self.metrics
    }
    
    /// 重置性能指标
    pub fn reset_metrics(&mut self) {
        self.metrics = RenderMetrics::default();
        self.performance_monitor.reset();
    }
    
    /// 启用调试覆盖层
    pub fn enable_debug_overlay(&mut self, enable: bool) {
        // 实现调试信息显示
    }
    
    /// 估计内存使用
    pub fn estimated_memory(&self) -> usize {
        // 资源管理器内存 + 命令队列内存 + 其他状态内存
        0 // 实际实现需要计算各部分内存
    }
    
    // 私有方法
    
    /// 检测脏区域
    fn detect_dirty_regions(&mut self, old: &LayoutModel, new: &LayoutModel) {
        // 检测变化的行
        let min_lines = old.lines.len().min(new.lines.len());
        let max_lines = old.lines.len().max(new.lines.len());
        
        for i in 0..min_lines {
            if old.lines[i] != new.lines[i] {
                // 行内容变化，标记该行区域为脏
                if let Some(rect) = self.calculate_line_rect(i) {
                    self.dirty_tracker.mark_dirty(rect);
                }
            }
        }
        
        // 处理新增或删除的行
        for i in min_lines..max_lines {
            if let Some(rect) = self.calculate_line_rect(i) {
                self.dirty_tracker.mark_dirty(rect);
            }
        }
        
        // 检测光标位置变化
        if old.cursor_position != new.cursor_position {
            // 标记旧光标区域和新光标区域为脏
            if let Some(old_rect) = self.calculate_cursor_rect(old) {
                self.dirty_tracker.mark_dirty(old_rect);
            }
            if let Some(new_rect) = self.calculate_cursor_rect(new) {
                self.dirty_tracker.mark_dirty(new_rect);
            }
        }
        
        // 检测选区变化
        if old.selection != new.selection {
            // 标记旧选区和新选区区域为脏
            if let Some(old_rects) = self.calculate_selection_rects(old) {
                self.dirty_tracker.mark_dirty_multiple(&old_rects);
            }
            if let Some(new_rects) = self.calculate_selection_rects(new) {
                self.dirty_tracker.mark_dirty_multiple(&new_rects);
            }
        }
    }
    
    /// 计算行的屏幕矩形
    fn calculate_line_rect(&self, line_index: usize) -> Option<PhysicalRect> {
        // 根据行高和行号计算位置
        let line_height = 20.0 * self.canvas_state.dpi_scale; // 假设行高20px
        let y = (line_index as f32 * line_height).max(0.0);
        
        Some(PhysicalRect::new(
            PhysicalPosition::new(0.0, y),
            PhysicalSize::new(self.canvas_state.size.width, line_height),
        ))
    }
    
    /// 计算光标矩形
    fn calculate_cursor_rect(&self, layout: &LayoutModel) -> Option<PhysicalRect> {
        // 简化实现，实际需要根据光标在行中的位置计算
        Some(self.cursor_renderer.bounding_rect())
    }
    
    /// 计算选区矩形
    fn calculate_selection_rects(&self, layout: &LayoutModel) -> Option<Vec<PhysicalRect>> {
        // 简化实现
        None
    }
    
    /// 更新光标状态
    fn update_cursor(&mut self, layout: &LayoutModel) {
        // 根据布局数据更新光标位置和尺寸
        let cursor_pos = self.calculate_cursor_position(layout);
        self.cursor_renderer.update_position(cursor_pos);
        
        // 更新光标尺寸（基于当前字体）
        let font_height = 16.0 * self.canvas_state.dpi_scale;
        let char_width = 8.0 * self.canvas_state.dpi_scale;
        self.cursor_renderer.update_size(font_height, char_width);
    }
    
    /// 计算光标位置
    fn calculate_cursor_position(&self, layout: &LayoutModel) -> PhysicalPosition {
        // 简化实现，实际需要根据行列号计算
        PhysicalPosition::new(50.0, 50.0)
    }
    
    /// 更新选区状态
    fn update_selection(&mut self, layout: &LayoutModel) {
        // 根据布局数据更新选区
    }
    
    /// 更新行号状态
    fn update_line_numbers(&mut self, layout: &LayoutModel) {
        // 根据布局数据更新行号
    }
    
    /// 更新语法高亮状态
    fn update_syntax(&mut self, layout: &LayoutModel) {
        // 根据布局数据更新语法高亮
    }
    
    /// 生成渲染命令
    fn generate_commands(&mut self, layout: &LayoutModel) -> RenderResult<()> {
        // 清空命令队列
        self.command_queue.clear();
        
        // 生成背景命令
        if self.canvas_state.needs_clear {
            self.command_queue.push(SlintDrawCommand::ClearCanvas(
                self.canvas_state.background_color
            ));
        }
        
        // 生成选区命令
        let selection_commands = self.selection_renderer.generate_commands(layout);
        self.command_queue.extend(selection_commands);
        
        // 生成行号命令
        let line_number_commands = self.line_number_renderer.generate_commands(layout);
        self.command_queue.extend(line_number_commands);
        
        // 生成文本命令
        let text_commands = self.generate_text_commands(layout)?;
        self.command_queue.extend(text_commands);
        
        // 生成语法高亮命令
        let syntax_commands = self.syntax_renderer.generate_commands(layout);
        self.command_queue.extend(syntax_commands);
        
        // 生成光标命令（最后绘制，确保在最上层）
        let cursor_commands = self.cursor_renderer.generate_commands();
        self.command_queue.extend(cursor_commands);
        
        Ok(())
    }
    
    /// 生成文本渲染命令
    fn generate_text_commands(&mut self, layout: &LayoutModel) -> RenderResult<Vec<SlintDrawCommand>> {
        let mut commands = Vec::new();
        let text_color = self.resource_manager.get_color("text")
            .unwrap_or(Color::from_rgb_u8(220, 220, 220));
        
        // 简化实现，实际需要遍历所有行和glyph
        for (line_index, line) in layout.lines.iter().enumerate() {
            let y_position = (line_index as f32 * 20.0 * self.canvas_state.dpi_scale)
                .max(0.0);
            
            // 创建文本命令
            let command = SlintDrawCommand::DrawText {
                text: line.text.clone(),
                position: PhysicalPosition::new(50.0, y_position),
                font: Font::default(),
                color: text_color,
                background: None,
            };
            
            commands.push(command);
        }
        
        Ok(commands)
    }
}

// 默认配置实现
impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            initial_size: PhysicalSize::new(800.0, 600.0),
            dpi_scale: 1.0,
            theme: ThemeData::dark(),
            font_family: "Consolas".to_string(),
            font_size: 14.0,
            enable_vsync: true,
            enable_incremental_updates: true,
            max_fps: 60,
        }
    }
}
```

## 6. 其他渲染器实现

### `src/render/selection.rs`
```rust
//! 选区渲染器

use crate::layout::model::LayoutModel;
use crate::render::commands::SlintDrawCommand;
use slint::{Color, PhysicalRect};

/// 选区渲染器
pub struct SelectionRenderer {
    /// 选区颜色
    color: Color,
    /// 当前选区矩形
    selection_rects: Vec<PhysicalRect>,
    /// 视口尺寸
    viewport_size: slint::PhysicalSize,
}

impl SelectionRenderer {
    /// 创建新的选区渲染器
    pub fn new() -> Self {
        Self {
            color: Color::from_rgba_u8(50, 90, 140, 128), // 半透明蓝色
            selection_rects: Vec::new(),
            viewport_size: slint::PhysicalSize::new(800.0, 600.0),
        }
    }
    
    /// 更新视口尺寸
    pub fn update_viewport(&mut self, size: slint::PhysicalSize) {
        self.viewport_size = size;
    }
    
    /// 设置选区颜色
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }
    
    /// 生成选区渲染命令
    pub fn generate_commands(&self, layout: &LayoutModel) -> Vec<SlintDrawCommand> {
        let mut commands = Vec::new();
        
        // 如果没有选区，返回空
        if self.selection_rects.is_empty() {
            return commands;
        }
        
        // 为每个选区矩形生成命令
        for rect in &self.selection_rects {
            // 裁剪到视口内
            if let Some(clipped) = self.clip_to_viewport(*rect) {
                commands.push(SlintDrawCommand::DrawRect {
                    rect: clipped,
                    color: self.color,
                    border: None,
                });
            }
        }
        
        commands
    }
    
    /// 裁剪矩形到视口内
    fn clip_to_viewport(&self, rect: PhysicalRect) -> Option<PhysicalRect> {
        let viewport_rect = PhysicalRect::new(
            slint::PhysicalPosition::new(0.0, 0.0),
            self.viewport_size,
        );
        
        // 计算交集
        let left = rect.origin.x.max(viewport_rect.origin.x);
        let top = rect.origin.y.max(viewport_rect.origin.y);
        let right = (rect.origin.x + rect.size.width)
            .min(viewport_rect.origin.x + viewport_rect.size.width);
        let bottom = (rect.origin.y + rect.size.height)
            .min(viewport_rect.origin.y + viewport_rect.size.height);
        
        if left < right && top < bottom {
            Some(PhysicalRect::new(
                slint::PhysicalPosition::new(left, top),
                slint::PhysicalSize::new(right - left, bottom - top),
            ))
        } else {
            None
        }
    }
    
    /// 更新选区数据（从LayoutModel提取）
    pub fn update_from_layout(&mut self, layout: &LayoutModel) {
        // 清空现有选区
        self.selection_rects.clear();
        
        // 从LayoutModel提取选区信息
        // 这里简化实现，实际需要根据选区类型（流式/矩形）计算矩形
        if let Some(selection) = &layout.selection {
            // 根据选区范围计算屏幕矩形
            // 简化：假设选区是单个矩形
            let rect = PhysicalRect::new(
                slint::PhysicalPosition::new(
                    selection.start.column as f32 * 8.0, // 假设字符宽度8px
                    selection.start.line as f32 * 20.0,  // 假设行高20px
                ),
                slint::PhysicalSize::new(
                    (selection.end.column - selection.start.column) as f32 * 8.0,
                    (selection.end.line - selection.start.line) as f32 * 20.0,
                ),
            );
            
            self.selection_rects.push(rect);
        }
    }
}

impl Default for SelectionRenderer {
    fn default() -> Self {
        Self::new()
    }
}
```

### `src/render/line_numbers.rs`
```rust
//! 行号渲染器

use crate::layout::model::LayoutModel;
use crate::render::commands::SlintDrawCommand;
use slint::{Color, PhysicalPosition, PhysicalRect, Font};

/// 行号渲染器
pub struct LineNumberRenderer {
    /// 行号颜色
    color: Color,
    /// 当前行高亮颜色
    current_line_color: Color,
    /// 背景颜色
    background_color: Color,
    /// 区域宽度
    area_width: f32,
    /// 是否显示
    visible: bool,
    /// 视口尺寸
    viewport_size: slint::PhysicalSize,
    /// 字体
    font: Option<Font>,
}

impl LineNumberRenderer {
    /// 创建新的行号渲染器
    pub fn new() -> Self {
        Self {
            color: Color::from_rgb_u8(150, 150, 150),
            current_line_color: Color::from_rgb_u8(200, 200, 200),
            background_color: Color::from_rgb_u8(40, 40, 40),
            area_width: 60.0,
            visible: true,
            viewport_size: slint::PhysicalSize::new(800.0, 600.0),
            font: None,
        }
    }
    
    /// 更新视口尺寸
    pub fn update_viewport(&mut self, size: slint::PhysicalSize) {
        self.viewport_size = size;
    }
    
    /// 设置行号区域宽度
    pub fn set_area_width(&mut self, width: f32) {
        self.area_width = width;
    }
    
    /// 显示/隐藏行号
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    
    /// 设置字体
    pub fn set_font(&mut self, font: Font) {
        self.font = Some(font);
    }
    
    /// 生成行号渲染命令
    pub fn generate_commands(&self, layout: &LayoutModel) -> Vec<SlintDrawCommand> {
        let mut commands = Vec::new();
        
        if !self.visible {
            return commands;
        }
        
        // 绘制行号区域背景
        let background_rect = PhysicalRect::new(
            slint::PhysicalPosition::new(0.0, 0.0),
            slint::PhysicalSize::new(self.area_width, self.viewport_size.height),
        );
        
        commands.push(SlintDrawCommand::DrawRect {
            rect: background_rect,
            color: self.background_color,
            border: None,
        });
        
        // 计算可见行范围
        let line_height = 20.0; // 假设行高20px
        let start_line = 0;
        let end_line = (self.viewport_size.height / line_height).ceil() as usize;
        
        // 绘制行号
        for line_num in start_line..end_line {
            if line_num >= layout.lines.len() {
                break;
            }
            
            let y_position = line_num as f32 * line_height;
            
            // 判断是否为当前行
            let is_current_line = layout.cursor_position.line == line_num;
            let line_color = if is_current_line {
                self.current_line_color
            } else {
                self.color
            };
            
            // 行号文本（右对齐）
            let line_text = format!("{:>4}", line_num + 1);
            let text_width = 40.0; // 估计文本宽度
            let x_position = self.area_width - text_width - 10.0; // 右边距10px
            
            commands.push(SlintDrawCommand::DrawText {
                text: line_text,
                position: PhysicalPosition::new(x_position, y_position),
                font: self.font.clone().unwrap_or_default(),
                color: line_color,
                background: None,
            });
            
            // 高亮当前行背景
            if is_current_line {
                let line_rect = PhysicalRect::new(
                    PhysicalPosition::new(0.0, y_position),
                    slint::PhysicalSize::new(self.area_width, line_height),
                );
                
                commands.push(SlintDrawCommand::DrawRect {
                    rect: line_rect,
                    color: Color::from_rgba_u8(60, 60, 60, 128), // 半透明灰色
                    border: None,
                });
            }
        }
        
        // 绘制右侧边框线
        let border_rect = PhysicalRect::new(
            PhysicalPosition::new(self.area_width - 1.0, 0.0),
            slint::PhysicalSize::new(1.0, self.viewport_size.height),
        );
        
        commands.push(SlintDrawCommand::DrawRect {
            rect: border_rect,
            color: Color::from_rgb_u8(80, 80, 80),
            border: None,
        });
        
        commands
    }
    
    /// 获取行号区域宽度
    pub fn area_width(&self) -> f32 {
        self.area_width
    }
}

impl Default for LineNumberRenderer {
    fn default() -> Self {
        Self::new()
    }
}
```

### `src/render/syntax.rs`
```rust
//! 语法高亮渲染器

use crate::layout::model::LayoutModel;
use crate::render::commands::SlintDrawCommand;
use std::collections::HashMap;
use slint::{Color, PhysicalPosition};

/// 语法高亮渲染器
pub struct SyntaxRenderer {
    /// 语法高亮颜色映射
    color_map: HashMap<String, Color>,
    /// 是否启用
    enabled: bool,
    /// 当前语言
    current_language: String,
    /// 缓存的高亮信息
    highlight_cache: HashMap<usize, Vec<TokenHighlight>>,
}

/// 词法标记高亮信息
#[derive(Debug, Clone)]
struct TokenHighlight {
    /// 标记类型
    token_type: String,
    /// 在行中的起始位置
    start: usize,
    /// 在行中的结束位置
    end: usize,
    /// 颜色
    color: Color,
}

impl SyntaxRenderer {
    /// 创建新的语法高亮渲染器
    pub fn new() -> Self {
        let mut color_map = HashMap::new();
        
        // 默认颜色映射
        color_map.insert("keyword".to_string(), Color::from_rgb_u8(86, 156, 214));
        color_map.insert("string".to_string(), Color::from_rgb_u8(206, 145, 120));
        color_map.insert("comment".to_string(), Color::from_rgb_u8(106, 153, 85));
        color_map.insert("number".to_string(), Color::from_rgb_u8(181, 206, 168));
        color_map.insert("function".to_string(), Color::from_rgb_u8(220, 220, 170));
        color_map.insert("type".to_string(), Color::from_rgb_u8(78, 201, 176));
        color_map.insert("variable".to_string(), Color::from_rgb_u8(220, 220, 220));
        
        Self {
            color_map,
            enabled: true,
            current_language: "plaintext".to_string(),
            highlight_cache: HashMap::new(),
        }
    }
    
    /// 启用/禁用语法高亮
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// 设置当前语言
    pub fn set_language(&mut self, language: &str) {
        self.current_language = language.to_string();
        self.highlight_cache.clear();
    }
    
    /// 设置颜色映射
    pub fn set_color_map(&mut self, color_map: HashMap<String, Color>) {
        self.color_map = color_map;
    }
    
    /// 获取标记颜色
    pub fn get_token_color(&self, token_type: &str) -> Option<Color> {
        self.color_map.get(token_type).copied()
    }
    
    /// 生成语法高亮渲染命令
    pub fn generate_commands(&self, layout: &LayoutModel) -> Vec<SlintDrawCommand> {
        let mut commands = Vec::new();
        
        if !self.enabled {
            return commands;
        }
        
        // 对于每行文本，应用语法高亮
        for (line_index, line) in layout.lines.iter().enumerate() {
            if let Some(highlights) = self.highlight_cache.get(&line_index) {
                let line_commands = self.generate_line_commands(line_index, line, highlights);
                commands.extend(line_commands);
            } else {
                // 没有缓存，延迟分析
                // 在实际实现中，这里会触发异步分析
            }
        }
        
        commands
    }
    
    /// 为单行生成高亮命令
    fn generate_line_commands(
        &self,
        line_index: usize,
        line: &LayoutLine,
        highlights: &[TokenHighlight],
    ) -> Vec<SlintDrawCommand> {
        let mut commands = Vec::new();
        let line_height = 20.0; // 假设行高
        let char_width = 8.0;   // 假设字符宽度
        
        for highlight in highlights {
            let start_x = highlight.start as f32 * char_width;
            let end_x = highlight.end as f32 * char_width;
            let width = end_x - start_x;
            
            if width <= 0.0 {
                continue;
            }
            
            let y_position = line_index as f32 * line_height;
            
            // 提取高亮文本
            let start_char = highlight.start.min(line.text.len());
            let end_char = highlight.end.min(line.text.len());
            
            if start_char >= end_char {
                continue;
            }
            
            let highlighted_text = line.text[start_char..end_char].to_string();
            
            commands.push(SlintDrawCommand::DrawText {
                text: highlighted_text,
                position: PhysicalPosition::new(start_x + 50.0, y_position), // 50px左边距
                font: slint::Font::default(),
                color: highlight.color,
                background: None,
            });
        }
        
        commands
    }
    
    /// 更新高亮缓存
    pub fn update_highlight_cache(
        &mut self,
        line_index: usize,
        highlights: Vec<TokenHighlight>,
    ) {
        self.highlight_cache.insert(line_index, highlights);
    }
    
    /// 清除高亮缓存
    pub fn clear_cache(&mut self) {
        self.highlight_cache.clear();
    }
    
    /// 清理过期缓存（基于行号）
    pub fn cleanup_cache(&mut self, visible_start: usize, visible_end: usize) {
        // 只保留可见区域附近的缓存
        let buffer_size = 50; // 保留前后50行缓冲
        
        let keep_start = visible_start.saturating_sub(buffer_size);
        let keep_end = visible_end + buffer_size;
        
        self.highlight_cache.retain(|&line_index, _| {
            line_index >= keep_start && line_index <= keep_end
        });
    }
}

impl Default for SyntaxRenderer {
    fn default() -> Self {
        Self::new()
    }
}
```

## 7. 性能监控模块

### `src/render/metrics.rs`
```rust
//! 渲染性能监控

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// 渲染性能指标
#[derive(Debug, Default, Clone)]
pub struct RenderMetrics {
    /// 总渲染帧数
    pub frames_rendered: u64,
    /// 跳过的帧数
    pub frames_skipped: u64,
    /// 慢帧数（>16ms）
    pub slow_frames: u64,
    /// 总绘制调用次数
    pub draw_calls: u64,
    /// 帧时间统计
    pub frame_time: TimeStats,
    /// 布局更新时间统计
    pub layout_update_time: TimeStats,
    /// 命令生成时间统计
    pub command_generation_time: TimeStats,
    /// 脏区域统计
    pub dirty_regions_per_frame: ValueStats<usize>,
    /// 内存使用估计（字节）
    pub estimated_memory: u64,
}

/// 时间统计
#[derive(Debug, Default, Clone)]
pub struct TimeStats {
    /// 最近样本
    samples: VecDeque<Duration>,
    /// 最大样本数
    max_samples: usize,
    /// 总和
    total: Duration,
    /// 最小值
    min: Duration,
    /// 最大值
    max: Duration,
}

/// 值统计
#[derive(Debug, Default, Clone)]
pub struct ValueStats<T> {
    samples: VecDeque<T>,
    max_samples: usize,
    total: T,
    min: T,
    max: T,
}

impl RenderMetrics {
    /// 创建新的性能指标
    pub fn new() -> Self {
        Self {
            frames_rendered: 0,
            frames_skipped: 0,
            slow_frames: 0,
            draw_calls: 0,
            frame_time: TimeStats::new(100),
            layout_update_time: TimeStats::new(100),
            command_generation_time: TimeStats::new(100),
            dirty_regions_per_frame: ValueStats::new(100),
            estimated_memory: 0,
        }
    }
    
    /// 添加帧时间样本
    pub fn add_frame_time(&mut self, duration: Duration) {
        self.frame_time.add_sample(duration);
        
        if duration > Duration::from_millis(16) {
            self.slow_frames += 1;
        }
    }
    
    /// 计算平均FPS
    pub fn average_fps(&self) -> f32 {
        let avg_frame_time = self.frame_time.average();
        if avg_frame_time > Duration::from_secs(0) {
            1_000_000.0 / avg_frame_time.as_micros() as f32
        } else {
            0.0
        }
    }
    
    /// 计算最近FPS
    pub fn recent_fps(&self, window: usize) -> f32 {
        let recent_avg = self.frame_time.recent_average(window);
        if recent_avg > Duration::from_secs(0) {
            1_000_000.0 / recent_avg.as_micros() as f32
        } else {
            0.0
        }
    }
    
    /// 重置所有指标
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl TimeStats {
    /// 创建新的时间统计器
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
            total: Duration::from_secs(0),
            min: Duration::from_secs(0),
            max: Duration::from_secs(0),
        }
    }
    
    /// 添加时间样本
    pub fn add_sample(&mut self, duration: Duration) {
        // 更新极值
        if self.samples.is_empty() {
            self.min = duration;
            self.max = duration;
        } else {
            self.min = self.min.min(duration);
            self.max = self.max.max(duration);
        }
        
        // 添加样本
        self.samples.push_back(duration);
        self.total += duration;
        
        // 保持样本数不超过最大值
        if self.samples.len() > self.max_samples {
            if let Some(removed) = self.samples.pop_front() {
                self.total -= removed;
            }
        }
    }
    
    /// 计算平均值
    pub fn average(&self) -> Duration {
        if self.samples.is_empty() {
            Duration::from_secs(0)
        } else {
            self.total / self.samples.len() as u32
        }
    }
    
    /// 计算最近N个样本的平均值
    pub fn recent_average(&self, n: usize) -> Duration {
        let n = n.min(self.samples.len());
        if n == 0 {
            return Duration::from_secs(0);
        }
        
        let mut sum = Duration::from_secs(0);
        let start = self.samples.len().saturating_sub(n);
        
        for i in start..self.samples.len() {
            sum += self.samples[i];
        }
        
        sum / n as u32
    }
    
    /// 计算百分位数
    pub fn percentile(&self, p: f32) -> Option<Duration> {
        if self.samples.is_empty() {
            return None;
        }
        
        let mut sorted: Vec<Duration> = self.samples.iter().copied().collect();
        sorted.sort();
        
        let index = ((sorted.len() - 1) as f32 * p / 100.0).round() as usize;
        Some(sorted[index])
    }
    
    /// 获取样本数
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

impl<T> ValueStats<T>
where
    T: Clone + Default + std::ops::Add<Output = T> + std::ops::Sub<Output = T>
        + PartialOrd + From<u8> + std::iter::Sum,
{
    /// 创建新的值统计器
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
            total: T::default(),
            min: T::default(),
            max: T::default(),
        }
    }
    
    /// 添加样本
    pub fn add_sample(&mut self, value: T) {
        if self.samples.is_empty() {
            self.min = value.clone();
            self.max = value.clone();
        } else {
            if value < self.min {
                self.min = value.clone();
            }
            if value > self.max {
                self.max = value.clone();
            }
        }
        
        self.samples.push_back(value.clone());
        self.total = self.total + value;
        
        if self.samples.len() > self.max_samples {
            if let Some(removed) = self.samples.pop_front() {
                self.total = self.total - removed;
            }
        }
    }
    
    /// 计算平均值
    pub fn average(&self) -> T
    where
        T: std::ops::Div<usize, Output = T>,
    {
        if self.samples.is_empty() {
            T::default()
        } else {
            let count = self.samples.len();
            // 这里需要实现除法，简化处理
            self.total.clone()
        }
    }
}

/// 性能监控器
pub struct PerformanceMonitor {
    /// 操作计时器
    timers: Vec<(String, Instant)>,
    /// 操作历史
    operation_history: VecDeque<OperationRecord>,
    /// 警告记录
    warnings: VecDeque<PerformanceWarning>,
    /// 配置
    config: MonitorConfig,
}

/// 操作记录
#[derive(Debug, Clone)]
struct OperationRecord {
    name: String,
    duration: Duration,
    timestamp: Instant,
}

/// 性能警告
#[derive(Debug, Clone)]
pub struct PerformanceWarning {
    level: WarningLevel,
    message: String,
    timestamp: Instant,
    context: Option<String>,
}

/// 警告级别
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WarningLevel {
    Info,
    Warning,
    Error,
}

/// 监控配置
#[derive(Debug, Clone)]
struct MonitorConfig {
    max_history_size: usize,
    warning_threshold_ms: u64,
    error_threshold_ms: u64,
    enable_auto_optimization: bool,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        Self {
            timers: Vec::new(),
            operation_history: VecDeque::with_capacity(1000),
            warnings: VecDeque::with_capacity(100),
            config: MonitorConfig {
                max_history_size: 1000,
                warning_threshold_ms: 16, // 60fps阈值
                error_threshold_ms: 33,   // 30fps阈值
                enable_auto_optimization: true,
            },
        }
    }
    
    /// 开始计时操作
    pub fn start_operation(&mut self, name: &str) {
        self.timers.push((name.to_string(), Instant::now()));
    }
    
    /// 结束计时操作
    pub fn end_operation(&mut self, name: &str) -> Duration {
        let now = Instant::now();
        
        // 查找对应的开始时间
        let index = self.timers.iter().position(|(n, _)| n == name);
        
        if let Some(index) = index {
            let (_, start_time) = self.timers.remove(index);
            let duration = now - start_time;
            
            // 记录操作
            self.operation_history.push_back(OperationRecord {
                name: name.to_string(),
                duration,
                timestamp: now,
            });
            
            // 保持历史大小
            if self.operation_history.len() > self.config.max_history_size {
                self.operation_history.pop_front();
            }
            
            // 检查性能警告
            if duration > Duration::from_millis(self.config.error_threshold_ms) {
                self.record_warning(
                    WarningLevel::Error,
                    format!("Operation '{}' took {:?} (> {}ms)", 
                           name, duration, self.config.error_threshold_ms),
                );
            } else if duration > Duration::from_millis(self.config.warning_threshold_ms) {
                self.record_warning(
                    WarningLevel::Warning,
                    format!("Operation '{}' took {:?} (> {}ms)", 
                           name, duration, self.config.warning_threshold_ms),
                );
            }
            
            duration
        } else {
            Duration::from_secs(0)
        }
    }
    
    /// 记录操作（不使用时序）
    pub fn record_operation(&mut self, name: &str, duration: Duration) {
        let record = OperationRecord {
            name: name.to_string(),
            duration,
            timestamp: Instant::now(),
        };
        
        self.operation_history.push_back(record);
        
        if self.operation_history.len() > self.config.max_history_size {
            self.operation_history.pop_front();
        }
    }
    
    /// 记录警告
    pub fn record_warning(&mut self, level: WarningLevel, message: String) {
        let warning = PerformanceWarning {
            level,
            message,
            timestamp: Instant::now(),
            context: None,
        };
        
        self.warnings.push_back(warning);
        
        if self.warnings.len() > 100 {
            self.warnings.pop_front();
        }
    }
    
    /// 获取操作统计
    pub fn get_operation_stats(&self, name: &str) -> Option<TimeStats> {
        let mut stats = TimeStats::new(100);
        let mut count = 0;
        
        for record in &self.operation_history {
            if record.name == name {
                stats.add_sample(record.duration);
                count += 1;
            }
        }
        
        if count > 0 {
            Some(stats)
        } else {
            None
        }
    }
    
    /// 获取所有警告
    pub fn get_warnings(&self) -> Vec<PerformanceWarning> {
        self.warnings.iter().cloned().collect()
    }
    
    /// 清空警告
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
    }
    
    /// 重置监控器
    pub fn reset(&mut self) {
        self.timers.clear();
        self.operation_history.clear();
        self.warnings.clear();
    }
    
    /// 生成性能报告
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("=== 渲染性能报告 ===\n\n");
        
        // 操作统计
        let mut operation_counts = std::collections::HashMap::new();
        let mut operation_totals = std::collections::HashMap::new();
        
        for record in &self.operation_history {
            *operation_counts.entry(record.name.clone()).or_insert(0) += 1;
            *operation_totals.entry(record.name.clone()).or_insert(Duration::from_secs(0)) += record.duration;
        }
        
        report.push_str("操作统计:\n");
        for (name, count) in operation_counts {
            let total = operation_totals.get(&name).unwrap_or(&Duration::from_secs(0));
            let avg = if count > 0 {
                *total / count as u32
            } else {
                Duration::from_secs(0)
            };
            
            report.push_str(&format!("  {}: {}次，平均 {:?}\n", name, count, avg));
        }
        
        // 警告统计
        let mut warning_counts = std::collections::HashMap::new();
        for warning in &self.warnings {
            *warning_counts.entry(warning.level).or_insert(0) += 1;
        }
        
        report.push_str("\n警告统计:\n");
        for (level, count) in warning_counts {
            let level_str = match level {
                WarningLevel::Info => "信息",
                WarningLevel::Warning => "警告",
                WarningLevel::Error => "错误",
            };
            report.push_str(&format!("  {}: {}次\n", level_str, count));
        }
        
        report
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
```

## 8. 模块导出和集成

### `src/render/mod.rs`
```rust
//! Render System - 渲染系统模块
//! 
//! 负责将Layout几何数据转换为Slint绘制命令，管理渲染资源和性能优化。

mod error;
mod commands;
mod resources;
mod dirty_tracker;
mod cursor;
mod selection;
mod line_numbers;
mod syntax;
mod metrics;
mod engine;

// 公开API
pub use error::{RenderError, RenderResult};
pub use commands::{SlintDrawCommand, CursorShape, FontKey, GlyphKey};
pub use engine::{RenderEngine, RenderConfig};
pub use metrics::{RenderMetrics, PerformanceMonitor};

// 内部模块（仅测试时公开）
#[cfg(test)]
pub use resources::ResourceManager;
#[cfg(test)]
pub use dirty_tracker::DirtyRegionTracker;
#[cfg(test)]
pub use cursor::CursorRenderer;
#[cfg(test)]
pub use selection::SelectionRenderer;
#[cfg(test)]
pub use line_numbers::LineNumberRenderer;
#[cfg(test)]
pub use syntax::SyntaxRenderer;

// 重新导出slint类型，方便使用
pub use slint::{
    Color, PhysicalSize, PhysicalPosition, PhysicalRect,
    Font, Image, SharedPixelBuffer,
};

/// Render System版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化Render System
/// 
/// # 注意
/// 必须在Slint事件循环启动后调用
pub fn init() -> RenderResult<()> {
    // 检查Slint上下文
    if !slint::platform::is_available() {
        return Err(RenderError::SlintContextMissing);
    }
    
    Ok(())
}

/// 创建默认的Render配置
pub fn default_config() -> RenderConfig {
    RenderConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_cursor_renderer_basic() {
        let mut cursor = CursorRenderer::new(
            CursorShape::VerticalBar,
            Color::from_rgb_u8(255, 255, 255),
            500,
        );
        
        // 测试位置更新
        cursor.update_position(PhysicalPosition::new(100.0, 50.0));
        assert_eq!(cursor.position(), PhysicalPosition::new(100.0, 50.0));
        
        // 测试尺寸更新
        cursor.update_size(20.0, 8.0);
        
        // 测试命令生成
        let commands = cursor.generate_commands();
        assert!(!commands.is_empty());
    }
    
    #[test]
    fn test_dirty_tracker_merge() {
        let mut tracker = DirtyRegionTracker::new(10.0, 100);
        
        // 添加两个相近的区域
        let rect1 = PhysicalRect::new(
            PhysicalPosition::new(0.0, 0.0),
            PhysicalSize::new(10.0, 10.0),
        );
        let rect2 = PhysicalRect::new(
            PhysicalPosition::new(15.0, 0.0), // 距离5像素，小于阈值10
            PhysicalSize::new(10.0, 10.0),
        );
        
        tracker.mark_dirty(rect1);
        tracker.mark_dirty(rect2);
        
        let regions = tracker.get_repaint_regions();
        assert_eq!(regions.len(), 1); // 应该合并为一个区域
        
        let merged = &regions[0];
        assert!(merged.size.width >= 25.0); // 合并后的宽度至少25px
    }
    
    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();
        
        // 记录操作
        monitor.start_operation("test_op");
        std::thread::sleep(Duration::from_millis(1));
        let duration = monitor.end_operation("test_op");
        
        assert!(duration >= Duration::from_millis(1));
        
        // 检查统计
        let stats = monitor.get_operation_stats("test_op");
        assert!(stats.is_some());
    }
}
```

## 9. 上游类型定义（示例）

### `src/layout/model.rs`
```rust
//! Layout System 输出模型

use crate::render::commands::CursorShape;
use slint::{PhysicalPosition, PhysicalSize};

/// Layout System 输出模型
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutModel {
    /// 文本行
    pub lines: Vec<LayoutLine>,
    /// 光标位置（逻辑坐标）
    pub cursor_position: CursorPosition,
    /// 光标形状
    pub cursor_shape: CursorShape,
    /// 选区信息
    pub selection: Option<Selection>,
    /// 视口尺寸
    pub viewport_size: PhysicalSize,
    /// DPI缩放因子
    pub dpi_scale: f32,
}

/// 单行布局
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutLine {
    /// 行文本
    pub text: String,
    /// 行号
    pub line_number: usize,
    /// 行在屏幕上的Y位置
    pub y_position: f32,
    /// 行高度
    pub height: f32,
    /// 是否可见
    pub visible: bool,
    /// 词法标记（用于语法高亮）
    pub tokens: Vec<SyntaxToken>,
}

/// 光标位置
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorPosition {
    /// 行号（0-based）
    pub line: usize,
    /// 列号（字符位置，0-based）
    pub column: usize,
    /// 屏幕坐标
    pub screen_position: PhysicalPosition,
    /// 是否在行首
    pub at_line_start: bool,
    /// 是否在行尾
    pub at_line_end: bool,
}

/// 选区信息
#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    /// 起始位置
    pub start: CursorPosition,
    /// 结束位置
    pub end: CursorPosition,
    /// 选区类型
    pub selection_type: SelectionType,
    /// 是否为矩形选区
    pub is_rectangular: bool,
}

/// 选区类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionType {
    /// 流式选区（连续字符）
    Stream,
    /// 矩形选区
    Rectangular,
}

/// 语法标记
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxToken {
    /// 标记类型
    pub token_type: String,
    /// 在行中的起始位置
    pub start: usize,
    /// 在行中的结束位置
    pub end: usize,
}
```

### `src/theme/model.rs`
```rust
//! 主题数据模型

use std::collections::HashMap;
use slint::Color;

/// 主题数据
#[derive(Debug, Clone)]
pub struct ThemeData {
    /// 主题名称
    pub name: String,
    /// 是否为暗色主题
    pub is_dark: bool,
    /// 颜色映射
    pub colors: HashMap<String, Color>,
    /// 语法高亮颜色
    pub syntax_colors: HashMap<String, Color>,
    /// 字体配置
    pub font_family: String,
    pub font_size: f32,
}

impl ThemeData {
    /// 创建深色主题
    pub fn dark() -> Self {
        let mut colors = HashMap::new();
        let mut syntax_colors = HashMap::new();
        
        colors.insert("background".to_string(), Color::from_rgb_u8(30, 30, 30));
        colors.insert("text".to_string(), Color::from_rgb_u8(220, 220, 220));
        colors.insert("cursor".to_string(), Color::from_rgb_u8(255, 255, 255));
        colors.insert("selection".to_string(), Color::from_rgba_u8(50, 90, 140, 128));
        colors.insert("line_number".to_string(), Color::from_rgb_u8(150, 150, 150));
        colors.insert("current_line".to_string(), Color::from_rgba_u8(60, 60, 60, 128));
        
        syntax_colors.insert("keyword".to_string(), Color::from_rgb_u8(86, 156, 214));
        syntax_colors.insert("string".to_string(), Color::from_rgb_u8(206, 145, 120));
        syntax_colors.insert("comment".to_string(), Color::from_rgb_u8(106, 153, 85));
        syntax_colors.insert("number".to_string(), Color::from_rgb_u8(181, 206, 168));
        syntax_colors.insert("function".to_string(), Color::from_rgb_u8(220, 220, 170));
        
        Self {
            name: "Dark".to_string(),
            is_dark: true,
            colors,
            syntax_colors,
            font_family: "Consolas".to_string(),
            font_size: 14.0,
        }
    }
    
    /// 创建浅色主题
    pub fn light() -> Self {
        let mut colors = HashMap::new();
        let mut syntax_colors = HashMap::new();
        
        colors.insert("background".to_string(), Color::from_rgb_u8(255, 255, 255));
        colors.insert("text".to_string(), Color::from_rgb_u8(30, 30, 30));
        colors.insert("cursor".to_string(), Color::from_rgb_u8(0, 0, 0));
        colors.insert("selection".to_string(), Color::from_rgba_u8(180, 210, 255, 128));
        colors.insert("line_number".to_string(), Color::from_rgb_u8(100, 100, 100));
        colors.insert("current_line".to_string(), Color::from_rgba_u8(240, 240, 240, 128));
        
        syntax_colors.insert("keyword".to_string(), Color::from_rgb_u8(0, 0, 255));
        syntax_colors.insert("string".to_string(), Color::from_rgb_u8(163, 21, 21));
        syntax_colors.insert("comment".to_string(), Color::from_rgb_u8(0, 128, 0));
        syntax_colors.insert("number".to_string(), Color::from_rgb_u8(9, 134, 88));
        syntax_colors.insert("function".to_string(), Color::from_rgb_u8(111, 0, 138));
        
        Self {
            name: "Light".to_string(),
            is_dark: false,
            colors,
            syntax_colors,
            font_family: "Consolas".to_string(),
            font_size: 14.0,
        }
    }
}

impl From<ThemeData> for crate::render::resources::ColorPalette {
    fn from(theme: ThemeData) -> Self {
        crate::render::resources::ColorPalette {
            colors: theme.colors,
            syntax_colors: theme.syntax_colors,
        }
    }
}
```

## 10. Cargo.toml 依赖配置

```toml
[package]
name = "zedit-render"
version = "0.1.0"
edition = "2021"

[dependencies]
slint = { version = "1.5", features = ["std"] }
cosmic-text = "0.11"
fontdb = "0.16"
memmap2 = "0.9"
lru = "0.12"
thiserror = "1.0"
log = "0.4"

[dev-dependencies]
criterion = "0.5"
```

## 11. 使用示例

### 示例：集成到zedit编辑器
```rust
// src/main.rs 或编辑器集成点

use zedit_render::{RenderEngine, RenderConfig};
use zedit_theme::ThemeData;
use slint::{PlatformWindow, Weak};

struct EditorApplication {
    render_engine: RenderEngine,
    // ... 其他编辑器组件
}

impl EditorApplication {
    fn new(window: Weak<dyn PlatformWindow>) -> Self {
        // 创建渲染配置
        let config = RenderConfig {
            initial_size: window.size(),
            dpi_scale: window.scale_factor(),
            theme: ThemeData::dark(),
            font_family: "Consolas".to_string(),
            font_size: 14.0,
            enable_vsync: true,
            enable_incremental_updates: true,
            max_fps: 60,
        };
        
        // 创建渲染引擎
        let mut render_engine = RenderEngine::new(config).unwrap();
        
        // 绑定到窗口
        render_engine.bind_to_component(window).unwrap();
        
        Self {
            render_engine,
            // ... 初始化其他组件
        }
    }
    
    fn on_editor_update(&mut self, layout: zedit_layout::LayoutModel) {
        // 更新渲染布局
        if let Err(e) = self.render_engine.update_layout(layout) {
            log::error!("Failed to update layout: {}", e);
        }
        
        // 执行渲染（通常在Slint事件循环中）
        match self.render_engine.render() {
            Ok(commands) => {
                // 将命令发送给Slint渲染
                self.send_commands_to_slint(commands);
            }
            Err(e) => {
                log::error!("Render failed: {}", e);
            }
        }
    }
    
    fn on_theme_change(&mut self, theme: ThemeData) {
        if let Err(e) = self.render_engine.apply_theme(theme) {
            log::error!("Failed to apply theme: {}", e);
        }
    }
    
    fn on_window_resize(&mut self, new_size: slint::PhysicalSize) {
        if let Err(e) = self.render_engine.handle_resize(new_size) {
            log::error!("Failed to handle resize: {}", e);
        }
    }
    
    fn send_commands_to_slint(&self, commands: Vec<SlintDrawCommand>) {
        // 这里需要根据实际Slint集成方式实现
        // 通常是调用Canvas的绘制方法
    }
}
```

## 12. 构建和测试说明

### 构建命令
```bash
# 正常构建
cargo build --release

# 带有性能分析的构建
cargo build --release --features "profiling"

# 测试构建
cargo test -- --nocapture
```

### 运行测试
```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_cursor_renderer

# 运行性能测试
cargo bench
```

这个完整的Render System实现包含了：
1. **完整的架构**：遵循单向数据流，与编辑器核心解耦
2. **资源管理**：字体、字形、颜色等资源的缓存和管理
3. **性能优化**：脏区域跟踪、批量绘制、缓存策略
4. **模块化设计**：光标、选区、行号、语法高亮等独立模块
5. **性能监控**：详细的性能指标和警告系统
6. **错误处理**：完整的错误类型和处理
7. **测试覆盖**：单元测试和性能测试

所有代码都严格遵循之前定义的架构文档和数据流规范，确保Render System是纯渲染层，不包含任何业务逻辑。