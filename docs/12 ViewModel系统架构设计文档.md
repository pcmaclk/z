# 📚 **ViewModel系统文档体系**

```
docs/
├── 架构层/
│   └── viewmodel-design.md      # 设计理念和架构位置
├── 实现层/
│   └── viewmodel-implementation.md  # 实现细节和规范
├── API层/
│   └── viewmodel-api.md         # API参考和使用示例
└── 优化层/
    └── viewmodel-optimizations.md  # 性能优化记录
```

---

## 1. **架构层文档**：ViewModel设计理念

```markdown
# ViewModel系统架构设计文档

## 📋 文档信息
- **版本**：1.0
- **状态**：已冻结
- **关联文档**：[架构宪法] · [数据流规范] · [Viewport设计] · [PieceTable设计]

## 🎯 设计目标

### 核心定位
ViewModel是zedit编辑器的**渲染数据转换层**，负责：
1. **数据融合**：将逻辑状态、装饰信息、用户配置合并为渲染友好格式
2. **格式转换**：将内部数据结构转换为UI可直接消费的形式
3. **增量更新**：基于脏区信息最小化转换工作量
4. **线程安全**：生成完全独立、可跨线程传递的只读快照

### 设计哲学
1. **只读快照**：ViewModel是时间点的冻结状态，不含可变引用
2. **数据完整**：包含渲染所需的一切信息，不依赖外部查询
3. **格式优化**：针对渲染性能优化数据结构，而非编辑操作
4. **惰性构建**：按需构建，支持部分更新和缓存

## 🏗️ 架构位置

### 在系统中的作用
```
┌─────────────────┐
│   Editor Core   │  ← 权威状态源
├─────────────────┤
│   Viewport      │  ← 可见性控制
├─────────────────┤
│   ViewModel     │  ← 本文档对象（渲染数据转换）
├─────────────────┤
│   Render System │  ← 渲染消费者
└─────────────────┘
```

### 数据流角色
- **输入**：`ViewportData`（逻辑内容）、`DecorationSet`（装饰信息）、`RenderConfig`（渲染配置）
- **输出**：`ViewModelSnapshot`（完整渲染快照）
- **特点**：**纯函数转换**，无副作用，可缓存和复用

## 📊 核心设计决策

### 已冻结决策
1. **快照隔离性**：每个ViewModel是完全独立的`Arc<Snapshot>`，可安全跨线程
2. **分层装饰**：基础文本层、语法层、搜索层、选区层分离合成
3. **增量构建**：基于脏区范围只重建受影响部分
4. **格式优化**：使用连续内存数组，避免渲染时二次转换

### 与其他组件的关系
| 组件 | 与ViewModel的关系 | 通信方式 |
|------|-------------------|----------|
| Editor Core | 数据源，提供逻辑文本 | ViewportData查询 |
| Viewport | 可见性源，提供范围 | 只接受可见范围数据 |
| Syntax System | 装饰源，提供语法标记 | 接收Token流，融合到文本 |
| Search System | 装饰源，提供高亮范围 | 接收Range集合，分层叠加 |
| Render System | 消费者，直接渲染 | 传递完整Snapshot |
| Theme System | 样式源，提供颜色字体 | 预应用样式到ViewModel |

## 🔧 设计约束

### 必须遵守的约束
1. **不可变性**：生成后不可修改，所有更新创建新快照
2. **完整性**：包含渲染所需的所有信息，无外部依赖
3. **性能保证**：转换时间与可见内容大小线性相关
4. **内存安全**：无循环引用，可安全序列化

### 性能目标
| 操作 | 目标延迟 | 备注 |
|------|----------|------|
| 全量构建（100行） | <5ms | 含语法高亮 |
| 增量构建（10行变化） | <1ms | 基于脏区 |
| 装饰层合成 | <2ms | 多层叠加 |
| 快照克隆 | O(1) | Arc引用计数 |

## 📈 演进原则

### 允许的演进
1. **装饰系统扩展**：添加新类型的装饰层（错误波浪线、TODO标记等）
2. **格式优化**：改进内存布局，提高缓存友好性
3. **增量算法优化**：更智能的脏区追踪和部分重建
4. **压缩优化**：对大文本视图的压缩表示

### 禁止的演进
1. **可变状态**：不添加任何可变字段或内部可变性
2. **外部依赖**：不持有外部资源引用（文件句柄、网络连接等）
3. **编辑逻辑**：不包含任何文本修改或状态管理能力
4. **渲染耦合**：不包含平台特定的渲染逻辑

## 🔗 核心概念定义

### 关键术语
| 术语 | 定义 |
|------|------|
| ViewModelSnapshot | 时间点的完整渲染状态快照 |
| RenderedLine | 单行的完整渲染表示 |
| DecorationLayer | 装饰信息的逻辑层（语法、搜索、选区等） |
| VisualAttribute | 视觉属性（颜色、字体、样式等） |
| DirtyRegion | 需要重建的视图区域 |
| IncrementalBuilder | 基于前一个快照的增量构建器 |

### 装饰层合成顺序
1. **Base Layer**：基础文本和位置
2. **Syntax Layer**：语法高亮标记
3. **Search Layer**：搜索结果高亮
4. **Selection Layer**：选中区域渲染
5. **Cursor Layer**：光标位置标记
6. **Overlay Layer**：临时覆盖层（错误、断点等）

---

*本文档定义了ViewModel系统的架构角色和设计约束，所有实现必须遵守。*
```

---

## 2. **实现层文档**：ViewModel实现细节

```markdown
# ViewModel系统实现规范文档

## 📋 文档信息
- **版本**：1.0
- **状态**：实施指南（可优化）
- **关联代码**：`src/core/viewmodel/`

## 🏗️ 核心数据结构

### 1. ViewModelSnapshot（渲染快照）
```rust
struct ViewModelSnapshot {
    // 标识信息
    id: SnapshotId,
    version: u64,
    timestamp: Instant,
    
    // 视图范围
    viewport_range: LineRange,
    total_lines: usize,
    
    // 渲染数据（连续内存，缓存友好）
    lines: Arc<[RenderedLine]>,
    
    // 装饰状态
    decorations: Arc<DecorationSet>,
    
    // 光标和选区
    cursor: Option<CursorState>,
    selection: Option<SelectionState>,
    
    // 元数据
    metadata: SnapshotMetadata,
}

// 完全独立的只读快照
impl ViewModelSnapshot {
    // 所有字段都是私有 + 只读访问器
    // 没有可变方法
}
```

**设计考虑**：
- **Arc共享**：快照本身是`Arc<Snapshot>`，克隆成本O(1)
- **连续内存**：`lines`是连续数组，提高缓存局部性
- **分层装饰**：装饰信息独立存储，支持部分更新
- **完全隔离**：不持有任何外部引用，可安全跨线程

### 2. RenderedLine（单行渲染表示）
```rust
struct RenderedLine {
    // 基础信息
    line_number: usize,
    logical_text: Arc<str>,      // 逻辑文本（UTF-8）
    
    // 视觉表示
    visual_spans: Arc<[VisualSpan]>, // 视觉片段（已应用装饰）
    line_height: f32,
    baseline_offset: f32,
    
    // 布局信息
    glyph_positions: Option<Arc<[GlyphPosition]>>, // 可选，按需计算
    line_width: f32,
    
    // 装饰状态
    is_folded: bool,
    has_breakpoint: bool,
    is_changed: bool,
}

struct VisualSpan {
    text: Arc<str>,              // 该片段的文本
    visual_attrs: VisualAttributes, // 视觉属性
    byte_range: Range<usize>,    // 在逻辑文本中的范围
    column_range: Range<usize>,  // 列范围
}
```

### 3. DecorationSet（装饰集合）
```rust
struct DecorationSet {
    // 分层装饰
    syntax_layer: Option<Arc<SyntaxLayer>>,
    search_layer: Option<Arc<SearchLayer>>,
    selection_layer: Option<Arc<SelectionLayer>>,
    overlay_layer: Option<Arc<OverlayLayer>>,
    
    // 合并缓存（惰性计算）
    merged_cache: OnceCell<Arc<[MergedDecoration]>>,
}

// 装饰层接口
trait DecorationLayer {
    fn decorations_for_line(&self, line: usize) -> Option<Arc<[Decoration]>>;
    fn affected_range(&self) -> Option<LineRange>;
    fn version(&self) -> u64;
}

// 具体装饰
struct Decoration {
    byte_range: Range<usize>,    // 字节范围
    visual_attrs: VisualAttributes, // 视觉属性
    layer_priority: u8,          // 图层优先级（解决冲突）
}
```

### 4. VisualAttributes（视觉属性）
```rust
#[derive(Clone, Copy)]
struct VisualAttributes {
    // 颜色
    foreground: Option<Color>,
    background: Option<Color>,
    
    // 字体
    font_family: Option<FontFamily>,
    font_size: Option<f32>,
    font_weight: FontWeight,
    font_style: FontStyle,
    
    // 样式
    underline: Option<UnderlineStyle>,
    strikethrough: bool,
    
    // 其他
    opacity: f32,                // 0.0-1.0
}
```

## ⚙️ 核心算法实现

### 1. 增量构建算法
**位置**：`incremental_builder.rs` - `IncrementalBuilder::build()`

**算法流程**：
```rust
impl IncrementalBuilder {
    fn build_snapshot(
        &self,
        previous: Option<&ViewModelSnapshot>,
        viewport_data: ViewportData,
        decorations: &DecorationSet,
        config: &RenderConfig,
    ) -> ViewModelSnapshot {
        // 1. 计算脏区
        let dirty_regions = self.calculate_dirty_regions(
            previous,
            &viewport_data,
            decorations,
        );
        
        // 2. 如果有前快照且脏区小，增量构建
        if let Some(prev) = previous {
            if self.should_incremental_build(&dirty_regions, prev) {
                return self.incremental_build(prev, dirty_regions, viewport_data, decorations, config);
            }
        }
        
        // 3. 否则全量构建
        self.full_build(viewport_data, decorations, config)
    }
    
    fn calculate_dirty_regions(
        &self,
        previous: Option<&ViewModelSnapshot>,
        viewport_data: &ViewportData,
        decorations: &DecorationSet,
    ) -> Vec<DirtyRegion> {
        let mut regions = Vec::new();
        
        // a. 检查视口范围变化
        if let Some(prev) = previous {
            if prev.viewport_range != viewport_data.visible_range {
                regions.push(DirtyRegion::ViewportChanged {
                    old_range: prev.viewport_range,
                    new_range: viewport_data.visible_range,
                });
            }
        }
        
        // b. 检查文本内容变化（来自EditorStateSnapshot.dirty_range）
        if let Some(dirty_range) = viewport_data.dirty_range {
            regions.push(DirtyRegion::ContentChanged(dirty_range));
        }
        
        // c. 检查装饰变化
        regions.extend(self.check_decoration_changes(previous, decorations));
        
        regions
    }
}
```

### 2. 装饰合成算法
**位置**：`decoration_compositor.rs` - `DecorationCompositor::compose_line()`

**合成流程**：
```rust
impl DecorationCompositor {
    fn compose_line(
        &self,
        logical_text: &str,
        line_number: usize,
        decorations: &DecorationSet,
    ) -> Vec<VisualSpan> {
        // 1. 初始化基础片段（无装饰）
        let mut fragments = vec![VisualSpan::plain(logical_text)];
        
        // 2. 按优先级顺序应用装饰层
        let layers = self.get_layers_in_priority_order(decorations);
        
        for layer in layers {
            if let Some(decorations) = layer.decorations_for_line(line_number) {
                fragments = self.apply_layer(fragments, &decorations);
            }
        }
        
        // 3. 合并相邻相同样式的片段（优化）
        fragments = self.merge_adjacent_fragments(fragments);
        
        fragments
    }
    
    fn apply_layer(
        &self,
        fragments: Vec<VisualSpan>,
        decorations: &[Decoration],
    ) -> Vec<VisualSpan> {
        // 类似区间合并算法，但保留多层信息
        let mut result = Vec::new();
        let mut frag_idx = 0;
        let mut deco_idx = 0;
        
        while frag_idx < fragments.len() && deco_idx < decorations.len() {
            let frag = &fragments[frag_idx];
            let deco = &decorations[deco_idx];
            
            // 计算重叠区域
            match frag.byte_range.relative_to(&deco.byte_range) {
                RangeRelation::Before => {
                    // 片段在装饰前，直接保留
                    result.push(frag.clone());
                    frag_idx += 1;
                }
                RangeRelation::After => {
                    // 片段在装饰后，跳过此装饰
                    deco_idx += 1;
                }
                RangeRelation::Overlap(overlap) => {
                    // 有重叠，分割片段并应用装饰
                    let (before, overlapping, after) = frag.split_at_overlap(&overlap);
                    
                    if let Some(before) = before {
                        result.push(before);
                    }
                    
                    if let Some(mut overlapping) = overlapping {
                        overlapping.apply_decorations(deco.visual_attrs);
                        result.push(overlapping);
                    }
                    
                    if let Some(after) = after {
                        // 将剩余部分放回片段列表继续处理
                        fragments[frag_idx] = after;
                        // 不增加frag_idx，继续处理同一片段
                    } else {
                        frag_idx += 1;
                    }
                }
                RangeRelation::Contains => {
                    // 装饰完全包含片段
                    let mut decorated = frag.clone();
                    decorated.apply_decorations(deco.visual_attrs);
                    result.push(decorated);
                    frag_idx += 1;
                }
                RangeRelation::ContainedBy => {
                    // 片段完全包含装饰，需要分割
                    let (before, overlapping, after) = frag.split_at_ranges(
                        deco.byte_range.start,
                        deco.byte_range.end,
                    );
                    
                    if let Some(before) = before {
                        result.push(before);
                    }
                    
                    if let Some(mut overlapping) = overlapping {
                        overlapping.apply_decorations(deco.visual_attrs);
                        result.push(overlapping);
                    }
                    
                    if let Some(after) = after {
                        fragments[frag_idx] = after;
                        // 继续处理同一片段的剩余部分
                        deco_idx += 1;
                    } else {
                        frag_idx += 1;
                        deco_idx += 1;
                    }
                }
            }
        }
        
        // 添加剩余片段
        result.extend(fragments[frag_idx..].iter().cloned());
        
        result
    }
}
```

### 3. 视觉属性合并算法
**位置**：`visual_attributes.rs` - `VisualAttributes::merge()`

**合并规则**：
```rust
impl VisualAttributes {
    fn merge(&self, other: &VisualAttributes, priority: LayerPriority) -> VisualAttributes {
        let mut result = *self;
        
        // 颜色合并（高优先级覆盖低优先级）
        if priority.should_override(self.foreground.is_some()) {
            result.foreground = other.foreground.or(self.foreground);
        }
        
        if priority.should_override(self.background.is_some()) {
            result.background = other.background.or(self.background);
        }
        
        // 字体合并（累积性）
        result.font_weight = self.font_weight.max(other.font_weight);
        if other.font_style != FontStyle::Normal {
            result.font_style = other.font_style;
        }
        
        // 样式合并（选择性的）
        if other.underline.is_some() {
            result.underline = other.underline;
        }
        
        result.strikethrough = self.strikethrough || other.strikethrough;
        
        // 透明度叠加
        result.opacity = self.opacity * other.opacity;
        
        result
    }
}
```

### 4. 增量更新优化算法
**位置**：`delta_builder.rs` - `DeltaBuilder::compute_delta()`

**算法目标**：计算新旧快照的最小差异，支持部分更新
```rust
struct ViewModelDelta {
    // 行级更新
    updated_lines: Vec<LineUpdate>,
    inserted_lines: Vec<(usize, RenderedLine)>,
    deleted_lines: Range<usize>,
    
    // 装饰更新
    updated_decorations: Option<Arc<DecorationSet>>,
    
    // 元数据更新
    metadata_changed: bool,
}

impl DeltaBuilder {
    fn compute_delta(
        old_snapshot: &ViewModelSnapshot,
        new_snapshot: &ViewModelSnapshot,
    ) -> ViewModelDelta {
        let mut delta = ViewModelDelta::empty();
        
        // 1. 检查行范围变化
        if old_snapshot.viewport_range != new_snapshot.viewport_range {
            delta = self.handle_viewport_shift(old_snapshot, new_snapshot);
        } else {
            // 2. 逐行比较内容
            delta.updated_lines = self.compare_lines(old_snapshot, new_snapshot);
        }
        
        // 3. 检查装饰变化
        if old_snapshot.decorations.version() != new_snapshot.decorations.version() {
            delta.updated_decorations = Some(new_snapshot.decorations.clone());
        }
        
        // 4. 检查光标/选区变化
        delta.metadata_changed = self.metadata_changed(old_snapshot, new_snapshot);
        
        delta
    }
    
    fn compare_lines(
        &self,
        old: &ViewModelSnapshot,
        new: &ViewModelSnapshot,
    ) -> Vec<LineUpdate> {
        let mut updates = Vec::new();
        
        // 并行比较（假设行数相同，范围相同）
        for i in 0..old.lines.len() {
            if !self.lines_equal(&old.lines[i], &new.lines[i]) {
                updates.push(LineUpdate {
                    line_index: i,
                    old_line: old.lines[i].clone(),
                    new_line: new.lines[i].clone(),
                });
            }
        }
        
        updates
    }
    
    fn lines_equal(&self, a: &RenderedLine, b: &RenderedLine) -> bool {
        // 快速路径：比较哈希值
        if a.content_hash() != b.content_hash() {
            return false;
        }
        
        // 慢速路径：比较视觉属性
        a.visual_spans == b.visual_spans &&
        a.visual_attrs == b.visual_attrs
    }
}
```

## 🧩 子系统实现

### 1. SnapshotCache（快照缓存）
**位置**：`snapshot_cache.rs`
**职责**：缓存历史快照，支持时间旅行和增量构建

**设计要点**：
```rust
struct SnapshotCache {
    // 最近快照（LRU）
    recent_snapshots: LruCache<SnapshotId, Arc<ViewModelSnapshot>>,
    
    // 增量构建的基线快照
    baseline_snapshot: Option<Arc<ViewModelSnapshot>>,
    
    // 统计信息
    hit_count: usize,
    miss_count: usize,
}

impl SnapshotCache {
    fn get_or_build(
        &mut self,
        viewport_data: ViewportData,
        decorations: &DecorationSet,
        config: &RenderConfig,
    ) -> Arc<ViewModelSnapshot> {
        // 1. 尝试通过key查找缓存
        let key = self.compute_cache_key(&viewport_data, decorations, config);
        if let Some(cached) = self.recent_snapshots.get(&key) {
            self.hit_count += 1;
            return cached.clone();
        }
        
        // 2. 缓存未命中，构建新快照
        self.miss_count += 1;
        let baseline = self.select_baseline(&viewport_data);
        
        let snapshot = if let Some(baseline) = baseline {
            // 增量构建
            self.builder.incremental_build(baseline, viewport_data, decorations, config)
        } else {
            // 全量构建
            self.builder.full_build(viewport_data, decorations, config)
        };
        
        // 3. 缓存结果
        let snapshot_arc = Arc::new(snapshot);
        self.recent_snapshots.put(key, snapshot_arc.clone());
        
        snapshot_arc
    }
    
    fn compute_cache_key(
        &self,
        viewport_data: &ViewportData,
        decorations: &DecorationSet,
        config: &RenderConfig,
    ) -> CacheKey {
        // 组合多个因素：范围、装饰版本、配置哈希
        CacheKey {
            range: viewport_data.visible_range,
            decoration_version: decorations.version(),
            config_hash: config.hash(),
            text_hash: viewport_data.text_hash(),
        }
    }
}
```

### 2. DecorationManager（装饰管理器）
**位置**：`decoration_manager.rs`
**职责**：协调多个装饰源，管理装饰生命周期

**关键特性**：
- **装饰源注册**：语法、搜索、选区等分别注册
- **版本管理**：每个装饰源有独立版本号
- **范围跟踪**：只重新计算受影响范围的装饰
- **冲突解决**：定义装饰层优先级和覆盖规则

### 3. VisualSpanOptimizer（视觉片段优化器）
**位置**：`span_optimizer.rs`
**职责**：优化视觉片段表示，减少渲染调用

**优化策略**：
1. **相邻合并**：相同样式的相邻片段合并
2. **空白压缩**：连续空白字符的特殊处理
3. **属性去重**：共享相同视觉属性的片段
4. **字形预计算**：高频文本的预渲染字形

### 4. SnapshotSerializer（快照序列化）
**位置**：`serializer.rs`
**职责**：快照的序列化和反序列化，用于调试和持久化

**设计考虑**：
- **调试输出**：人类可读格式，便于问题诊断
- **二进制格式**：高效的网络传输和文件存储
- **差分序列化**：只序列化变化部分

## 🧪 测试策略

### 单元测试覆盖
```rust
#[cfg(test)]
mod tests {
    // 1. 装饰合成测试
    test_decoration_layering()
    test_visual_attributes_merge()
    test_decoration_conflict_resolution()
    
    // 2. 增量构建测试
    test_incremental_build_small_change()
    test_incremental_build_viewport_shift()
    test_delta_computation_correctness()
    
    // 3. 性能特性测试
    test_snapshot_clone_performance()
    test_decoration_synthesis_performance()
    test_memory_usage_bounds()
    
    // 4. 边界条件测试
    test_empty_viewport()
    test_unicode_boundary_handling()
    test_large_line_splitting()
}
```

### 可视化测试
```rust
// 快照可视化比较测试
fn visualize_snapshot_differences(
    old: &ViewModelSnapshot,
    new: &ViewModelSnapshot,
) -> DiffVisualization {
    let delta = DeltaBuilder::compute_delta(old, new);
    
    // 生成HTML或图片格式的可视化差异
    DiffVisualization {
        line_diffs: delta.updated_lines.iter().map(|update| {
            LineDiff {
                line_number: update.line_index,
                old_visual: render_line_visual(&update.old_line),
                new_visual: render_line_visual(&update.new_line),
            }
        }).collect(),
        
        metadata_diffs: if delta.metadata_changed {
            Some(compare_metadata(old, new))
        } else {
            None
        },
    }
}
```

### 性能测试
```rust
#[bench]
fn bench_full_snapshot_build(b: &mut Bencher) {
    let data = create_viewport_data(100); // 100行
    let decorations = create_complex_decorations();
    let config = RenderConfig::default();
    
    b.iter(|| {
        let builder = ViewModelBuilder::new();
        builder.full_build(data.clone(), &decorations, &config)
    });
}

#[bench]
fn bench_incremental_update(b: &mut Bencher) {
    let old_snapshot = create_test_snapshot();
    let mut new_data = old_snapshot.viewport_data().clone();
    
    // 模拟小修改（第5行变化）
    new_data.set_line_text(5, "modified text");
    
    b.iter(|| {
        let builder = ViewModelBuilder::new();
        builder.incremental_build(&old_snapshot, new_data.clone(), &decorations, &config)
    });
}

#[bench]
fn bench_decoration_composition(b: &mut Bencher) {
    let compositor = DecorationCompositor::new();
    let line_text = "fn main() { println!(\"Hello\"); }";
    let decorations = create_rust_syntax_decorations();
    
    b.iter(|| {
        compositor.compose_line(line_text, 0, &decorations)
    });
}
```

## 🔄 维护指南

### 代码组织原则
1. **纯函数核心**：核心转换算法是纯函数，易于测试
2. **快照不可变**：所有快照相关结构都是`#[derive(Clone)]` + `Arc`
3. **装饰可插拔**：通过trait定义装饰层接口
4. **配置驱动**：所有行为通过配置控制，无硬编码

### 监控指标
```rust
struct ViewModelMetrics {
    // 构建性能
    build_times: Histogram<Duration>,
    incremental_build_ratio: f32,
    
    // 内存使用
    snapshot_size_bytes: usize,
    avg_lines_per_snapshot: usize,
    
    // 缓存效率
    cache_hit_rate: f32,
    delta_compression_ratio: f32,
    
    // 装饰处理
    decoration_layer_count: usize,
    avg_decorations_per_line: f32,
}

impl ViewModelMetrics {
    fn check_health(&self) -> Option<HealthWarning> {
        if self.build_times.p95() > Duration::from_millis(50) {
            Some(HealthWarning::SlowSnapshotBuild)
        } else if self.snapshot_size_bytes > 100 * 1024 * 1024 {
            Some(HealthWarning::LargeSnapshotSize)
        } else if self.cache_hit_rate < 0.5 {
            Some(HealthWarning::LowCacheEfficiency)
        } else {
            None
        }
    }
}
```

### 调试支持
```rust
// 快照调试信息
impl ViewModelSnapshot {
    fn debug_info(&self) -> SnapshotDebugInfo {
        SnapshotDebugInfo {
            id: self.id,
            version: self.version,
            line_count: self.lines.len(),
            total_size_bytes: self.estimated_size(),
            
            // 详细的行信息
            line_samples: self.lines.iter()
                .step_by(self.lines.len() / 10) // 采样10行
                .map(|line| line.debug_info())
                .collect(),
            
            // 装饰信息
            decoration_summary: self.decorations.debug_summary(),
        }
    }
    
    fn visualize(&self) -> SnapshotVisualization {
        // 生成HTML或文本可视化
        let mut html = String::new();
        html.push_str("<div class='snapshot'>\n");
        
        for line in &self.lines {
            html.push_str(&format!("<div class='line'>"));
            for span in &line.visual_spans {
                let style = span.visual_attrs.to_css();
                html.push_str(&format!(
                    "<span style='{}'>{}</span>",
                    style,
                    escape_html(&span.text)
                ));
            }
            html.push_str("</div>\n");
        }
        
        html.push_str("</div>");
        SnapshotVisualization::Html(html)
    }
}

// 快照差异调试
fn debug_snapshot_differences(a: &ViewModelSnapshot, b: &ViewModelSnapshot) -> String {
    let delta = DeltaBuilder::compute_delta(a, b);
    
    let mut output = String::new();
    output.push_str(&format!("Snapshot Delta (A v{} -> B v{})\n", a.version, b.version));
    
    if !delta.updated_lines.is_empty() {
        output.push_str(&format!("Updated lines: {:?}\n", 
            delta.updated_lines.iter().map(|u| u.line_index).collect::<Vec<_>>()));
    }
    
    if let Some(decorations) = &delta.updated_decorations {
        output.push_str(&format!("Decorations updated: {:?}\n", decorations.version()));
    }
    
    if delta.metadata_changed {
        output.push_str("Metadata changed\n");
    }
    
    output
}
```

---

*本文档是ViewModel系统的实现指南，实施时可进行优化但不违反架构约束。*
```

---

## 3. **API层文档**：API参考和使用示例

```markdown
# ViewModel系统API参考文档

## 📋 文档信息
- **版本**：1.0
- **状态**：API稳定（可扩展）
- **关联模块**：`crate::core::viewmodel`

## 🎯 快速开始

### 基本使用
```rust
use zedit_core::viewmodel::*;
use zedit_core::viewport::{Viewport, ViewportData};

// 1. 创建ViewModelBuilder
let mut builder = ViewModelBuilder::new();

// 2. 准备输入数据
let viewport_data = ViewportData {
    visible_range: LineRange::new(0, 100),
    lines: vec![/* 逻辑行数据 */],
    dirty_range: None,
};

let decorations = DecorationSet::empty();
let config = RenderConfig::default();

// 3. 构建快照（全量构建）
let snapshot = builder.full_build(viewport_data, &decorations, &config);

// 4. 使用快照（只读）
for line in snapshot.lines() {
    println!("Line {}: {}", line.line_number(), line.text());
    
    for span in line.visual_spans() {
        println!("  Span: '{}' {:?}", span.text(), span.visual_attrs());
    }
}

// 5. 增量更新
let mut new_data = viewport_data.clone();
new_data.lines[42] = LineData::new("modified line");
new_data.dirty_range = Some(LineRange::new(42, 43));

let new_snapshot = builder.incremental_build(&snapshot, new_data, &decorations, &config);
```

### 完整编辑器集成示例
```rust
struct EditorViewModelPipeline {
    builder: ViewModelBuilder,
    cache: SnapshotCache,
    decoration_manager: DecorationManager,
    config: RenderConfig,
    
    current_snapshot: Option<Arc<ViewModelSnapshot>>,
    pending_updates: VecDeque<ViewModelUpdate>,
}

impl EditorViewModelPipeline {
    fn process_viewport_update(
        &mut self,
        viewport: &Viewport,
        editor: &dyn EditorCore,
    ) -> Option<Arc<ViewModelSnapshot>> {
        // 1. 获取视口数据
        let queries = viewport.generate_queries();
        let viewport_data = self.collect_viewport_data(queries, editor);
        
        // 2. 获取装饰信息
        let decorations = self.decoration_manager.current_decorations();
        
        // 3. 构建或获取缓存的快照
        let snapshot = self.cache.get_or_build(
            viewport_data,
            &decorations,
            &self.config,
        );
        
        // 4. 计算与前一个快照的差异（用于增量渲染）
        let delta = if let Some(prev) = &self.current_snapshot {
            Some(DeltaBuilder::compute_delta(prev, &snapshot))
        } else {
            None
        };
        
        // 5. 存储当前快照
        self.current_snapshot = Some(snapshot.clone());
        
        // 6. 返回快照和差异信息
        Some(snapshot)
    }
    
    fn collect_viewport_data(
        &self,
        queries: Vec<ViewportQuery>,
        editor: &dyn EditorCore,
    ) -> ViewportData {
        let mut all_lines = Vec::new();
        let mut visible_range = LineRange::empty();
        
        for query in queries {
            if query.priority == QueryPriority::Immediate {
                let data = editor.query_viewport(query);
                visible_range = data.visible_range;
                all_lines.extend(data.lines);
            }
        }
        
        ViewportData {
            visible_range,
            lines: all_lines,
            dirty_range: None, // 从EditorStateSnapshot获取
        }
    }
}
```

## 📖 API参考

### 核心结构体

#### `ViewModelSnapshot` - 渲染快照
```rust
impl ViewModelSnapshot {
    /// 获取快照ID
    pub fn id(&self) -> SnapshotId
    
    /// 获取版本号
    pub fn version(&self) -> u64
    
    /// 获取视口范围
    pub fn viewport_range(&self) -> LineRange
    
    /// 获取总行数
    pub fn total_lines(&self) -> usize
    
    /// 获取渲染行（只读）
    pub fn lines(&self) -> &[RenderedLine]
    
    /// 获取指定行
    pub fn line_at(&self, index: usize) -> Option<&RenderedLine>
    
    /// 通过行号查找（需在可见范围内）
    pub fn line_by_number(&self, line_number: usize) -> Option<&RenderedLine>
    
    /// 获取装饰集合
    pub fn decorations(&self) -> &DecorationSet
    
    /// 获取光标状态
    pub fn cursor(&self) -> Option<&CursorState>
    
    /// 获取选区状态
    pub fn selection(&self) -> Option<&SelectionState>
    
    /// 获取元数据
    pub fn metadata(&self) -> &SnapshotMetadata
    
    /// 估计内存占用
    pub fn estimated_size(&self) -> usize
    
    /// 克隆为Arc（廉价克隆）
    pub fn clone_arc(&self) -> Arc<ViewModelSnapshot>
    
    /// 调试信息
    pub fn debug_info(&self) -> SnapshotDebugInfo
}
```

#### `RenderedLine` - 渲染行
```rust
impl RenderedLine {
    /// 获取行号
    pub fn line_number(&self) -> usize
    
    /// 获取逻辑文本
    pub fn logical_text(&self) -> &str
    
    /// 获取视觉片段
    pub fn visual_spans(&self) -> &[VisualSpan]
    
    /// 获取行高
    pub fn line_height(&self) -> f32
    
    /// 获取基线偏移
    pub fn baseline_offset(&self) -> f32
    
    /// 获取行宽
    pub fn line_width(&self) -> f32
    
    /// 是否折叠
    pub fn is_folded(&self) -> bool
    
    /// 是否有断点
    pub fn has_breakpoint(&self) -> bool
    
    /// 是否已修改
    pub fn is_changed(&self) -> bool
    
    /// 获取可见文本（考虑折叠）
    pub fn visible_text(&self) -> &str
    
    /// 获取字形位置（如果已计算）
    pub fn glyph_positions(&self) -> Option<&[GlyphPosition]>
    
    /// 在指定列获取视觉属性
    pub fn visual_attrs_at_column(&self, column: usize) -> Option<VisualAttributes>
}
```

#### `VisualSpan` - 视觉片段
```rust
impl VisualSpan {
    /// 获取片段文本
    pub fn text(&self) -> &str
    
    /// 获取视觉属性
    pub fn visual_attrs(&self) -> VisualAttributes
    
    /// 获取字节范围
    pub fn byte_range(&self) -> Range<usize>
    
    /// 获取列范围
    pub fn column_range(&self) -> Range<usize>
    
    /// 获取片段长度（字符数）
    pub fn char_len(&self) -> usize
    
    /// 获取片段宽度
    pub fn width(&self) -> f32
    
    /// 应用额外装饰（返回新片段）
    pub fn with_additional_attrs(&self, attrs: VisualAttributes) -> VisualSpan
    
    /// 分割片段
    pub fn split_at_byte(&self, byte_offset: usize) -> Option<(VisualSpan, VisualSpan)>
    
    /// 分割片段（按字符）
    pub fn split_at_char(&self, char_offset: usize) -> Option<(VisualSpan, VisualSpan)>
}
```

### 构建器API

#### `ViewModelBuilder`
```rust
impl ViewModelBuilder {
    /// 创建新构建器
    pub fn new() -> Self
    
    /// 全量构建快照
    pub fn full_build(
        &self,
        viewport_data: ViewportData,
        decorations: &DecorationSet,
        config: &RenderConfig,
    ) -> ViewModelSnapshot
    
    /// 增量构建快照
    pub fn incremental_build(
        &self,
        previous: &ViewModelSnapshot,
        viewport_data: ViewportData,
        decorations: &DecorationSet,
        config: &RenderConfig,
    ) -> ViewModelSnapshot
    
    /// 设置构建选项
    pub fn with_options(&mut self, options: BuildOptions) -> &mut Self
    
    /// 启用/禁用增量构建
    pub fn enable_incremental(&mut self, enabled: bool) -> &mut Self
    
    /// 设置增量构建阈值
    pub fn set_incremental_threshold(&mut self, threshold: IncrementalThreshold) -> &mut Self
}

struct BuildOptions {
    pub optimize_for_rendering: bool,
    pub precompute_glyphs: bool,
    pub merge_adjacent_spans: bool,
    pub compress_whitespace: bool,
    pub max_line_length: Option<usize>,
}
```

#### `DeltaBuilder`
```rust
impl DeltaBuilder {
    /// 计算两个快照之间的差异
    pub fn compute_delta(
        old: &ViewModelSnapshot,
        new: &ViewModelSnapshot,
    ) -> ViewModelDelta
    
    /// 应用差异到快照（创建新快照）
    pub fn apply_delta(
        &self,
        snapshot: &ViewModelSnapshot,
        delta: &ViewModelDelta,
    ) -> ViewModelSnapshot
    
    /// 合并多个差异
    pub fn merge_deltas(deltas: &[ViewModelDelta]) -> ViewModelDelta
    
    /// 检查差异是否为空
    pub fn is_delta_empty(delta: &ViewModelDelta) -> bool
    
    /// 获取差异影响的区域
    pub fn affected_range(delta: &ViewModelDelta) -> Option<LineRange>
}

struct ViewModelDelta {
    // 差异信息
    pub updated_lines: Vec<LineUpdate>,
    pub inserted_lines: Vec<(usize, RenderedLine)>,
    pub deleted_lines: Range<usize>,
    pub updated_decorations: Option<Arc<DecorationSet>>,
    pub metadata_changed: bool,
}
```

### 装饰系统API

#### `DecorationSet`
```rust
impl DecorationSet {
    /// 创建空装饰集
    pub fn empty() -> Self
    
    /// 添加装饰层
    pub fn add_layer(&mut self, layer: Arc<dyn DecorationLayer>) -> &mut Self
    
    /// 移除装饰层
    pub fn remove_layer(&mut self, layer_id: LayerId) -> bool
    
    /// 获取装饰层
    pub fn layer(&self, layer_id: LayerId) -> Option<&Arc<dyn DecorationLayer>>
    
    /// 获取所有装饰层
    pub fn layers(&self) -> Vec<&Arc<dyn DecorationLayer>>
    
    /// 获取版本号（所有层的组合版本）
    pub fn version(&self) -> u64
    
    /// 获取指定行的合并装饰
    pub fn decorations_for_line(&self, line: usize) -> Arc<[Decoration]>
    
    /// 获取受影响范围
    pub fn affected_range(&self) -> Option<LineRange>
    
    /// 清空所有装饰
    pub fn clear(&mut self)
    
    /// 克隆装饰集
    pub fn clone_without_cache(&self) -> Self
}
```

#### `DecorationLayer` trait
```rust
trait DecorationLayer {
    /// 层标识
    fn id(&self) -> LayerId;
    
    /// 层名称（调试用）
    fn name(&self) -> &str;
    
    /// 层优先级（0-255，越大优先级越高）
    fn priority(&self) -> u8;
    
    /// 获取指定行的装饰
    fn decorations_for_line(&self, line: usize) -> Option<Arc<[Decoration]>>;
    
    /// 获取受影响的范围
    fn affected_range(&self) -> Option<LineRange>;
    
    /// 获取版本号（用于检测变化）
    fn version(&self) -> u64;
    
    /// 配置信息
    fn config(&self) -> &LayerConfig;
}
```

### 缓存API

#### `SnapshotCache`
```rust
impl SnapshotCache {
    /// 创建缓存
    pub fn new(capacity: usize) -> Self
    
    /// 获取或构建快照
    pub fn get_or_build(
        &mut self,
        viewport_data: ViewportData,
        decorations: &DecorationSet,
        config: &RenderConfig,
    ) -> Arc<ViewModelSnapshot>
    
    /// 手动添加快照到缓存
    pub fn put(
        &mut self,
        key: CacheKey,
        snapshot: Arc<ViewModelSnapshot>,
    )
    
    /// 从缓存获取快照（不构建）
    pub fn get(&self, key: &CacheKey) -> Option<Arc<ViewModelSnapshot>>
    
    /// 使缓存失效
    pub fn invalidate(&mut self, range: Option<LineRange>)
    
    /// 清空缓存
    pub fn clear(&mut self)
    
    /// 获取缓存统计
    pub fn stats(&self) -> CacheStats
    
    /// 调整缓存大小
    pub fn resize(&mut self, new_capacity: usize)
}
```

## 🎪 使用示例

### 示例1：自定义装饰层
```rust
// 实现自定义装饰层（例如：TODO高亮）
struct TodoDecorationLayer {
    id: LayerId,
    pattern: Regex,
    color: Color,
    todos: HashMap<usize, Vec<Range<usize>>>,
}

impl DecorationLayer for TodoDecorationLayer {
    fn id(&self) -> LayerId {
        self.id
    }
    
    fn name(&self) -> &str {
        "TODO Highlighter"
    }
    
    fn priority(&self) -> u8 {
        50 // 中等优先级
    }
    
    fn decorations_for_line(&self, line: usize) -> Option<Arc<[Decoration]>> {
        self.todos.get(&line).map(|ranges| {
            ranges.iter()
                .map(|range| Decoration {
                    byte_range: range.clone(),
                    visual_attrs: VisualAttributes {
                        foreground: Some(self.color),
                        background: None,
                        font_weight: FontWeight::Bold,
                        ..Default::default()
                    },
                    layer_priority: self.priority(),
                })
                .collect::<Vec<_>>()
                .into()
        })
    }
    
    fn affected_range(&self) -> Option<LineRange> {
        if self.todos.is_empty() {
            None
        } else {
            let min = *self.todos.keys().min().unwrap();
            let max = *self.todos.keys().max().unwrap();
            Some(LineRange::new(min, max + 1))
        }
    }
    
    fn version(&self) -> u64 {
        // 基于内容和配置计算版本
        let mut hasher = DefaultHasher::new();
        self.pattern.as_str().hash(&mut hasher);
        self.color.hash(&mut hasher);
        self.todos.len().hash(&mut hasher);
        hasher.finish()
    }
}

// 使用自定义装饰层
fn setup_todo_highlighter(viewmodel: &mut ViewModelBuilder) {
    let todo_layer = Arc::new(TodoDecorationLayer::new(
        Regex::new(r"TODO|FIXME|NOTE").unwrap(),
        Color::rgb(255, 200, 0),
    ));
    
    let mut decorations = DecorationSet::empty();
    decorations.add_layer(todo_layer);
    
    viewmodel.with_decorations(decorations);
}
```

### 示例2：快照时间旅行
```rust
// 快照历史管理（支持撤销时的视图回滚）
struct SnapshotHistory {
    snapshots: VecDeque<Arc<ViewModelSnapshot>>,
    current_index: usize,
    max_history: usize,
}

impl SnapshotHistory {
    fn push(&mut self, snapshot: Arc<ViewModelSnapshot>) {
        // 截断当前索引之后的历史
        if self.current_index + 1 < self.snapshots.len() {
            self.snapshots.truncate(self.current_index + 1);
        }
        
        // 添加新快照
        self.snapshots.push_back(snapshot);
        self.current_index = self.snapshots.len() - 1;
        
        // 限制历史大小
        if self.snapshots.len() > self.max_history {
            self.snapshots.pop_front();
            self.current_index -= 1;
        }
    }
    
    fn undo(&mut self) -> Option<&Arc<ViewModelSnapshot>> {
        if self.current_index > 0 {
            self.current_index -= 1;
            Some(&self.snapshots[self.current_index])
        } else {
            None
        }
    }
    
    fn redo(&mut self) -> Option<&Arc<ViewModelSnapshot>> {
        if self.current_index + 1 < self.snapshots.len() {
            self.current_index += 1;
            Some(&self.snapshots[self.current_index])
        } else {
            None
        }
    }
    
    fn current(&self) -> Option<&Arc<ViewModelSnapshot>> {
        self.snapshots.get(self.current_index)
    }
}

// 在编辑器中使用
fn handle_undo(history: &mut SnapshotHistory, renderer: &mut Renderer) {
    if let Some(old_snapshot) = history.undo() {
        // 计算从当前到历史快照的差异
        let current = history.current().unwrap(); // 当前实际上是新的"当前"
        let delta = DeltaBuilder::compute_delta(old_snapshot, current);
        
        // 增量渲染差异
        renderer.apply_delta(&delta);
    }
}
```

### 示例3：性能监控和调优
```rust
// ViewModel性能监控器
struct ViewModelProfiler {
    metrics: Arc<Mutex<ViewModelMetrics>>,
    sampler: MetricsSampler,
}

impl ViewModelProfiler {
    fn record_build(&self, start: Instant, snapshot: &ViewModelSnapshot) {
        let duration = start.elapsed();
        let mut metrics = self.metrics.lock().unwrap();
        
        metrics.build_times.record(duration);
        metrics.snapshot_size_bytes = snapshot.estimated_size();
        metrics.avg_lines_per_snapshot = snapshot.lines().len();
        
        // 性能警告
        if duration > Duration::from_millis(100) {
            self.warn_slow_build(duration, snapshot);
        }
        
        // 采样详细数据
        if self.sampler.should_sample() {
            self.sample_detailed_metrics(snapshot);
        }
    }
    
    fn warn_slow_build(&self, duration: Duration, snapshot: &ViewModelSnapshot) {
        let info = snapshot.debug_info();
        log::warn!(
            "Slow ViewModel build: {:?} for {} lines ({} spans)",
            duration,
            info.line_count,
            info.total_spans()
        );
        
        // 如果持续慢，建议优化
        if self.metrics.lock().unwrap().build_times.p95() > Duration::from_millis(50) {
            self.suggest_optimizations(snapshot);
        }
    }
    
    fn suggest_optimizations(&self, snapshot: &ViewModelSnapshot) {
        let info = snapshot.debug_info();
        let mut suggestions = Vec::new();
        
        // 检查装饰层数量
        if info.decoration_summary.layer_count > 5 {
            suggestions.push("Too many decoration layers, consider merging some");
        }
        
        // 检查平均片段长度
        if info.decoration_summary.avg_span_length < 3.0 {
            suggestions.push("Average visual span length is too short, consider merging adjacent spans");
        }
        
        // 检查快照大小
        if info.total_size_bytes > 50 * 1024 * 1024 {
            suggestions.push("Snapshot size exceeds 50MB, consider enabling compression or reducing visible range");
        }
        
        if !suggestions.is_empty() {
            log::info!("Optimization suggestions: {:?}", suggestions);
        }
    }
}

// 在构建器中集成性能监控
struct InstrumentedViewModelBuilder {
    inner: ViewModelBuilder,
    profiler: ViewModelProfiler,
}

impl InstrumentedViewModelBuilder {
    fn full_build(
        &self,
        viewport_data: ViewportData,
        decorations: &DecorationSet,
        config: &RenderConfig,
    ) -> ViewModelSnapshot {
        let start = Instant::now();
        let snapshot = self.inner.full_build(viewport_data, decorations, config);
        self.profiler.record_build(start, &snapshot);
        snapshot
    }
}
```

## ⚠️ 注意事项

### 性能建议
1. **合理设置缓存大小**：
   ```rust
   // 根据典型使用场景调整
   let cache = SnapshotCache::new(10); // 缓存最近10个快照
   ```

2. **增量构建阈值**：
   ```rust
   // 当变化小于30%时使用增量构建
   builder.set_incremental_threshold(IncrementalThreshold::Percentage(0.3));
   ```

3. **装饰层管理**：
   ```rust
   // 按需启用装饰层
   let mut decorations = DecorationSet::empty();
   if config.syntax_highlight_enabled {
       decorations.add_layer(syntax_layer);
   }
   if config.search_highlight_enabled {
       decorations.add_layer(search_layer);
   }
   ```

### 内存管理
1. **监控快照大小**：
   ```rust
   let size = snapshot.estimated_size();
   if size > 100 * 1024 * 1024 { // 100MB
       log::warn!("Large snapshot detected: {}MB", size / 1024 / 1024);
   }
   ```

2. **适时清理缓存**：
   ```rust
   // 文档关闭时
   fn on_document_close(&mut self) {
       self.cache.clear();
       self.current_snapshot.take();
   }
   
   // 内存警告时
   fn on_memory_warning(&mut self) {
       self.cache.resize(5); // 缩减到5个快照
   }
   ```

3. **避免装饰层泄漏**：
   ```rust
   // 及时移除不需要的装饰层
   decorations.remove_layer(search_layer_id);
   
   // 清空装饰缓存
   decorations.clear();
   ```

### 错误处理
```rust
// 构建失败处理
match builder.full_build(viewport_data, &decorations, &config) {
    Ok(snapshot) => {
        // 成功构建
        self.current_snapshot = Some(Arc::new(snapshot));
    }
    Err(ViewModelError::InvalidRange(range)) => {
        // 无效范围，可能是视图状态不一致
        log::error!("Invalid view range: {:?}", range);
        self.reset_viewport();
    }
    Err(ViewModelError::DecorationConflict(layers)) => {
        // 装饰层冲突，尝试禁用一些层
        log::warn!("Decoration conflict between layers: {:?}", layers);
        self.disable_conflicting_layers(layers);
    }
    Err(e) => {
        log::error!("Failed to build viewmodel: {}", e);
        // 降级到简单渲染
        self.fallback_to_simple_rendering();
    }
}

// 增量构建失败降级到全量构建
fn safe_incremental_build(
    builder: &ViewModelBuilder,
    previous: &ViewModelSnapshot,
    viewport_data: ViewportData,
    decorations: &DecorationSet,
    config: &RenderConfig,
) -> ViewModelSnapshot {
    match builder.incremental_build(previous, viewport_data, decorations, config) {
        Ok(snapshot) => snapshot,
        Err(ViewModelError::IncrementalBuildFailed) => {
            // 增量构建失败，降级到全量构建
            log::warn!("Incremental build failed, falling back to full build");
            builder.full_build(viewport_data, decorations, config)
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}
```

### 调试技巧
```rust
// 启用详细日志
env_logger::Builder::new()
    .filter_module("zedit_core::viewmodel", LevelFilter::Debug)
    .init();

// 快照差异调试
fn debug_snapshot_transition(old: &ViewModelSnapshot, new: &ViewModelSnapshot) {
    let delta = DeltaBuilder::compute_delta(old, new);
    
    if !delta.updated_lines.is_empty() {
        log::debug!("{} lines updated", delta.updated_lines.len());
        for update in &delta.updated_lines {
            log::debug!("  Line {}: '{}' -> '{}'", 
                update.line_index,
                update.old_line.logical_text(),
                update.new_line.logical_text());
        }
    }
    
    if delta.metadata_changed {
        log::debug!("Metadata changed");
    }
    
    // 生成可视化差异
    let diff_viz = visualize_snapshot_differences(old, new);
    diff_viz.save_to_file("snapshot_diff.html");
}

// 性能分析
fn profile_viewmodel_performance(builder: &ViewModelBuilder) -> PerformanceReport {
    let test_cases = vec![
        ("small", create_small_viewport_data()),
        ("medium", create_medium_viewport_data()),
        ("large", create_large_viewport_data()),
    ];
    
    let mut report = PerformanceReport::new();
    
    for (name, data) in test_cases {
        let start = Instant::now();
        let snapshot = builder.full_build(data, &empty_decorations(), &default_config());
        let duration = start.elapsed();
        
        report.add_test_case(name, duration, snapshot.estimated_size());
    }
    
    report
}
```

---

*本文档是ViewModel系统的API参考，所有公共API应保持向后兼容。*
```

---

## **核心洞见总结**

这个ViewModel设计有几个关键特点：

### 1. **快照隔离性**
- 每个`ViewModelSnapshot`是完全独立、不可变的`Arc<T>`
- 可安全跨线程传递，支持并行渲染和异步处理
- 基于引用计数的廉价克隆，支持时间旅行和撤销栈

### 2. **装饰层架构**
- 分层装饰系统：语法、搜索、选区、光标等独立层
- 优先级驱动的冲突解决机制
- 增量装饰更新，只重新计算受影响行

### 3. **增量构建优化**
- 基于脏区的最小化更新
- 智能选择增量vs全量构建
- Delta计算支持最小化渲染工作

### 4. **渲染友好格式**
- 连续内存数组布局，提高缓存局部性
- 预合并视觉片段，减少渲染调用
- 字形位置预计算（可选）

### 5. **全面的性能监控**
- 构建时间统计
- 内存使用分析
- 缓存效率监控
- 自动化优化建议

这个设计完全遵循了架构宪法中的单向数据流原则，ViewModel作为纯函数转换层，将逻辑状态转换为渲染友好的格式，同时保持高性能和大文件友好的特性。