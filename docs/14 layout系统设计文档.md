# 📚 **Layout系统文档体系**

```
docs/
├── 架构层/
│   └── layout-design.md      # 设计理念和架构位置
├── 实现层/
│   └── layout-implementation.md  # 实现细节和规范
├── API层/
│   └── layout-api.md         # API参考和使用示例
└── 优化层/
    └── layout-optimizations.md  # 性能优化记录
```

---

## 1. **架构层文档**：Layout设计理念

```markdown
# Layout系统架构设计文档

## 📋 文档信息
- **版本**：1.0
- **状态**：已冻结
- **关联文档**：[架构宪法] · [数据流规范] · [ViewModel设计]

## 🎯 设计目标

### 核心定位
Layout系统是zedit编辑器的**几何计算引擎**，负责：
1. **文本布局**：将逻辑文本转换为精确的物理坐标和几何信息
2. **坐标转换**：提供逻辑位置 ↔ 物理位置的双向映射
3. **换行处理**：根据视口宽度动态计算文本换行
4. **字形计算**：计算每个字符/字形的精确位置和边界

### 设计哲学
1. **纯计算**：不持有状态，只进行几何计算
2. **缓存友好**：计算结果可缓存和复用
3. **增量更新**：基于脏区的最小化重新计算
4. **精度保证**：支持亚像素精度的文本渲染

## 🏗️ 架构位置

### 在系统中的作用
```
┌─────────────────┐
│   ViewModel     │  ← 渲染数据（文本+装饰）
├─────────────────┤
│   Layout        │  ← 本文档对象（几何计算）
├─────────────────┤
│   Render System │  ← 渲染指令（像素坐标）
└─────────────────┘
```

### 数据流角色
- **输入**：`ViewModelSnapshot`（渲染数据）、`LayoutConfig`（布局配置）
- **输出**：`LayoutResult`（几何信息）、`PhysicalPosition`（像素坐标）
- **特点**：**无状态计算**，相同的输入总是产生相同的输出

## 📊 核心设计决策

### 已冻结决策
1. **分层布局模型**：基础文本层 + 装饰叠加层分离计算
2. **懒计算策略**：位置和边界信息按需计算
3. **缓存系统**：高频计算结果多级缓存
4. **增量更新**：只重新计算变化的区域
5. **物理坐标系**：基于实际像素，支持HiDPI

### 与其他组件的关系
| 组件 | 与Layout的关系 | 通信方式 |
|------|----------------|----------|
| ViewModel | 数据源，提供文本和装饰 | 接收快照，计算布局 |
| Render System | 消费者，使用布局结果 | 传递完整LayoutResult |
| Font System | 依赖，提供字形度量 | 通过FontMetrics接口 |
| Viewport | 协同，提供视口尺寸 | 接收视口宽度进行换行 |

## 🔧 设计约束

### 必须遵守的约束
1. **无状态性**：Layout实例不持有可变状态，所有计算纯函数化
2. **确定性**：相同的输入必须产生相同的输出
3. **性能边界**：布局计算时间与可见内容大小线性相关
4. **精度要求**：支持亚像素精度，确保文本清晰度

### 性能目标
| 操作 | 目标延迟 | 备注 |
|------|----------|------|
| 单行布局计算 | <0.1ms | 简单文本 |
| 视口布局计算（100行） | <5ms | 含换行和缓存 |
| 坐标转换（逻辑→物理） | <0.01ms | 缓存命中时 |
| 增量布局更新（10行） | <1ms | 基于脏区 |

## 📈 演进原则

### 允许的演进
1. **算法优化**：改进换行算法、缓存策略
2. **字形缓存扩展**：支持更多字体和样式
3. **布局特性扩展**：添加垂直文本、ruby注音等
4. **国际化增强**：改进复杂脚本处理

### 禁止的演进
1. **状态持有**：不添加任何编辑状态或UI状态
2. **渲染耦合**：不包含特定渲染后端的代码
3. **业务逻辑**：不包含编辑或装饰逻辑
4. **外部依赖**：不依赖网络或文件系统

## 🔗 核心概念定义

### 关键术语
| 术语 | 定义 |
|------|------|
| 布局上下文 | 包含字体、DPI、配置的计算环境 |
| 布局行 | 经过换行处理后的物理行 |
| 字形位置 | 字符在屏幕上的精确位置和大小 |
| 布局缓存 | 缓存布局计算结果的数据结构 |
| 脏区 | 需要重新布局的区域 |
| 换行点 | 文本需要换行的位置 |

### 坐标系系统
1. **逻辑坐标系**：文档内的行列位置（行号、列号）
2. **物理坐标系**：屏幕像素坐标（x, y，以像素为单位）
3. **布局坐标系**：相对于布局原点的坐标（用于计算）
4. **视口坐标系**：相对于视口左上角的坐标（用于渲染）

---

*本文档定义了Layout系统的架构角色和设计约束，所有实现必须遵守。*
```

---

## 2. **实现层文档**：Layout实现细节

```markdown
# Layout系统实现规范文档

## 📋 文档信息
- **版本**：1.0
- **状态**：实施指南（可优化）
- **关联代码**：`src/core/layout/`

## 🏗️ 核心数据结构

### 1. LayoutContext（布局上下文）
```rust
struct LayoutContext {
    // 字体配置
    font_metrics: FontMetrics,
    font_cache: FontCache,
    
    // 显示配置
    dpi_scale: f32,
    pixel_ratio: f32,
    
    // 布局配置
    config: LayoutConfig,
    
    // 缓存系统
    glyph_cache: GlyphCache,
    line_layout_cache: LineLayoutCache,
    coordinate_cache: CoordinateCache,
    
    // 统计信息
    stats: LayoutStats,
}
```

**设计考虑**：
- **上下文隔离**：每个文档/视图有独立的上下文
- **字体共享**：`FontCache`在多个上下文间共享
- **缓存分层**：字形、行布局、坐标分别缓存
- **配置驱动**：所有行为通过配置控制

### 2. LayoutResult（布局结果）
```rust
struct LayoutResult {
    // 标识信息
    id: LayoutId,
    snapshot_id: SnapshotId,
    
    // 布局范围
    viewport_range: LineRange,
    
    // 布局行数据
    layout_lines: Arc<[LayoutLine]>,
    
    // 几何信息
    total_height: f32,
    max_line_width: f32,
    
    // 缓存信息
    cached_glyphs: usize,
    hit_rate: f32,
    
    // 元数据
    metadata: LayoutMetadata,
}

struct LayoutLine {
    // 源信息
    source_line: LineHandle,          // 指向ViewModel中的行
    line_number: usize,               // 逻辑行号
    
    // 布局信息
    fragments: Arc<[LayoutFragment]>, // 布局片段（可能被换行分割）
    y_position: f32,                  // 垂直位置（从文档顶部）
    height: f32,                      // 行高
    
    // 换行信息
    is_wrapped: bool,                 // 是否被换行
    wrap_count: usize,                // 换行次数
    wrapped_lines: Option<Arc<[WrappedLine]>>, // 换行后的子行
    
    // 缓存键
    layout_hash: u64,
}

struct LayoutFragment {
    // 文本信息
    visual_span: Arc<VisualSpan>,     // 来源的视觉片段
    text: Arc<str>,                   // 片段文本
    
    // 几何信息
    x_position: f32,                  // 水平位置（从行首）
    width: f32,                       // 片段宽度
    ascent: f32,                      // 上伸高度
    descent: f32,                     // 下伸高度
    
    // 字形信息
    glyphs: Arc<[PositionedGlyph]>,   // 定位后的字形
    cluster_map: Arc<[usize]>,        // 字符到字形的映射
    
    // 视觉属性
    visual_attrs: VisualAttributes,
}
```

### 3. PositionedGlyph（定位字形）
```rust
struct PositionedGlyph {
    // 字形标识
    glyph_id: GlyphId,
    font_id: FontId,
    
    // 位置信息
    x: f32,                           // 相对于片段的x坐标
    y: f32,                           // 基线偏移
    advance: f32,                     // 前进宽度
    
    // 边界框
    bounds: Option<Rect>,             // 字形边界（可选）
    
    // 文本信息
    cluster_index: usize,             // 在文本簇中的索引
    char_index: usize,                // 在字符串中的字符索引
}
```

### 4. LayoutCache（布局缓存）
```rust
struct LayoutCache {
    // 字形度量缓存
    glyph_metrics: LruCache<GlyphKey, GlyphMetrics>,
    
    // 行布局缓存
    line_layouts: LruCache<LineLayoutKey, Arc<LayoutLine>>,
    
    // 坐标转换缓存
    coord_mapping: LruCache<CoordKey, PhysicalPosition>,
    
    // 统计信息
    stats: CacheStats,
}

struct GlyphKey {
    font_id: FontId,
    glyph_id: GlyphId,
    font_size: f32,
    dpi_scale: f32,
    hinting: HintingMode,
}

struct LineLayoutKey {
    line_hash: u64,                   // 行内容哈希
    max_width: f32,                   // 最大宽度（影响换行）
    tab_width: usize,                 // 制表符宽度
    font_config_hash: u64,            // 字体配置哈希
}
```

## ⚙️ 核心算法实现

### 1. 文本布局算法
**位置**：`text_layout.rs` - `TextLayoutEngine::layout_line()`

**布局流程**：
```rust
impl TextLayoutEngine {
    fn layout_line(
        &mut self,
        context: &mut LayoutContext,
        line: &RenderedLine,
        max_width: Option<f32>,
    ) -> LayoutLine {
        // 1. 检查缓存
        let cache_key = self.compute_line_cache_key(line, max_width, context);
        if let Some(cached) = context.line_layout_cache.get(&cache_key) {
            context.stats.record_cache_hit();
            return cached.clone();
        }
        
        // 2. 布局视觉片段
        let mut fragments = Vec::new();
        let mut current_x = 0.0;
        
        for visual_span in line.visual_spans() {
            let fragment = self.layout_fragment(
                context,
                visual_span,
                current_x,
                max_width,
            );
            
            fragments.push(fragment);
            current_x += fragment.width;
        }
        
        // 3. 处理换行（如果需要）
        let (fragments, wrapped_lines) = if let Some(max_width) = max_width {
            self.handle_line_wrapping(fragments, max_width, context)
        } else {
            (fragments, None)
        };
        
        // 4. 计算行高和位置
        let (ascent, descent) = self.compute_line_metrics(&fragments, context);
        let height = ascent + descent + context.config.line_spacing;
        
        // 5. 创建布局行
        let layout_line = LayoutLine {
            source_line: LineHandle::from(line),
            line_number: line.line_number(),
            fragments: fragments.into(),
            y_position: 0.0, // 将在布局过程中设置
            height,
            is_wrapped: wrapped_lines.is_some(),
            wrap_count: wrapped_lines.as_ref().map_or(0, |w| w.len()),
            wrapped_lines: wrapped_lines.map(Arc::from),
            layout_hash: cache_key.line_hash,
        };
        
        // 6. 缓存结果
        context.line_layout_cache.put(cache_key, Arc::new(layout_line.clone()));
        
        layout_line
    }
    
    fn layout_fragment(
        &mut self,
        context: &mut LayoutContext,
        visual_span: &VisualSpan,
        start_x: f32,
        max_width: Option<f32>,
    ) -> LayoutFragment {
        // 获取字体
        let font = context.get_font_for_attrs(&visual_span.visual_attrs());
        
        // 布局文本
        let glyphs = self.shape_text(
            context,
            &font,
            visual_span.text(),
            &visual_span.visual_attrs(),
        );
        
        // 计算宽度和位置
        let (width, positioned_glyphs) = self.position_glyphs(&glyphs, &font);
        
        // 检查是否超出最大宽度
        if let Some(max_width) = max_width {
            if start_x + width > max_width {
                // 需要截断或换行处理
                return self.handle_overflow(
                    visual_span,
                    start_x,
                    max_width,
                    &font,
                    context,
                );
            }
        }
        
        // 计算垂直度量
        let (ascent, descent) = font.metrics().ascent_descent(
            visual_span.visual_attrs().font_size.unwrap_or(context.config.font_size),
        );
        
        LayoutFragment {
            visual_span: Arc::new(visual_span.clone()),
            text: visual_span.text().into(),
            x_position: start_x,
            width,
            ascent,
            descent,
            glyphs: positioned_glyphs.into(),
            cluster_map: self.build_cluster_map(visual_span.text(), &positioned_glyphs),
            visual_attrs: visual_span.visual_attrs(),
        }
    }
}
```

### 2. 换行处理算法
**位置**：`line_wrapping.rs` - `LineWrapper::wrap_line()`

**换行策略**：
```rust
impl LineWrapper {
    fn wrap_line(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        let total_width: f32 = fragments.iter().map(|f| f.width).sum();
        
        // 1. 如果总宽度不超过最大宽度，不需要换行
        if total_width <= max_width {
            return (fragments, None);
        }
        
        // 2. 检查换行策略
        match context.config.wrap_mode {
            WrapMode::None => {
                // 不换行，允许水平滚动
                (fragments, None)
            }
            WrapMode::Word => {
                // 按单词换行
                self.wrap_by_word(fragments, max_width, context)
            }
            WrapMode::Character => {
                // 按字符换行
                self.wrap_by_character(fragments, max_width, context)
            }
            WrapMode::Whitespace => {
                // 在空白处换行
                self.wrap_at_whitespace(fragments, max_width, context)
            }
        }
    }
    
    fn wrap_by_word(
        &self,
        fragments: Vec<LayoutFragment>,
        max_width: f32,
        context: &LayoutContext,
    ) -> (Vec<LayoutFragment>, Option<Arc<[WrappedLine]>>) {
        let mut wrapped_lines = Vec::new();
        let mut current_line = Vec::new();
        let mut current_width = 0.0;
        
        for fragment in fragments {
            // 如果片段本身超过最大宽度，需要进一步分割
            if fragment.width > max_width {
                let sub_fragments = self.split_fragment_by_word(&fragment, max_width, context);
                
                for sub_fragment in sub_fragments {
                    if current_width + sub_fragment.width > max_width {
                        // 当前行已满，开始新行
                        if !current_line.is_empty() {
                            wrapped_lines.push(WrappedLine {
                                fragments: current_line.into(),
                                width: current_width,
                            });
                            current_line = Vec::new();
                            current_width = 0.0;
                        }
                        
                        // 将子片段放入新行
                        current_line.push(sub_fragment.clone());
                        current_width = sub_fragment.width;
                    } else {
                        // 添加到当前行
                        current_line.push(sub_fragment.clone());
                        current_width += sub_fragment.width;
                    }
                }
            } else if current_width + fragment.width > max_width {
                // 当前行已满，开始新行
                if !current_line.is_empty() {
                    wrapped_lines.push(WrappedLine {
                        fragments: current_line.into(),
                        width: current_width,
                    });
                    current_line = Vec::new();
                    current_width = 0.0;
                }
                
                // 将片段放入新行
                current_line.push(fragment);
                current_width = fragment.width;
            } else {
                // 添加到当前行
                current_line.push(fragment);
                current_width += fragment.width;
            }
        }
        
        // 添加最后一行
        if !current_line.is_empty() {
            wrapped_lines.push(WrappedLine {
                fragments: current_line.into(),
                width: current_width,
            });
        }
        
        if wrapped_lines.len() > 1 {
            // 重建fragments为第一行
            let first_line_fragments = wrapped_lines[0].fragments.clone();
            (first_line_fragments.to_vec(), Some(wrapped_lines.into()))
        } else {
            // 没有换行或只有一行
            (fragments, None)
        }
    }
    
    fn split_fragment_by_word(
        &self,
        fragment: &LayoutFragment,
        max_width: f32,
        context: &LayoutContext,
    ) -> Vec<LayoutFragment> {
        let text = fragment.text.as_str();
        let font = context.get_font_for_attrs(&fragment.visual_attrs);
        
        // 查找单词边界
        let mut splits = Vec::new();
        let mut start = 0;
        
        while start < text.len() {
            // 查找下一个单词边界
            let mut end = start;
            let mut word_width = 0.0;
            
            while end < text.len() {
                let next_char = &text[start..=end];
                let char_width = font.measure_text(next_char, fragment.visual_attrs.font_size);
                
                if word_width + char_width > max_width {
                    // 超过宽度，在上一个边界处分割
                    break;
                }
                
                word_width += char_width;
                end += next_char.chars().last().map_or(1, |c| c.len_utf8());
                
                // 检查是否是单词边界
                if end < text.len() && self.is_word_boundary(&text, end) {
                    // 找到单词边界，可以在此分割
                    break;
                }
            }
            
            if end == start {
                // 没有找到合适的分割点，强制在字符边界分割
                end = self.find_char_boundary(&text, start + 1);
            }
            
            // 创建子片段
            let sub_text = &text[start..end];
            let sub_fragment = self.create_sub_fragment(fragment, sub_text, start, end, context);
            splits.push(sub_fragment);
            
            start = end;
        }
        
        splits
    }
}
```

### 3. 坐标转换算法
**位置**：`coordinate_mapping.rs` - `CoordinateMapper::logical_to_physical()`

**转换算法**：
```rust
impl CoordinateMapper {
    fn logical_to_physical(
        &self,
        context: &LayoutContext,
        layout_result: &LayoutResult,
        logical_pos: LogicalPosition,
    ) -> Option<PhysicalPosition> {
        // 1. 检查缓存
        let cache_key = CoordKey::from_logical(logical_pos, layout_result.id);
        if let Some(cached) = context.coordinate_cache.get(&cache_key) {
            context.stats.record_coord_cache_hit();
            return Some(cached);
        }
        
        // 2. 查找对应的布局行
        let layout_line = self.find_layout_line(layout_result, logical_pos.line)?;
        
        // 3. 计算列偏移
        let column_offset = self.column_to_x_offset(
            context,
            layout_line,
            logical_pos.column,
        )?;
        
        // 4. 计算垂直位置
        let y_position = self.line_to_y_position(layout_result, logical_pos.line)?;
        
        // 5. 组合坐标
        let physical_pos = PhysicalPosition {
            x: column_offset,
            y: y_position,
        };
        
        // 6. 缓存结果
        context.coordinate_cache.put(cache_key, physical_pos);
        
        Some(physical_pos)
    }
    
    fn find_layout_line(
        &self,
        layout_result: &LayoutResult,
        line_number: usize,
    ) -> Option<&LayoutLine> {
        // 查找逻辑行对应的布局行
        // 需要考虑换行情况
        
        for layout_line in &layout_result.layout_lines {
            if layout_line.line_number == line_number {
                return Some(layout_line);
            }
            
            // 如果该行被换行，检查换行后的子行
            if let Some(wrapped_lines) = &layout_line.wrapped_lines {
                for wrapped_line in wrapped_lines.iter() {
                    // 每个换行子行在逻辑上属于同一行
                    // 这里需要特殊的逻辑来处理...
                }
            }
        }
        
        None
    }
    
    fn column_to_x_offset(
        &self,
        context: &LayoutContext,
        layout_line: &LayoutLine,
        column: usize,
    ) -> Option<f32> {
        let mut current_x = 0.0;
        let mut chars_processed = 0;
        
        for fragment in layout_line.fragments.iter() {
            let fragment_chars = fragment.text.chars().count();
            
            if chars_processed + fragment_chars > column {
                // 目标列在当前片段内
                let offset_in_fragment = column - chars_processed;
                let sub_text = fragment.text
                    .chars()
                    .take(offset_in_fragment)
                    .collect::<String>();
                
                let font = context.get_font_for_attrs(&fragment.visual_attrs);
                let sub_width = font.measure_text(&sub_text, fragment.visual_attrs.font_size);
                
                return Some(current_x + sub_width);
            }
            
            current_x += fragment.width;
            chars_processed += fragment_chars;
        }
        
        // 列超出行的长度，返回行尾位置
        Some(current_x)
    }
    
    fn line_to_y_position(
        &self,
        layout_result: &LayoutResult,
        line_number: usize,
    ) -> Option<f32> {
        // 累积前面所有行的高度
        let mut y = 0.0;
        
        for layout_line in &layout_result.layout_lines {
            if layout_line.line_number == line_number {
                return Some(y + layout_line.ascent()); // 返回基线位置
            }
            
            y += layout_line.height;
            
            // 如果该行被换行，需要添加所有子行的高度
            if let Some(wrapped_lines) = &layout_line.wrapped_lines {
                for wrapped_line in wrapped_lines.iter() {
                    y += wrapped_line.height;
                }
            }
        }
        
        None
    }
}
```

### 4. 增量布局算法
**位置**：`incremental_layout.rs` - `IncrementalLayoutEngine::update()`

**增量更新策略**：
```rust
impl IncrementalLayoutEngine {
    fn update_layout(
        &mut self,
        context: &mut LayoutContext,
        old_result: &LayoutResult,
        new_snapshot: &ViewModelSnapshot,
        dirty_range: Option<LineRange>,
        config: &LayoutConfig,
    ) -> LayoutResult {
        // 1. 确定需要重新布局的范围
        let layout_range = self.determine_layout_range(
            old_result,
            new_snapshot,
            dirty_range,
            config,
        );
        
        // 2. 如果范围太大，进行全量布局
        if self.should_full_layout(layout_range, old_result, config) {
            return self.full_layout(context, new_snapshot, config);
        }
        
        // 3. 增量布局
        let mut new_lines = Vec::with_capacity(old_result.layout_lines.len());
        
        for (i, old_line) in old_result.layout_lines.iter().enumerate() {
            let line_number = old_line.line_number;
            
            if layout_range.contains(line_number) {
                // 需要重新布局的行
                let new_line = context.layout_line(
                    new_snapshot.line_by_number(line_number).unwrap(),
                    config.max_line_width,
                );
                new_lines.push(new_line);
            } else {
                // 可以复用的行（但可能需要调整垂直位置）
                let mut reused_line = old_line.clone();
                
                // 调整垂直位置（因为前面可能有行被重新布局）
                if i > 0 {
                    let prev_height: f32 = new_lines.iter()
                        .filter(|l| l.line_number < line_number)
                        .map(|l| l.height)
                        .sum();
                    
                    reused_line.y_position = prev_height;
                }
                
                new_lines.push(reused_line);
            }
        }
        
        // 4. 计算总高度和最大宽度
        let total_height: f32 = new_lines.iter().map(|l| l.height).sum();
        let max_line_width = new_lines.iter()
            .map(|l| self.compute_line_width(l))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        
        // 5. 创建新的布局结果
        LayoutResult {
            id: LayoutId::new(),
            snapshot_id: new_snapshot.id(),
            viewport_range: new_snapshot.viewport_range(),
            layout_lines: new_lines.into(),
            total_height,
            max_line_width,
            cached_glyphs: context.glyph_cache.len(),
            hit_rate: context.stats.cache_hit_rate(),
            metadata: LayoutMetadata {
                source: LayoutSource::Incremental,
                build_time: std::time::Instant::now(),
                lines_updated: layout_range.len(),
                total_lines: new_lines.len(),
            },
        }
    }
    
    fn determine_layout_range(
        &self,
        old_result: &LayoutResult,
        new_snapshot: &ViewModelSnapshot,
        dirty_range: Option<LineRange>,
        config: &LayoutConfig,
    ) -> LineRange {
        // 基本范围：显式指定的脏区
        let base_range = dirty_range.unwrap_or_else(|| LineRange::empty());
        
        // 扩展范围：考虑换行影响
        let expanded_range = self.expand_for_wrapping(
            base_range,
            old_result,
            new_snapshot,
            config,
        );
        
        expanded_range
    }
    
    fn expand_for_wrapping(
        &self,
        base_range: LineRange,
        old_result: &LayoutResult,
        new_snapshot: &ViewModelSnapshot,
        config: &LayoutConfig,
    ) -> LineRange {
        let mut expanded = base_range;
        
        // 如果配置变化可能影响换行，需要重新布局更多行
        if config.max_line_width != old_result.max_line_width {
            // 行宽变化可能影响所有行的换行
            return new_snapshot.viewport_range();
        }
        
        // 检查字体或DPI变化
        if config.font_size != old_result.metadata.font_size ||
           config.dpi_scale != old_result.metadata.dpi_scale {
            // 字体或DPI变化会影响所有行
            return new_snapshot.viewport_range();
        }
        
        expanded
    }
}
```

## 🧩 子系统实现

### 1. FontManager（字体管理器）
**位置**：`font_manager.rs`
**职责**：管理字体加载、缓存和度量查询

**关键特性**：
- **字体缓存**：按字体系列、大小、样式缓存字体实例
- **字形缓存**：缓存字形度量和形状信息
- **字体回退**：支持Unicode回退字体链
- **度量查询**：提供精确的字体度量信息

### 2. GlyphCache（字形缓存）
**位置**：`glyph_cache.rs`
**设计**：多层字形缓存系统

**缓存层级**：
1. **度量缓存**：字形宽度、高度等度量信息
2. **形状缓存**：字形的轮廓或位图表示
3. **整行缓存**：整行文本的预渲染结果

### 3. LineCache（行布局缓存）
**位置**：`line_cache.rs`
**职责**：缓存行布局结果，支持增量更新

**缓存策略**：
- **内容哈希**：基于行文本和样式计算哈希键
- **配置感知**：考虑字体、DPI、行宽等配置
- **LRU淘汰**：限制缓存大小，淘汰最旧条目
- **版本管理**：跟踪缓存条目的版本

### 4. CoordinateCache（坐标缓存）
**位置**：`coordinate_cache.rs`
**设计**：缓存逻辑→物理坐标映射，加速坐标转换

**优化策略**：
- **热点缓存**：特别缓存光标位置和选区边界
- **区域缓存**：缓存连续区域的坐标映射
- **预测预加载**：基于用户行为预加载可能需要的坐标

## 🧪 测试策略

### 单元测试覆盖
```rust
#[cfg(test)]
mod tests {
    // 1. 基础布局测试
    test_single_line_layout()
    test_multi_fragment_layout()
    test_unicode_text_layout()
    
    // 2. 换行测试
    test_word_wrapping()
    test_character_wrapping()
    test_mixed_wrapping_scenarios()
    
    // 3. 坐标转换测试
    test_logical_to_physical_mapping()
    test_physical_to_logical_mapping()
    test_coordinate_consistency()
    
    // 4. 增量布局测试
    test_incremental_line_update()
    test_viewport_range_change()
    test_font_config_change()
}
```

### 性能测试
```rust
#[bench]
fn bench_text_layout_performance(b: &mut Bencher) {
    let context = create_test_layout_context();
    let lines = create_test_lines(100);
    
    b.iter(|| {
        for line in &lines {
            context.layout_line(line, None);
        }
    });
}

#[bench]
fn bench_coordinate_conversion(b: &mut Bencher) {
    let context = create_test_layout_context();
    let layout_result = create_test_layout_result();
    
    b.iter(|| {
        for i in 0..100 {
            let logical_pos = LogicalPosition::new(i, i % 50);
            context.logical_to_physical(&layout_result, logical_pos);
        }
    });
}

#[bench]
fn bench_incremental_update(b: &mut Bencher) {
    let mut engine = IncrementalLayoutEngine::new();
    let context = create_test_layout_context();
    let old_result = create_test_layout_result();
    let new_snapshot = create_modified_snapshot();
    
    b.iter(|| {
        engine.update_layout(
            &mut context,
            &old_result,
            &new_snapshot,
            Some(LineRange::new(10, 20)),
            &LayoutConfig::default(),
        );
    });
}
```

### 可视化测试
```rust
// 布局可视化测试工具
fn visualize_layout_diff(
    old_result: &LayoutResult,
    new_result: &LayoutResult,
) -> LayoutDiffVisualization {
    let delta = LayoutDelta::compute(old_result, new_result);
    
    let mut visualization = String::new();
    visualization.push_str("<div class='layout-diff'>\n");
    
    for line_diff in &delta.line_diffs {
        visualization.push_str(&format!(
            "<div class='line-diff line-{}'>\n",
            line_diff.line_number
        ));
        
        visualization.push_str("  <div class='old'>\n");
        for fragment in &line_diff.old_fragments {
            visualization.push_str(&format!(
                "    <span style='left: {}px; width: {}px; {}'>{}</span>\n",
                fragment.x_position,
                fragment.width,
                fragment.visual_attrs.to_css(),
                escape_html(&fragment.text)
            ));
        }
        visualization.push_str("  </div>\n");
        
        visualization.push_str("  <div class='new'>\n");
        for fragment in &line_diff.new_fragments {
            visualization.push_str(&format!(
                "    <span style='left: {}px; width: {}px; {}'>{}</span>\n",
                fragment.x_position,
                fragment.width,
                fragment.visual_attrs.to_css(),
                escape_html(&fragment.text)
            ));
        }
        visualization.push_str("  </div>\n");
        
        visualization.push_str("</div>\n");
    }
    
    visualization.push_str("</div>");
    
    LayoutDiffVisualization::Html(visualization)
}
```

## 🔄 维护指南

### 代码组织原则
1. **纯函数核心**：核心布局算法是纯函数，易于测试
2. **缓存透明**：缓存机制对上层透明，可配置和监控
3. **配置驱动**：所有行为通过配置控制，无硬编码
4. **错误安全**：无效输入有明确的错误处理

### 监控指标
```rust
struct LayoutMetrics {
    // 性能指标
    layout_time_per_line_ms: f64,
    cache_hit_rate: f64,
    glyph_cache_efficiency: f64,
    
    // 质量指标
    layout_accuracy: f64,          // 坐标转换的精度
    wrapping_quality: f64,         // 换行质量评分
    
    // 资源使用
    memory_usage_bytes: usize,
    cache_size_items: usize,
    
    // 用户感知指标
    frame_drops: usize,
    layout_jank_count: usize,
}

impl LayoutMetrics {
    fn check_health(&self) -> Option<HealthWarning> {
        if self.layout_time_per_line_ms > 0.5 {
            Some(HealthWarning::SlowLayoutPerformance)
        } else if self.cache_hit_rate < 0.7 {
            Some(HealthWarning::LowCacheEfficiency)
        } else if self.memory_usage_bytes > 100 * 1024 * 1024 {
            Some(HealthWarning::HighMemoryUsage)
        } else {
            None
        }
    }
}
```

### 调试支持
```rust
// 布局调试信息
impl LayoutResult {
    fn debug_info(&self) -> LayoutDebugInfo {
        LayoutDebugInfo {
            id: self.id,
            snapshot_id: self.snapshot_id,
            line_count: self.layout_lines.len(),
            total_height: self.total_height,
            max_line_width: self.max_line_width,
            cached_glyphs: self.cached_glyphs,
            hit_rate: self.hit_rate,
            metadata: self.metadata.clone(),
        }
    }
    
    fn visualize(&self) -> LayoutVisualization {
        let mut svg = String::new();
        svg.push_str(&format!(
            "<svg width='{}' height='{}' xmlns='http://www.w3.org/2000/svg'>\n",
            self.max_line_width + 20.0,
            self.total_height + 20.0
        ));
        
        let mut y = 10.0;
        for line in &self.layout_lines {
            let mut x = 10.0;
            
            svg.push_str(&format!(
                "<rect x='{}' y='{}' width='{}' height='{}' fill='#f0f0f0' />\n",
                x - 2.0, y - 2.0, self.max_line_width + 4.0, line.height + 4.0
            ));
            
            for fragment in &line.fragments {
                svg.push_str(&format!(
                    "<rect x='{}' y='{}' width='{}' height='{}' fill='#e0e0e0' />\n",
                    x, y + fragment.ascent, fragment.width, fragment.descent - fragment.ascent
                ));
                
                svg.push_str(&format!(
                    "<text x='{}' y='{}' font-size='12px'>{}</text>\n",
                    x, y + fragment.ascent, escape_html(&fragment.text)
                ));
                
                x += fragment.width;
            }
            
            y += line.height;
        }
        
        svg.push_str("</svg>");
        LayoutVisualization::Svg(svg)
    }
}

// 布局问题诊断
fn diagnose_layout_issue(
    context: &LayoutContext,
    layout_result: &LayoutResult,
    issue: LayoutIssue,
) -> DiagnosisReport {
    let mut report = DiagnosisReport::new();
    
    match issue {
        LayoutIssue::SlowLayout => {
            report.add_section("Performance Analysis", || {
                format!(
                    "Layout time: {:.2}ms per line\n\
                     Cache hit rate: {:.1}%\n\
                     Glyph cache size: {} items",
                    context.stats.avg_layout_time_ms(),
                    context.stats.cache_hit_rate() * 100.0,
                    context.glyph_cache.len()
                )
            });
            
            if context.stats.cache_hit_rate() < 0.7 {
                report.add_recommendation("Increase glyph cache size");
            }
        }
        
        LayoutIssue::IncorrectWrapping => {
            report.add_section("Wrapping Analysis", || {
                let wrapped_lines: usize = layout_result.layout_lines
                    .iter()
                    .filter(|l| l.is_wrapped)
                    .count();
                
                format!(
                    "Total lines: {}\n\
                     Wrapped lines: {}\n\
                     Average wrap count: {:.1}",
                    layout_result.layout_lines.len(),
                    wrapped_lines,
                    layout_result.layout_lines
                        .iter()
                        .map(|l| l.wrap_count as f32)
                        .sum::<f32>() / layout_result.layout_lines.len() as f32
                )
            });
        }
        
        LayoutIssue::MemoryHigh => {
            report.add_section("Memory Analysis", || {
                format!(
                    "Total memory: {}MB\n\
                     Glyph cache: {} items\n\
                     Line cache: {} items\n\
                     Coordinate cache: {} items",
                    context.stats.memory_usage() / 1024 / 1024,
                    context.glyph_cache.len(),
                    context.line_layout_cache.len(),
                    context.coordinate_cache.len()
                )
            });
            
            report.add_recommendation("Reduce cache sizes");
            report.add_recommendation("Enable memory compression");
        }
    }
    
    report
}
```

---

*本文档是Layout系统的实现指南，实施时可进行优化但不违反架构约束。*
```

---

## 3. **API层文档**：API参考和使用示例

```markdown
# Layout系统API参考文档

## 📋 文档信息
- **版本**：1.0
- **状态**：API稳定（可扩展）
- **关联模块**：`crate::core::layout`

## 🎯 快速开始

### 基本使用
```rust
use zedit_core::layout::*;
use zedit_core::viewmodel::ViewModelSnapshot;

// 1. 创建布局上下文
let font_system = FontSystem::new();
let mut context = LayoutContext::new(font_system);

// 2. 配置布局
let config = LayoutConfig {
    font_size: 14.0,
    font_family: FontFamily::monospace(),
    tab_width: 4,
    line_spacing: 4.0,
    max_line_width: Some(800.0),
    wrap_mode: WrapMode::Word,
    dpi_scale: 1.0,
};

// 3. 创建布局引擎
let mut engine = LayoutEngine::new();

// 4. 布局视图模型快照
let snapshot: ViewModelSnapshot = /* 从ViewModel获取 */;
let layout_result = engine.layout_snapshot(&mut context, &snapshot, &config);

// 5. 使用布局结果
let line = layout_result.line_at(0).unwrap();
println!("Line height: {}", line.height());
println!("Line width: {}", line.width());

// 6. 坐标转换
let logical_pos = LogicalPosition::new(5, 10);
let physical_pos = context.logical_to_physical(&layout_result, logical_pos).unwrap();
println!("Logical {} -> Physical {}", logical_pos, physical_pos);
```

### 完整编辑器集成示例
```rust
struct EditorLayoutPipeline {
    layout_context: LayoutContext,
    layout_engine: LayoutEngine,
    current_layout: Option<Arc<LayoutResult>>,
    config: LayoutConfig,
}

impl EditorLayoutPipeline {
    fn process_viewmodel_update(
        &mut self,
        snapshot: Arc<ViewModelSnapshot>,
        delta: &ViewModelDelta,
    ) -> Option<Arc<LayoutResult>> {
        let start_time = Instant::now();
        
        // 1. 确定更新策略
        let update_strategy = self.determine_update_strategy(&snapshot, delta);
        
        // 2. 执行布局
        let new_layout = match update_strategy {
            UpdateStrategy::Full => {
                // 全量布局
                self.layout_engine.layout_snapshot(
                    &mut self.layout_context,
                    &snapshot,
                    &self.config,
                )
            }
            UpdateStrategy::Incremental(dirty_range) => {
                // 增量布局
                if let Some(current) = &self.current_layout {
                    self.layout_engine.incremental_layout(
                        &mut self.layout_context,
                        current,
                        &snapshot,
                        dirty_range,
                        &self.config,
                    )
                } else {
                    // 没有当前布局，退回到全量布局
                    self.layout_engine.layout_snapshot(
                        &mut self.layout_context,
                        &snapshot,
                        &self.config,
                    )
                }
            }
            UpdateStrategy::None => {
                // 无需更新，返回当前布局
                return self.current_layout.clone();
            }
        };
        
        // 3. 更新当前布局
        let layout_arc = Arc::new(new_layout);
        self.current_layout = Some(layout_arc.clone());
        
        // 4. 记录性能指标
        let duration = start_time.elapsed();
        self.layout_context.stats().record_layout(duration);
        
        Some(layout_arc)
    }
    
    fn determine_update_strategy(
        &self,
        snapshot: &ViewModelSnapshot,
        delta: &ViewModelDelta,
    ) -> UpdateStrategy {
        // 检查是否需要全量布局
        if self.should_full_layout(snapshot, delta) {
            return UpdateStrategy::Full;
        }
        
        // 检查是否有增量更新的可能
        if let Some(dirty_range) = delta.affected_range() {
            // 计算脏区大小
            let dirty_lines = dirty_range.len();
            let total_lines = snapshot.viewport_range().len();
            let dirty_ratio = dirty_lines as f32 / total_lines as f32;
            
            // 如果脏区较小，使用增量布局
            if dirty_ratio < 0.3 {
                return UpdateStrategy::Incremental(dirty_range);
            }
        }
        
        // 默认全量布局
        UpdateStrategy::Full
    }
    
    fn should_full_layout(&self, snapshot: &ViewModelSnapshot, delta: &ViewModelDelta) -> bool {
        // 配置变化需要全量布局
        if delta.metadata_changed {
            return true;
        }
        
        // 视口范围变化需要全量布局
        if let Some(current) = &self.current_layout {
            if current.viewport_range() != snapshot.viewport_range() {
                return true;
            }
        }
        
        // 装饰变化可能需要全量布局（如果影响布局）
        if delta.updated_decorations {
            // 检查装饰是否影响布局（如字体变化）
            return self.decorations_affect_layout(snapshot);
        }
        
        false
    }
}
```

## 📖 API参考

### 核心结构体

#### `LayoutContext` - 布局上下文
```rust
impl LayoutContext {
    /// 创建新上下文
    pub fn new(font_system: FontSystem) -> Self
    
    /// 配置布局上下文
    pub fn configure(&mut self, config: LayoutConfig) -> &mut Self
    
    /// 布局单行文本
    pub fn layout_line(
        &mut self,
        line: &RenderedLine,
        max_width: Option<f32>,
    ) -> LayoutLine
    
    /// 布局整个快照
    pub fn layout_snapshot(
        &mut self,
        snapshot: &ViewModelSnapshot,
        config: &LayoutConfig,
    ) -> LayoutResult
    
    /// 逻辑位置 → 物理位置
    pub fn logical_to_physical(
        &self,
        layout_result: &LayoutResult,
        logical_pos: LogicalPosition,
    ) -> Option<PhysicalPosition>
    
    /// 物理位置 → 逻辑位置
    pub fn physical_to_logical(
        &self,
        layout_result: &LayoutResult,
        physical_pos: PhysicalPosition,
    ) -> Option<LogicalPosition>
    
    /// 获取布局统计
    pub fn stats(&self) -> &LayoutStats
    
    /// 清空缓存
    pub fn clear_cache(&mut self)
    
    /// 调整缓存大小
    pub fn resize_cache(&mut self, glyph_cache: usize, line_cache: usize)
}
```

#### `LayoutResult` - 布局结果
```rust
impl LayoutResult {
    /// 获取布局ID
    pub fn id(&self) -> LayoutId
    
    /// 获取关联的快照ID
    pub fn snapshot_id(&self) -> SnapshotId
    
    /// 获取视口范围
    pub fn viewport_range(&self) -> LineRange
    
    /// 获取布局行
    pub fn lines(&self) -> &[LayoutLine]
    
    /// 获取指定索引的行
    pub fn line_at(&self, index: usize) -> Option<&LayoutLine>
    
    /// 获取指定行号的布局行
    pub fn line_by_number(&self, line_number: usize) -> Option<&LayoutLine>
    
    /// 获取总高度
    pub fn total_height(&self) -> f32
    
    /// 获取最大行宽
    pub fn max_line_width(&self) -> f32
    
    /// 获取缓存统计
    pub fn cached_glyphs(&self) -> usize
    
    /// 获取缓存命中率
    pub fn hit_rate(&self) -> f32
    
    /// 获取元数据
    pub fn metadata(&self) -> &LayoutMetadata
    
    /// 克隆为Arc
    pub fn clone_arc(&self) -> Arc<LayoutResult>
    
    /// 估计内存占用
    pub fn estimated_size(&self) -> usize
}
```

#### `LayoutLine` - 布局行
```rust
impl LayoutLine {
    /// 获取源行句柄
    pub fn source_line(&self) -> &LineHandle
    
    /// 获取逻辑行号
    pub fn line_number(&self) -> usize
    
    /// 获取布局片段
    pub fn fragments(&self) -> &[LayoutFragment]
    
    /// 获取垂直位置
    pub fn y_position(&self) -> f32
    
    /// 获取行高
    pub fn height(&self) -> f32
    
    /// 获取上伸高度
    pub fn ascent(&self) -> f32
    
    /// 获取下伸高度
    pub fn descent(&self) -> f32
    
    /// 检查是否被换行
    pub fn is_wrapped(&self) -> bool
    
    /// 获取换行次数
    pub fn wrap_count(&self) -> usize
    
    /// 获取换行后的子行
    pub fn wrapped_lines(&self) -> Option<&[WrappedLine]>
    
    /// 计算行宽
    pub fn width(&self) -> f32
    
    /// 在指定列获取水平位置
    pub fn x_at_column(&self, column: usize) -> Option<f32>
    
    /// 在指定水平位置获取列
    pub fn column_at_x(&self, x: f32) -> usize
}
```

#### `LayoutFragment` - 布局片段
```rust
impl LayoutFragment {
    /// 获取源视觉片段
    pub fn visual_span(&self) -> &Arc<VisualSpan>
    
    /// 获取片段文本
    pub fn text(&self) -> &str
    
    /// 获取水平位置
    pub fn x_position(&self) -> f32
    
    /// 获取片段宽度
    pub fn width(&self) -> f32
    
    /// 获取上伸高度
    pub fn ascent(&self) -> f32
    
    /// 获取下伸高度
    pub fn descent(&self) -> f32
    
    /// 获取定位后的字形
    pub fn glyphs(&self) -> &[PositionedGlyph]
    
    /// 获取字符到字形的映射
    pub fn cluster_map(&self) -> &[usize]
    
    /// 获取视觉属性
    pub fn visual_attrs(&self) -> VisualAttributes
    
    /// 在片段内获取字符的水平位置
    pub fn x_at_char(&self, char_index: usize) -> Option<f32>
    
    /// 在片段内获取指定水平位置的字符索引
    pub fn char_at_x(&self, x: f32) -> usize
}
```

### 配置API

#### `LayoutConfig`
```rust
impl LayoutConfig {
    /// 默认配置
    pub fn default() -> Self
    
    /// 编程配置（等宽字体，tab=4）
    pub fn programming() -> Self
    
    /// 写作配置（比例字体，换行优化）
    pub fn writing() -> Self
    
    /// 调试配置（显示所有度量）
    pub fn debug() -> Self
    
    /// 从设置加载
    pub fn from_settings(settings: &Settings) -> Self
    
    /// 保存到设置
    pub fn save_to_settings(&self, settings: &mut Settings)
}

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
    pub hinting: HintingMode,
    pub antialiasing: AntialiasingMode,
    
    // 缓存配置
    pub glyph_cache_size: usize,
    pub line_cache_size: usize,
    pub enable_incremental: bool,
}

/// 换行模式
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WrapMode {
    None,           // 不换行
    Word,           // 按单词换行
    Character,      // 按字符换行
    Whitespace,     // 在空白处换行
    Smart,          // 智能换行（混合策略）
}
```

### 引擎API

#### `LayoutEngine`
```rust
impl LayoutEngine {
    /// 创建新引擎
    pub fn new() -> Self
    
    /// 全量布局快照
    pub fn layout_snapshot(
        &self,
        context: &mut LayoutContext,
        snapshot: &ViewModelSnapshot,
        config: &LayoutConfig,
    ) -> LayoutResult
    
    /// 增量布局更新
    pub fn incremental_layout(
        &self,
        context: &mut LayoutContext,
        previous: &LayoutResult,
        snapshot: &ViewModelSnapshot,
        dirty_range: LineRange,
        config: &LayoutConfig,
    ) -> LayoutResult
    
    /// 计算布局差异
    pub fn compute_layout_delta(
        &self,
        old_result: &LayoutResult,
        new_result: &LayoutResult,
    ) -> LayoutDelta
    
    /// 应用布局差异（创建新布局）
    pub fn apply_layout_delta(
        &self,
        context: &mut LayoutContext,
        base_result: &LayoutResult,
        delta: &LayoutDelta,
    ) -> LayoutResult
    
    /// 验证布局结果
    pub fn validate_layout(&self, result: &LayoutResult) -> ValidationResult
    
    /// 优化布局（合并片段等）
    pub fn optimize_layout(
        &self,
        context: &mut LayoutContext,
        result: &LayoutResult,
    ) -> LayoutResult
}
```

### 字体管理API

#### `FontManager`
```rust
impl FontManager {
    /// 创建字体管理器
    pub fn new() -> Self
    
    /// 加载字体文件
    pub fn load_font_file(&mut self, path: &str) -> Result<FontId>
    
    /// 加载字体数据
    pub fn load_font_data(&mut self, data: &[u8]) -> Result<FontId>
    
    /// 获取字体
    pub fn get_font(&self, font_id: FontId) -> Option<&Font>
    
    /// 根据属性选择字体
    pub fn select_font_for_attrs(
        &self,
        attrs: &VisualAttributes,
        fallback: bool,
    ) -> Option<FontId>
    
    /// 获取字体度量
    pub fn font_metrics(&self, font_id: FontId, size: f32) -> Option<FontMetrics>
    
    /// 测量文本
    pub fn measure_text(
        &self,
        font_id: FontId,
        text: &str,
        size: f32,
    ) -> Option<TextMetrics>
    
    /// 形状化文本
    pub fn shape_text(
        &self,
        font_id: FontId,
        text: &str,
        size: f32,
        features: &FontFeatures,
    ) -> Option<Vec<PositionedGlyph>>
}
```

## 🎪 使用示例

### 示例1：自定义换行策略
```rust
// 实现自定义换行算法
struct CustomLineWrapper {
    config: CustomWrapConfig,
    word_breaker: WordBreaker,
    hyphenator: Option<Hyphenator>,
}

impl CustomLineWrapper {
    fn wrap_line_custom(
        &self,
        fragments: &[LayoutFragment],
        max_width: f32,
        context: &LayoutContext,
    ) -> Vec<Vec<LayoutFragment>> {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut current_width = 0.0;
        
        for fragment in fragments {
            // 自定义分割逻辑
            if fragment.width > max_width {
                let sub_fragments = self.split_fragment_custom(fragment, max_width, context);
                
                for sub_fragment in sub_fragments {
                    if current_width + sub_fragment.width > max_width {
                        // 换行
                        lines.push(current_line);
                        current_line = Vec::new();
                        current_width = 0.0;
                    }
                    
                    current_line.push(sub_fragment);
                    current_width += sub_fragment.width;
                }
            } else if current_width + fragment.width > max_width {
                // 换行
                lines.push(current_line);
                current_line = vec![fragment.clone()];
                current_width = fragment.width;
            } else {
                // 添加到当前行
                current_line.push(fragment.clone());
                current_width += fragment.width;
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }
    
    fn split_fragment_custom(
        &self,
        fragment: &LayoutFragment,
        max_width: f32,
        context: &LayoutContext,
    ) -> Vec<LayoutFragment> {
        let text = fragment.text.as_str();
        let font = context.font_for_attrs(&fragment.visual_attrs);
        
        // 使用自定义分词器
        let breakpoints = self.word_breaker.break_text(text);
        
        let mut splits = Vec::new();
        let mut start = 0;
        
        for breakpoint in breakpoints {
            let sub_text = &text[start..breakpoint];
            let sub_width = font.measure_text(sub_text, fragment.visual_attrs.font_size);
            
            if sub_width > max_width {
                // 子段仍然太宽，需要进一步分割
                let char_splits = self.split_by_character(sub_text, max_width, font, fragment);
                splits.extend(char_splits);
            } else {
                // 创建子片段
                let sub_fragment = self.create_sub_fragment(fragment, sub_text, start, breakpoint, context);
                splits.push(sub_fragment);
            }
            
            start = breakpoint;
        }
        
        splits
    }
}

// 集成自定义换行器
fn setup_custom_wrapping(layout_engine: &mut LayoutEngine) {
    let wrapper = CustomLineWrapper::new();
    layout_engine.set_line_wrapper(Box::new(wrapper));
}
```

### 示例2：布局调试和可视化
```rust
// 布局调试工具
struct LayoutDebugger {
    context: LayoutContext,
    visualizer: LayoutVisualizer,
    profiler: LayoutProfiler,
}

impl LayoutDebugger {
    fn analyze_layout_performance(&self, result: &LayoutResult) -> PerformanceReport {
        let stats = self.context.stats();
        
        PerformanceReport {
            total_lines: result.lines().len(),
            total_fragments: result.lines().iter().map(|l| l.fragments().len()).sum(),
            total_glyphs: result.lines().iter()
                .flat_map(|l| l.fragments())
                .map(|f| f.glyphs().len())
                .sum(),
            
            cache_efficiency: stats.cache_hit_rate(),
            avg_layout_time_per_line: stats.avg_layout_time_ms(),
            memory_usage: self.context.estimated_memory(),
            
            recommendations: self.generate_recommendations(result, stats),
        }
    }
    
    fn visualize_layout_structure(&self, result: &LayoutResult) -> LayoutVisualization {
        let mut svg = String::new();
        
        // 创建SVG可视化
        svg.push_str("<svg xmlns='http://www.w3.org/2000/svg' width='1000' height='600'>\n");
        
        // 背景
        svg.push_str("<rect width='100%' height='100%' fill='white'/>\n");
        
        let mut y = 20.0;
        for (i, line) in result.lines().iter().enumerate() {
            // 行背景
            svg.push_str(&format!(
                "<rect x='10' y='{}' width='980' height='{}' fill='#f8f8f8'/>\n",
                y, line.height()
            ));
            
            // 行号
            svg.push_str(&format!(
                "<text x='15' y='{}' font-size='12' fill='#666'>{}</text>\n",
                y + line.ascent(), i
            ));
            
            // 片段
            let mut x = 50.0;
            for fragment in line.fragments() {
                // 片段矩形
                svg.push_str(&format!(
                    "<rect x='{}' y='{}' width='{}' height='{}' fill='#e0e0e0' stroke='#ccc'/>\n",
                    x, y + fragment.ascent, fragment.width, fragment.descent - fragment.ascent
                ));
                
                // 片段文本
                svg.push_str(&format!(
                    "<text x='{}' y='{}' font-size='12'>{}</text>\n",
                    x + 2.0, y + fragment.ascent - 2.0,
                    escape_html(&fragment.text)
                ));
                
                // 片段信息
                svg.push_str(&format!(
                    "<text x='{}' y='{}' font-size='10' fill='#999'>{:.1}px</text>\n",
                    x, y + line.height() - 2.0, fragment.width
                ));
                
                x += fragment.width;
            }
            
            y += line.height() + 5.0;
        }
        
        svg.push_str("</svg>");
        
        LayoutVisualization::Svg(svg)
    }
    
    fn debug_coordinate_mapping(
        &self,
        result: &LayoutResult,
        test_points: &[(LogicalPosition, PhysicalPosition)],
    ) -> CoordinateDebugReport {
        let mut report = CoordinateDebugReport::new();
        
        for (logical, expected_physical) in test_points {
            let actual_physical = self.context.logical_to_physical(result, *logical);
            
            match actual_physical {
                Some(actual) => {
                    let distance = ((actual.x - expected_physical.x).powi(2) + 
                                   (actual.y - expected_physical.y).powi(2)).sqrt();
                    
                    if distance > 0.5 {
                        // 误差超过0.5像素
                        report.add_mismatch(
                            *logical,
                            *expected_physical,
                            actual,
                            distance,
                        );
                    }
                }
                None => {
                    report.add_missing(*logical, *expected_physical);
                }
            }
        }
        
        report
    }
}
```

### 示例3：高性能布局流水线
```rust
// 并行布局流水线
struct ParallelLayoutPipeline {
    worker_pool: ThreadPool,
    layout_engines: Vec<LayoutEngine>,
    font_system: Arc<FontSystem>,
}

impl ParallelLayoutPipeline {
    fn layout_snapshot_parallel(
        &self,
        snapshot: Arc<ViewModelSnapshot>,
        config: LayoutConfig,
    ) -> LayoutResult {
        let line_count = snapshot.lines().len();
        let batch_size = (line_count + self.worker_pool.size() - 1) / self.worker_pool.size();
        
        // 分割行到多个任务
        let mut tasks = Vec::new();
        for chunk_start in (0..line_count).step_by(batch_size) {
            let chunk_end = (chunk_start + batch_size).min(line_count);
            let snapshot_clone = snapshot.clone();
            let config_clone = config.clone();
            let font_system = self.font_system.clone();
            
            tasks.push(move || {
                // 为每个任务创建独立的布局上下文
                let mut context = LayoutContext::new(font_system);
                context.configure(&config_clone);
                
                let mut lines = Vec::new();
                for i in chunk_start..chunk_end {
                    let line = snapshot_clone.line_at(i).unwrap();
                    let layout_line = context.layout_line(line, config_clone.max_line_width);
                    lines.push((i, layout_line));
                }
                
                lines
            });
        }
        
        // 并行执行
        let results: Vec<Vec<(usize, LayoutLine)>> = self.worker_pool.parallel_map(tasks);
        
        // 合并结果
        let mut all_lines: Vec<(usize, LayoutLine)> = results.into_iter().flatten().collect();
        all_lines.sort_by_key(|(i, _)| *i);
        
        // 计算垂直位置
        let mut y = 0.0;
        let mut max_width = 0.0;
        let mut lines_vec = Vec::with_capacity(all_lines.len());
        
        for (_, mut line) in all_lines {
            line.y_position = y;
            y += line.height();
            
            let line_width = line.width();
            if line_width > max_width {
                max_width = line_width;
            }
            
            lines_vec.push(line);
        }
        
        LayoutResult {
            id: LayoutId::new(),
            snapshot_id: snapshot.id(),
            viewport_range: snapshot.viewport_range(),
            layout_lines: lines_vec.into(),
            total_height: y,
            max_line_width: max_width,
            cached_glyphs: 0, // 需要从各个上下文收集
            hit_rate: 0.0,
            metadata: LayoutMetadata {
                source: LayoutSource::Parallel,
                build_time: Instant::now(),
                lines_updated: line_count,
                total_lines: line_count,
            },
        }
    }
    
    fn incremental_layout_parallel(
        &self,
        previous: Arc<LayoutResult>,
        snapshot: Arc<ViewModelSnapshot>,
        dirty_range: LineRange,
        config: LayoutConfig,
    ) -> LayoutResult {
        // 类似实现，但只并行处理脏区内的行
        // 非脏区的行可以直接从previous复制
        
        unimplemented!()
    }
}
```

## ⚠️ 注意事项

### 性能建议
1. **合理配置缓存大小**：
   ```rust
   let config = LayoutConfig {
       glyph_cache_size: 10000,    // 字形缓存
       line_cache_size: 500,       // 行布局缓存
       enable_incremental: true,   // 启用增量布局
       ..Default::default()
   };
   ```

2. **增量布局阈值**：
   ```rust
   // 当脏区小于30%时使用增量布局
   if dirty_ratio < 0.3 {
       engine.incremental_layout(...)
   } else {
       engine.layout_snapshot(...)
   }
   ```

3. **字体管理优化**：
   ```rust
   // 预加载常用字体
   font_manager.preload_fonts(&[
       "Consolas", "Monaco", "Courier New",
       "Arial", "Helvetica", "Times New Roman"
   ]);
   
   // 启用字体回退
   context.enable_font_fallback(true);
   ```

### 内存管理
1. **监控缓存使用**：
   ```rust
   let stats = context.stats();
   if stats.memory_usage() > 100 * 1024 * 1024 { // 100MB
       context.clear_cache();
   }
   ```

2. **适时清理**：
   ```rust
   // 文档关闭时
   fn on_document_close(&mut self) {
       self.layout_context.clear_cache();
       self.current_layout.take();
   }
   
   // 内存警告时
   fn on_memory_warning(&mut self) {
       self.layout_context.resize_cache(5000, 200); // 缩减缓存
   }
   ```

3. **避免内存泄漏**：
   ```rust
   // 定期检查缓存有效性
   fn cleanup_stale_cache(&mut self) {
       let stale_count = self.layout_context.cleanup_stale_entries();
       if stale_count > 0 {
           log::debug!("Cleaned up {} stale cache entries", stale_count);
       }
   }
   ```

### 错误处理
```rust
// 布局错误处理
match engine.layout_snapshot(&mut context, &snapshot, &config) {
    Ok(result) => {
        self.current_layout = Some(Arc::new(result));
    }
    Err(LayoutError::FontNotFound(font_name)) => {
        log::error!("Font not found: {}", font_name);
        // 使用回退字体
        let fallback_config = config.with_fallback_font();
        let result = engine.layout_snapshot(&mut context, &snapshot, &fallback_config)?;
        self.current_layout = Some(Arc::new(result));
    }
    Err(LayoutError::InvalidInput(msg)) => {
        log::error!("Invalid layout input: {}", msg);
        // 尝试修复输入或使用默认值
        self.recover_from_layout_error(&snapshot, &config);
    }
    Err(LayoutError::OutOfMemory) => {
        log::error!("Layout out of memory");
        // 清理缓存并重试
        context.clear_cache();
        let result = engine.layout_snapshot(&mut context, &snapshot, &config)?;
        self.current_layout = Some(Arc::new(result));
    }
    Err(e) => {
        log::error!("Layout error: {}", e);
        return Err(e.into());
    }
}

// 坐标转换错误处理
fn safe_coordinate_conversion(
    context: &LayoutContext,
    layout_result: &LayoutResult,
    logical_pos: LogicalPosition,
) -> Result<PhysicalPosition> {
    context.logical_to_physical(layout_result, logical_pos)
        .ok_or_else(|| {
            // 位置无效，调整到最近的有效位置
            let adjusted = layout_result.clamp_position(logical_pos);
            context.logical_to_physical(layout_result, adjusted)
                .ok_or(LayoutError::CoordinateConversionFailed)
        })?
}

// 增量布局失败降级
fn safe_incremental_layout(
    engine: &LayoutEngine,
    context: &mut LayoutContext,
    previous: &LayoutResult,
    snapshot: &ViewModelSnapshot,
    dirty_range: LineRange,
    config: &LayoutConfig,
) -> LayoutResult {
    match engine.incremental_layout(context, previous, snapshot, dirty_range, config) {
        Ok(result) => result,
        Err(LayoutError::IncrementalFailed) => {
            log::warn!("Incremental layout failed, falling back to full layout");
            engine.layout_snapshot(context, snapshot, config)
        }
        Err(e) => {
            panic!("Unexpected layout error: {}", e);
        }
    }
}
```

### 调试技巧
```rust
// 启用详细日志
env_logger::Builder::new()
    .filter_module("zedit_core::layout", LevelFilter::Debug)
    .init();

// 布局性能分析
fn profile_layout_performance(engine: &LayoutEngine, context: &mut LayoutContext) {
    let test_snapshots = create_performance_test_snapshots();
    let config = LayoutConfig::default();
    
    for (i, snapshot) in test_snapshots.iter().enumerate() {
        let start = Instant::now();
        let result = engine.layout_snapshot(context, snapshot, &config);
        let duration = start.elapsed();
        
        log::info!(
            "Test {}: {} lines, {:.2}ms, {:.2}ms/line, {} glyphs cached",
            i,
            result.lines().len(),
            duration.as_secs_f64() * 1000.0,
            duration.as_secs_f64() * 1000.0 / result.lines().len() as f64,
            result.cached_glyphs()
        );
        
        // 验证布局正确性
        if let Err(e) = engine.validate_layout(&result) {
            log::warn!("Layout validation failed: {}", e);
        }
    }
}

// 坐标映射验证
fn verify_coordinate_mapping(
    context: &LayoutContext,
    layout_result: &LayoutResult,
) -> VerificationResult {
    let mut errors = Vec::new();
    
    // 测试随机点
    for _ in 0..100 {
        let line_idx = rand::random::<usize>() % layout_result.lines().len();
        let line = &layout_result.lines()[line_idx];
        
        let column = rand::random::<usize>() % 100.min(line.width() as usize);
        let logical_pos = LogicalPosition::new(line.line_number(), column);
        
        let physical_pos = context.logical_to_physical(layout_result, logical_pos);
        let round_trip = context.physical_to_logical(layout_result, physical_pos.unwrap());
        
        if let Some(rt_pos) = round_trip {
            if rt_pos != logical_pos {
                errors.push(CoordinateError {
                    original: logical_pos,
                    round_trip: rt_pos,
                    physical: physical_pos.unwrap(),
                });
            }
        }
    }
    
    VerificationResult {
        total_tests: 100,
        errors,
        success_rate: (100 - errors.len()) as f32 / 100.0,
    }
}
```

---

*本文档是Layout系统的API参考，所有公共API应保持向后兼容。*
```

---

## **核心洞见总结**

这个Layout系统设计有几个关键特点：

### 1. **纯计算架构**
- Layout系统完全无状态，所有计算都是纯函数
- 相同的输入总是产生相同的输出，便于缓存和测试
- 通过`LayoutContext`管理缓存和字体等资源

### 2. **多层缓存系统**
- **字形缓存**：高频字形的度量和形状信息
- **行布局缓存**：基于内容哈希的完整行布局
- **坐标缓存**：逻辑↔物理坐标映射的快速查找
- **增量缓存**：支持增量更新的中间结果

### 3. **智能增量更新**
- 基于脏区范围的最小化重新计算
- 配置变化的智能检测和全量更新触发
- 换行影响的区域扩展计算

### 4. **精确坐标系统**
- 支持亚像素精度的文本定位
- 逻辑、物理、布局、视口四层坐标系
- 双向映射的快速坐标转换

### 5. **可扩展的换行策略**
- 支持单词、字符、空白、智能等多种换行模式
- 可插拔的换行算法接口
- 换行质量的评估和优化

### 6. **全面的性能监控**
- 缓存命中率、布局时间、内存使用等统计
- 布局质量验证和调试工具
- 自动化优化建议

这个Layout系统作为ViewModel和Render System之间的桥梁，将逻辑文本数据转换为精确的物理几何信息，是确保编辑器渲染性能和精度的关键组件。设计完全遵循架构宪法，保持了单向数据流和状态隔离的原则。