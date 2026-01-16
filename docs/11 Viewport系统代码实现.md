// src/core/viewport/mod.rs
pub mod viewport;
pub mod cache;
pub mod follow;
pub mod scroll;
pub mod sync;
pub mod query;
pub mod layout;

// 重新导出主要类型
pub use viewport::Viewport;
pub use cache::{ViewportCache, CacheStats};
pub use follow::{FollowMode, FollowTarget};
pub use scroll::{ScrollEvent, ScrollHandler};
pub use sync::{SyncResult, ViewportUpdate};
pub use query::{ViewportQuery, QueryPriority};
pub use layout::{PhysicalLayoutCalculator, CoordinateConverter};

// src/core/viewport/viewport.rs
use crate::editor::{EditorCore, EditorStateSnapshot};
use crate::core::logical::{LogicalPosition, LogicalRange, LineRange};
use crate::core::physical::{PhysicalSize, PhysicalPosition};

/// 视口主结构体 - 负责管理可见区域和滚动状态
pub struct Viewport {
    // 状态
    visible_range: LineRange,
    scroll_offset: LogicalPosition,
    viewport_size: PhysicalSize,
    
    // 配置
    config: ViewportConfig,
    
    // 子系统
    cache: ViewportCache,
    follow_controller: FollowController,
    scroll_handler: ScrollHandler,
    sync_manager: SyncManager,
    query_generator: QueryGenerator,
    
    // 性能
    metrics: ViewportMetrics,
    last_update_time: Instant,
}

impl Viewport {
    /// 创建新视口
    pub fn new() -> Self {
        Self {
            visible_range: LineRange::new(0, 0),
            scroll_offset: LogicalPosition::new(0, 0),
            viewport_size: PhysicalSize::new(800.0, 600.0),
            config: ViewportConfig::default(),
            cache: ViewportCache::new(500),
            follow_controller: FollowController::new(),
            scroll_handler: ScrollHandler::new(),
            sync_manager: SyncManager::new(),
            query_generator: QueryGenerator::new(),
            metrics: ViewportMetrics::new(),
            last_update_time: Instant::now(),
        }
    }
    
    /// 与编辑器状态同步
    pub fn sync_with_editor(
        &mut self,
        snapshot: &EditorStateSnapshot,
    ) -> SyncResult {
        let sync_start = Instant::now();
        
        // 1. 检查版本（防止重复处理）
        if snapshot.version <= self.sync_manager.last_sync_version {
            return SyncResult::UpToDate;
        }
        
        // 2. 增量同步
        let sync_result = self.sync_manager.incremental_sync(
            snapshot,
            &self.visible_range,
        );
        
        // 3. 更新缓存（失效受影响区域）
        if let Some(dirty_range) = sync_result.dirty_range() {
            self.cache.invalidate_range(dirty_range);
        }
        
        // 4. 检查是否需要视口跟随
        if let Some(follow_action) = self.follow_controller.should_follow(snapshot) {
            self.apply_follow_action(follow_action);
        }
        
        // 5. 记录性能指标
        self.metrics.record_sync(sync_start.elapsed());
        
        sync_result
    }
    
    /// 处理滚动事件
    pub fn handle_scroll_event(&mut self, event: ScrollEvent) -> ViewportUpdate {
        let scroll_start = Instant::now();
        
        // 1. 处理滚动
        let scroll_result = self.scroll_handler.handle(event, &self.config);
        
        // 2. 更新视口状态
        self.update_from_scroll(&scroll_result);
        
        // 3. 生成查询
        let queries = self.query_generator.generate_queries(
            &self.visible_range,
            &self.config,
        );
        
        // 4. 记录指标
        self.metrics.record_scroll(scroll_start.elapsed());
        
        ViewportUpdate {
            needs_redraw: true,
            dirty_range: Some(self.visible_range),
            scroll_command: scroll_result.command,
            new_queries: queries,
        }
    }
    
    /// 生成数据查询
    pub fn generate_queries(&self) -> Vec<ViewportQuery> {
        let mut queries = Vec::new();
        
        // 1. 可见区域查询（最高优先级）
        queries.push(ViewportQuery {
            request_id: self.generate_request_id(),
            line_range: self.visible_range,
            include_text: true,
            include_metadata: true,
            priority: QueryPriority::Immediate,
        });
        
        // 2. 预加载查询（如果启用）
        if self.config.prefetch_enabled {
            if let Some(prefetch_range) = self.calculate_prefetch_range() {
                queries.push(ViewportQuery {
                    request_id: self.generate_request_id(),
                    line_range: prefetch_range,
                    include_text: true,
                    include_metadata: false,
                    priority: QueryPriority::Prefetch,
                });
            }
        }
        
        queries
    }
    
    /// 确保特定位置可见
    pub fn ensure_visible(
        &mut self,
        position: LogicalPosition,
        mode: EnsureVisibleMode,
    ) -> Option<ScrollCommand> {
        // 1. 检查是否已经在可见区域内
        if self.is_position_visible(position) {
            return None;
        }
        
        // 2. 计算滚动目标
        let target = match mode {
            EnsureVisibleMode::Center => {
                self.calculate_center_scroll(position)
            }
            EnsureVisibleMode::Top => {
                self.calculate_top_scroll(position)
            }
            EnsureVisibleMode::Bottom => {
                self.calculate_bottom_scroll(position)
            }
            EnsureVisibleMode::Smooth => {
                self.calculate_smooth_scroll(position)
            }
        };
        
        // 3. 边界检查
        let clamped_target = self.clamp_scroll_position(target);
        
        // 4. 生成滚动命令
        Some(ScrollCommand {
            target_position: clamped_target,
            animate: self.config.smooth_scroll_enabled,
            duration: self.config.scroll_animation_duration,
        })
    }
    
    /// 获取当前可见范围
    pub fn visible_range(&self) -> LineRange {
        self.visible_range
    }
    
    /// 获取缓存统计
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }
    
    /// 获取性能指标
    pub fn metrics(&self) -> &ViewportMetrics {
        &self.metrics
    }
    
    // 私有辅助方法
    fn update_from_scroll(&mut self, result: &ScrollResult) {
        self.visible_range = result.new_visible_range;
        self.scroll_offset = result.new_scroll_offset;
        self.last_update_time = Instant::now();
    }
    
    fn calculate_prefetch_range(&self) -> Option<LineRange> {
        if self.visible_range.is_empty() {
            return None;
        }
        
        let buffer_lines = self.config.prefetch_buffer_lines;
        let total_lines = self.estimate_total_lines();
        
        // 向前后扩展缓冲区域
        let start = self.visible_range.start.saturating_sub(buffer_lines);
        let end = (self.visible_range.end + buffer_lines).min(total_lines);
        
        // 检查是否需要预加载
        if start < self.visible_range.start || end > self.visible_range.end {
            Some(LineRange::new(start, end))
        } else {
            None
        }
    }
    
    fn is_position_visible(&self, pos: LogicalPosition) -> bool {
        pos.line >= self.visible_range.start && 
        pos.line < self.visible_range.end
    }
    
    fn generate_request_id(&self) -> u64 {
        // 简单递增ID生成
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    }
}

// src/core/viewport/cache.rs
use lru::LruCache;
use std::num::NonZeroUsize;

/// 三级视口缓存
pub struct ViewportCache {
    // L1: 行元数据缓存（常驻）
    metadata_cache: HashMap<usize, LineMetadata>,
    
    // L2: 文本内容缓存（LRU）
    text_cache: LruCache<usize, Arc<str>>,
    
    // L3: 布局结果缓存（可选）
    layout_cache: Option<LruCache<LayoutKey, Arc<LayoutResult>>>,
    
    // 统计信息
    stats: CacheStats,
}

impl ViewportCache {
    pub fn new(text_cache_capacity: usize) -> Self {
        Self {
            metadata_cache: HashMap::new(),
            text_cache: LruCache::new(
                NonZeroUsize::new(text_cache_capacity).unwrap()
            ),
            layout_cache: None,
            stats: CacheStats::new(),
        }
    }
    
    /// 获取或获取文本
    pub fn get_or_fetch_text(
        &mut self,
        line: usize,
        fetch_fn: impl FnOnce() -> String,
    ) -> Arc<str> {
        // 1. 检查缓存
        if let Some(text) = self.text_cache.get(&line) {
            self.stats.record_hit();
            return text.clone();
        }
        
        // 2. 未命中，获取数据
        self.stats.record_miss();
        let text = fetch_fn();
        let arc_text: Arc<str> = Arc::from(text);
        
        // 3. 放入缓存
        self.text_cache.put(line, arc_text.clone());
        
        arc_text
    }
    
    /// 获取行元数据
    pub fn get_metadata(&self, line: usize) -> Option<&LineMetadata> {
        self.metadata_cache.get(&line)
    }
    
    /// 缓存行元数据
    pub fn put_metadata(&mut self, line: usize, metadata: LineMetadata) {
        self.metadata_cache.insert(line, metadata);
    }
    
    /// 使特定范围缓存失效
    pub fn invalidate_range(&mut self, range: LineRange) {
        // 使文本缓存失效
        for line in range.start..range.end {
            self.text_cache.pop(&line);
        }
        
        // 使元数据缓存失效
        self.metadata_cache.retain(|&l, _| !range.contains(l));
        
        // 使布局缓存失效（如果存在）
        if let Some(layout_cache) = &mut self.layout_cache {
            layout_cache.clear(); // 简单实现：清空全部
        }
        
        self.stats.record_invalidation(range.len());
    }
    
    /// 获取缓存统计
    pub fn stats(&self) -> CacheStats {
        self.stats.clone()
    }
}

// src/core/viewport/sync.rs
/// 视口同步管理器
pub struct SyncManager {
    last_sync_version: u64,
    last_dirty_range: Option<LineRange>,
}

impl SyncManager {
    pub fn new() -> Self {
        Self {
            last_sync_version: 0,
            last_dirty_range: None,
        }
    }
    
    /// 增量同步
    pub fn incremental_sync(
        &mut self,
        snapshot: &EditorStateSnapshot,
        current_visible_range: &LineRange,
    ) -> SyncResult {
        // 1. 检查是否有脏区信息
        if let Some(dirty_byte_range) = snapshot.dirty_range {
            // 2. 转换为逻辑行范围（需要行索引）
            let dirty_line_range = self.convert_byte_range_to_lines(
                dirty_byte_range,
                snapshot.line_index.as_ref(),
            );
            
            // 3. 检查是否与可见区域相交
            if let Some(intersection) = dirty_line_range.intersect(current_visible_range) {
                // 4. 部分更新
                self.last_sync_version = snapshot.version;
                self.last_dirty_range = Some(dirty_line_range);
                
                return SyncResult::PartialUpdate {
                    dirty_range: intersection,
                    needs_scroll: self.check_if_needs_scroll(snapshot),
                };
            }
        }
        
        // 5. 非内容变化（光标移动、选区变化等）
        let needs_update = self.check_non_content_changes(snapshot);
        
        if needs_update {
            SyncResult::StateUpdate {
                needs_scroll: self.check_if_needs_scroll(snapshot),
            }
        } else {
            SyncResult::UpToDate
        }
    }
    
    fn convert_byte_range_to_lines(
        &self,
        byte_range: Range<usize>,
        line_index: Option<&LineIndex>,
    ) -> LineRange {
        // 如果有行索引，使用索引转换
        if let Some(index) = line_index {
            let start_line = index.line_at_byte(byte_range.start);
            let end_line = index.line_at_byte(byte_range.end);
            LineRange::new(start_line, end_line)
        } else {
            // 没有索引，粗略估计（每行平均100字节）
            let avg_bytes_per_line = 100;
            let start_line = byte_range.start / avg_bytes_per_line;
            let end_line = (byte_range.end + avg_bytes_per_line - 1) / avg_bytes_per_line;
            LineRange::new(start_line, end_line)
        }
    }
    
    fn check_if_needs_scroll(&self, snapshot: &EditorStateSnapshot) -> bool {
        // 检查光标或选区是否移出当前可见区域
        // 这是一个简化实现
        snapshot.cursor_moved || snapshot.selection_changed
    }
    
    fn check_non_content_changes(&self, snapshot: &EditorStateSnapshot) -> bool {
        // 检查非文本内容的变化
        snapshot.config_changed || 
        snapshot.theme_changed ||
        snapshot.layout_invalidated
    }
}

// src/core/viewport/query.rs
/// 视口查询生成器
pub struct QueryGenerator {
    last_queries: Vec<ViewportQuery>,
    prefetch_predictor: PrefetchPredictor,
}

impl QueryGenerator {
    pub fn new() -> Self {
        Self {
            last_queries: Vec::new(),
            prefetch_predictor: PrefetchPredictor::new(),
        }
    }
    
    pub fn generate_queries(
        &mut self,
        visible_range: &LineRange,
        config: &ViewportConfig,
    ) -> Vec<ViewportQuery> {
        let mut queries = Vec::new();
        
        // 1. 可见区域查询
        queries.push(self.create_visible_query(visible_range));
        
        // 2. 预加载查询（如果启用）
        if config.prefetch_enabled {
            if let Some(prefetch_range) = self.prefetch_predictor.predict(
                visible_range,
                config.prefetch_buffer_lines,
            ) {
                queries.push(self.create_prefetch_query(&prefetch_range));
            }
        }
        
        // 3. 更新历史
        self.last_queries = queries.clone();
        
        queries
    }
    
    fn create_visible_query(&self, range: &LineRange) -> ViewportQuery {
        ViewportQuery {
            request_id: self.generate_id(),
            line_range: *range,
            include_text: true,
            include_metadata: true,
            priority: QueryPriority::Immediate,
            timestamp: Instant::now(),
        }
    }
    
    fn create_prefetch_query(&self, range: &LineRange) -> ViewportQuery {
        ViewportQuery {
            request_id: self.generate_id(),
            line_range: *range,
            include_text: true,
            include_metadata: false,
            priority: QueryPriority::Prefetch,
            timestamp: Instant::now(),
        }
    }
    
    fn generate_id(&self) -> u64 {
        // 使用系统时间作为ID基础
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

// src/core/viewport/follow.rs
/// 视口跟随控制器
pub struct FollowController {
    mode: FollowMode,
    last_follow_time: Instant,
    follow_thresholds: FollowThresholds,
    follow_history: VecDeque<FollowEvent>,
}

impl FollowController {
    pub fn new() -> Self {
        Self {
            mode: FollowMode::Cursor,
            last_follow_time: Instant::now(),
            follow_thresholds: FollowThresholds::default(),
            follow_history: VecDeque::with_capacity(100),
        }
    }
    
    /// 检查是否需要跟随
    pub fn should_follow(
        &mut self,
        snapshot: &EditorStateSnapshot,
    ) -> Option<FollowAction> {
        // 1. 检查模式
        if self.mode == FollowMode::None {
            return None;
        }
        
        // 2. 检查时间防抖
        let time_since_last = self.last_follow_time.elapsed();
        if time_since_last < self.follow_thresholds.debounce_time {
            return None;
        }
        
        // 3. 根据模式检查特定条件
        match self.mode {
            FollowMode::Cursor => self.check_cursor_follow(snapshot),
            FollowMode::Selection => self.check_selection_follow(snapshot),
            FollowMode::SearchResult => self.check_search_follow(snapshot),
            _ => None,
        }
    }
    
    fn check_cursor_follow(&mut self, snapshot: &EditorStateSnapshot) -> Option<FollowAction> {
        if !snapshot.cursor_moved {
            return None;
        }
        
        let cursor_pos = snapshot.cursor_position?;
        
        // 检查距离阈值
        let needs_follow = self.calculate_follow_need(cursor_pos);
        
        if needs_follow {
            self.record_follow_event(FollowEvent::cursor(cursor_pos));
            Some(FollowAction::make_visible(
                cursor_pos,
                self.follow_thresholds.follow_mode,
            ))
        } else {
            None
        }
    }
    
    fn calculate_follow_need(&self, target: LogicalPosition) -> bool {
        // 简化实现：总是跟随
        // 实际实现应该考虑当前位置和视口边缘的距离
        true
    }
    
    fn record_follow_event(&mut self, event: FollowEvent) {
        self.follow_history.push_back(event);
        if self.follow_history.len() > 100 {
            self.follow_history.pop_front();
        }
        self.last_follow_time = Instant::now();
    }
}