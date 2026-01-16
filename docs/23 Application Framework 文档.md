好的，按照统一格式，提供Application Framework的完整文档体系：

# **Application Framework 文档体系**

```
docs/
├── 架构层/
│   └── application-framework-design.md      # 设计理念和架构位置
├── 实现层/
│   └── application-framework-implementation.md  # 实现细节和优化
├── API层/
│   └── application-framework-api.md         # API参考和使用示例
└── 优化层/
    └── application-framework-optimizations.md  # 性能优化记录
```

---

## 1. **架构层文档**：Application Framework设计理念

```markdown
# Application Framework 架构设计文档

## 📋 文档信息
- **版本**：1.0
- **状态**：已冻结
- **关联文档**：[架构宪法] · [数据流规范] · [Action System文档] · [Slint集成文档]

## 🎯 设计目标

### 核心定位
Application Framework是zedit编辑器的**系统协调中枢**，负责：
1. **系统集成**：集成所有子系统，建立完整的数据流管道
2. **事件循环**：管理Slint主循环与编辑器事件流的同步
3. **状态协调**：协调全局应用状态和各子系统状态
4. **生命周期**：管理应用启动、运行、关闭的全过程
5. **错误边界**：提供统一的错误处理和恢复机制

### 设计哲学
1. **胶水而非核心**：只负责协调，不实现业务逻辑
2. **最小化状态**：保持框架本身状态最少
3. **显式依赖**：所有依赖关系必须显式声明
4. **可测试**：支持依赖注入和模拟测试
5. **可观察**：所有状态变化可监控和调试

## 🏗️ 架构位置

### 在系统中的作用
```
┌─────────────────────────────────────┐
│    Application Framework            │ ← 本文档对象
├─────────────────────────────────────┤
│   • 子系统协调                      │
│   • Slint主循环集成                 │
│   • 全局状态管理                    │
│   • 错误边界处理                    │
└─────────────────────────────────────┘
            │
            ▼
┌─────────────────────────────────────┐
│          Subsystem Registry          │
├─────────────────────────────────────┤
│   Input → Action → Editor → Viewport │
│      → ViewModel → Layout → Render   │
└─────────────────────────────────────┘
```

### 数据流协调角色
- **事件流协调**：Slint事件 → Input System → Action System → Editor Core
- **状态流协调**：Editor Core → Viewport → ViewModel → Layout → Render → Slint
- **异步协调**：管理Search、Syntax Highlight等异步任务
- **错误流协调**：统一错误处理和恢复

## 📊 核心设计决策

### 已冻结决策
1. **主循环架构**：基于Slint事件循环的同步+异步混合模式
2. **子系统注册表**：显式注册和依赖管理
3. **状态同步策略**：状态变化监听与批量更新
4. **错误处理策略**：分层错误处理与优雅降级
5. **配置管理**：运行时配置热重载

### 与其他组件的关系
| 组件 | 与Application Framework的关系 | 通信方式 |
|------|-----------------------------|----------|
| Slint UI | 事件源和渲染目标 | SlintEvent / SlintCommand |
| Input System | 事件处理第一站 | InputEvent |
| Editor Core | 核心状态源 | EditorStateSnapshot |
| Viewport System | 可见性协调者 | ViewportQuery/Data |
| 所有子系统 | 注册和生命周期管理 | SubsystemHandle |

## 🔧 设计约束

### 必须遵守的约束
1. **无业务逻辑**：只协调，不实现编辑器功能
2. **最小状态**：不存储编辑器数据状态
3. **单向协调**：事件和状态流严格单向
4. **同步保证**：关键路径必须同步，异步任务有进度反馈
5. **可测试性**：支持全模拟测试

### 性能目标
| 操作 | 目标响应时间 | 备注 |
|------|-------------|------|
| 主循环迭代 | <1ms | 60FPS基础 |
| 事件处理 | <2ms | 输入到首帧渲染 |
| 状态同步 | <5ms | 批量更新优化 |
| 子系统启动 | <50ms | 按需懒加载 |
| 错误恢复 | <100ms | 不影响用户体验 |

## 📈 演进原则

### 允许的演进
1. **性能优化**：更智能的批量更新策略
2. **监控增强**：更详细的性能监控
3. **配置扩展**：更灵活的子系统配置
4. **错误处理**：更健壮的错误恢复机制

### 禁止的演进
1. **业务逻辑**：不添加任何编辑器功能逻辑
2. **状态存储**：不存储编辑器数据
3. **循环依赖**：不引入子系统间循环依赖
4. **平台耦合**：不引入平台特定代码

## 🔗 核心接口定义

### 必须实现的接口
```rust
// 应用框架核心接口
trait ApplicationFramework {
    /// 启动应用
    fn run(&mut self) -> Result<(), AppError>;
    
    /// 停止应用
    fn shutdown(&mut self) -> Result<(), AppError>;
    
    /// 注册子系统
    fn register_subsystem(&mut self, subsystem: Box<dyn Subsystem>) -> SubsystemHandle;
    
    /// 获取子系统
    fn get_subsystem<T: Subsystem + 'static>(&self, handle: SubsystemHandle) -> Option<&T>;
    
    /// 发送事件到子系统
    fn send_event(&self, event: AppEvent, target: SubsystemHandle) -> Result<(), AppError>;
    
    /// 广播事件
    fn broadcast_event(&self, event: AppEvent) -> Result<(), AppError>;
}

// 子系统接口
trait Subsystem {
    /// 子系统名称
    fn name(&self) -> &'static str;
    
    /// 初始化
    fn init(&mut self, context: &SubsystemContext) -> Result<(), SubsystemError>;
    
    /// 启动
    fn start(&mut self) -> Result<(), SubsystemError>;
    
    /// 停止
    fn stop(&mut self) -> Result<(), SubsystemError>;
    
    /// 处理事件
    fn handle_event(&mut self, event: &AppEvent) -> Result<(), SubsystemError>;
    
    /// 获取状态
    fn get_state(&self) -> SubsystemState;
}

// 应用事件
enum AppEvent {
    // UI事件
    UiEvent(SlintEvent),
    
    // 编辑器事件
    EditorAction(EditorAction),
    EditorStateChanged(EditorStateSnapshot),
    
    // 系统事件
    ConfigChanged(AppConfig),
    ThemeChanged(ThemeConfig),
    FontChanged(FontConfig),
    
    // 生命周期事件
    AppStarting,
    AppStarted,
    AppStopping,
    AppStopped,
    
    // 错误事件
    ErrorOccurred(AppError),
    
    // 自定义事件
    Custom { type_id: TypeId, data: Box<dyn Any> },
}
```

### 禁止的接口
```rust
// 禁止直接操作编辑器状态
fn modify_editor_state_directly(state: &mut EditorState)  // ❌

// 禁止跳过子系统直接通信
fn direct_communication(subsystem1: &mut Subsystem, subsystem2: &mut Subsystem) // ❌

// 禁止存储编辑器业务数据
fn store_editor_data(&mut self, data: EditorData) // ❌
```

---

*本文档定义了Application Framework的架构角色和设计约束，所有实现必须遵守。*
```

---

## 2. **实现层文档**：Application Framework实现细节

```markdown
# Application Framework 实现规范文档

## 📋 文档信息
- **版本**：1.0
- **状态**：实施指南（可优化）
- **关联代码**：`src/app/framework.rs` · `src/app/lifecycle.rs` · `src/app/subsystem.rs`

## 🏗️ 核心数据结构

### 1. 应用框架主结构
```rust
/// 应用框架核心实现
pub struct ApplicationFrameworkImpl {
    /// 子系统注册表
    subsystems: SubsystemRegistry,
    
    /// 事件总线
    event_bus: EventBus,
    
    /// 状态管理器
    state_manager: StateManager,
    
    /// 配置管理器
    config_manager: ConfigManager,
    
    /// 错误处理器
    error_handler: ErrorHandler,
    
    /// 性能监控器
    performance_monitor: PerformanceMonitor,
    
    /// Slint集成器
    slint_integrator: SlintIntegrator,
    
    /// 生命周期状态
    lifecycle_state: Arc<Mutex<LifecycleState>>,
    
    /// 运行时配置
    runtime_config: Arc<RwLock<RuntimeConfig>>,
}

/// 子系统注册表
struct SubsystemRegistry {
    subsystems: HashMap<SubsystemId, Box<dyn Subsystem>>,
    dependencies: HashMap<SubsystemId, Vec<SubsystemId>>,
    startup_order: Vec<SubsystemId>,
    shutdown_order: Vec<SubsystemId>,
    
    // 按类型索引，便于快速查找
    type_index: HashMap<TypeId, SubsystemId>,
}

/// 子系统ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubsystemId(uuid::Uuid);

impl SubsystemId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

/// 子系统句柄
#[derive(Debug, Clone, Copy)]
pub struct SubsystemHandle {
    id: SubsystemId,
    name: &'static str,
}
```

### 2. 事件系统
```rust
/// 事件总线
struct EventBus {
    // 事件队列（按优先级）
    event_queues: [VecDeque<AppEvent>; 4],
    
    // 事件监听器
    listeners: HashMap<EventType, Vec<EventListener>>,
    
    // 事件过滤器
    filters: Vec<EventFilter>,
    
    // 事件统计
    statistics: EventStatistics,
    
    // 事件日志
    event_log: Option<EventLog>,
}

/// 事件优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    Critical = 0,    // 关键事件（错误、崩溃）
    High = 1,        // 高优先级（用户输入、渲染）
    Normal = 2,      // 普通优先级（状态更新）
    Low = 3,         // 低优先级（后台任务）
}

/// 事件监听器
struct EventListener {
    subsystem_id: SubsystemId,
    callback: Box<dyn Fn(&AppEvent) -> Result<(), SubsystemError> + Send + Sync>,
    filter: Option<EventFilter>,
}

/// 事件统计
struct EventStatistics {
    total_events: AtomicU64,
    events_by_type: HashMap<EventType, AtomicU64>,
    average_processing_time: MovingAverage<Duration>,
    peak_queue_size: AtomicUsize,
}

/// 事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    // UI事件
    MouseEvent,
    KeyboardEvent,
    ScrollEvent,
    ResizeEvent,
    
    // 编辑器事件
    InsertText,
    DeleteText,
    MoveCursor,
    ChangeSelection,
    
    // 文件事件
    FileOpen,
    FileSave,
    FileChanged,
    
    // 系统事件
    ConfigChange,
    ThemeChange,
    FontChange,
    
    // 生命周期
    Startup,
    Shutdown,
    Suspend,
    Resume,
    
    // 错误事件
    Error,
    Warning,
    Info,
}
```

### 3. 状态管理
```rust
/// 状态管理器
struct StateManager {
    // 全局状态快照
    global_state: Arc<RwLock<GlobalState>>,
    
    // 状态监听器
    state_listeners: HashMap<StateType, Vec<StateListener>>,
    
    // 状态历史（用于调试）
    state_history: Option<StateHistory>,
    
    // 状态同步器
    state_synchronizer: StateSynchronizer,
}

/// 全局状态
#[derive(Debug, Clone)]
pub struct GlobalState {
    // 应用状态
    app_state: AppState,
    
    // 编辑器状态（来自Editor Core）
    editor_state: Option<EditorStateSnapshot>,
    
    // 视口状态
    viewport_state: ViewportState,
    
    // 配置状态
    config_state: ConfigState,
    
    // UI状态
    ui_state: UIState,
    
    // 性能状态
    performance_state: PerformanceState,
    
    // 错误状态
    error_state: ErrorState,
}

/// 应用状态
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Starting,      // 启动中
    Running,       // 运行中
    Suspended,     // 暂停（如失去焦点）
    ShuttingDown,  // 正在关闭
    Error(ErrorState), // 错误状态
}

/// 状态监听器
struct StateListener {
    subsystem_id: SubsystemId,
    callback: Box<dyn Fn(&GlobalState, StateChange) -> Result<(), SubsystemError> + Send + Sync>,
    filter: Option<StateFilter>,
}

/// 状态变更
#[derive(Debug, Clone)]
pub struct StateChange {
    pub changed_fields: Vec<StateField>,
    pub old_state: GlobalState,
    pub new_state: GlobalState,
    pub timestamp: Instant,
}
```

### 4. 配置管理
```rust
/// 配置管理器
struct ConfigManager {
    // 配置文件路径
    config_path: PathBuf,
    
    // 当前配置
    current_config: Arc<RwLock<AppConfig>>,
    
    // 配置监听器
    config_listeners: Vec<ConfigListener>,
    
    // 配置历史
    config_history: Vec<AppConfig>,
    
    // 配置验证器
    config_validator: ConfigValidator,
    
    // 配置热重载监视器
    hot_reload_watcher: Option<ConfigWatcher>,
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // 窗口配置
    pub window: WindowConfig,
    
    // 编辑器配置
    pub editor: EditorConfig,
    
    // 视图配置
    pub view: ViewConfig,
    
    // 主题配置
    pub theme: ThemeConfig,
    
    // 字体配置
    pub font: FontConfig,
    
    // 性能配置
    pub performance: PerformanceConfig,
    
    // 高级配置
    pub advanced: AdvancedConfig,
}

/// 配置监听器
struct ConfigListener {
    subsystem_id: SubsystemId,
    callback: Box<dyn Fn(&AppConfig, ConfigChange) -> Result<(), SubsystemError> + Send + Sync>,
    watched_keys: Vec<ConfigKey>,
}
```

### 5. 错误处理
```rust
/// 错误处理器
struct ErrorHandler {
    // 错误队列
    error_queue: VecDeque<AppError>,
    
    // 错误处理策略
    error_strategies: HashMap<ErrorType, ErrorStrategy>,
    
    // 错误监听器
    error_listeners: Vec<ErrorListener>,
    
    // 错误统计
    error_statistics: ErrorStatistics,
    
    // 错误恢复器
    error_recoverer: ErrorRecoverer,
}

/// 应用错误
#[derive(Debug, Clone)]
pub struct AppError {
    pub error_type: ErrorType,
    pub message: String,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
    pub subsystem: Option<SubsystemId>,
    pub severity: ErrorSeverity,
    pub timestamp: Instant,
    pub context: ErrorContext,
}

/// 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    // 子系统错误
    SubsystemInitFailed,
    SubsystemStartFailed,
    SubsystemStopFailed,
    
    // 事件处理错误
    EventProcessingFailed,
    EventDispatchFailed,
    
    // 状态错误
    StateCorruption,
    StateSyncFailed,
    
    // 配置错误
    ConfigLoadFailed,
    ConfigSaveFailed,
    ConfigValidationFailed,
    
    // 文件系统错误
    FileOpenFailed,
    FileSaveFailed,
    FileReadFailed,
    
    // 内存错误
    OutOfMemory,
    BufferOverflow,
    
    // 渲染错误
    RenderingFailed,
    GpuError,
    
    // 用户错误
    UserCanceled,
    InvalidInput,
}

/// 错误处理策略
enum ErrorStrategy {
    // 立即重试
    RetryImmediately { max_retries: u32 },
    
    // 延迟重试
    RetryWithBackoff { max_retries: u32, backoff: Duration },
    
    // 降级处理
    Degrade { fallback: Box<dyn Fn() -> Result<(), AppError>> },
    
    // 忽略（仅日志）
    Ignore,
    
    // 致命错误，需要关闭
    Fatal,
}
```

## ⚙️ 核心算法实现

### 1. 主循环算法
```rust
impl ApplicationFrameworkImpl {
    /// 主循环
    fn main_loop(&mut self) -> Result<(), AppError> {
        // 1. 初始化
        self.initialize()?;
        
        // 2. 启动Slint事件循环
        self.slint_integrator.run_event_loop(|slint_event| {
            // 3. 处理Slint事件
            self.process_slint_event(slint_event)?;
            
            // 4. 处理事件队列
            self.process_event_queue()?;
            
            // 5. 状态同步
            self.synchronize_states()?;
            
            // 6. 性能监控
            self.update_performance_metrics()?;
            
            // 7. 错误处理
            self.handle_errors()?;
            
            // 8. 渲染（通过Slint）
            self.request_render()?;
            
            Ok(())
        })?;
        
        // 9. 清理
        self.cleanup()?;
        
        Ok(())
    }
    
    fn process_event_queue(&mut self) -> Result<(), AppError> {
        // 按优先级处理事件
        for priority in EventPriority::iter() {
            let queue = &mut self.event_bus.event_queues[priority as usize];
            
            // 每帧限制处理的事件数量
            let max_events_per_frame = self.runtime_config.read().max_events_per_frame;
            let mut processed = 0;
            
            while let Some(event) = queue.pop_front() {
                // 处理事件
                self.dispatch_event(&event)?;
                
                processed += 1;
                if processed >= max_events_per_frame {
                    // 本帧达到限制，剩余事件下帧处理
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    fn dispatch_event(&mut self, event: &AppEvent) -> Result<(), AppError> {
        // 应用事件过滤器
        if !self.should_process_event(event) {
            return Ok(());
        }
        
        // 查找事件监听器
        let event_type = event.get_type();
        if let Some(listeners) = self.event_bus.listeners.get(&event_type) {
            for listener in listeners {
                // 检查过滤器
                if let Some(filter) = &listener.filter {
                    if !filter.matches(event) {
                        continue;
                    }
                }
                
                // 调用监听器回调
                if let Err(err) = (listener.callback)(event) {
                    self.handle_subsystem_error(listener.subsystem_id, err)?;
                }
            }
        }
        
        // 更新事件统计
        self.update_event_statistics(event);
        
        Ok(())
    }
}
```

### 2. 子系统生命周期管理
```rust
impl ApplicationFrameworkImpl {
    /// 初始化所有子系统
    fn initialize_subsystems(&mut self) -> Result<(), AppError> {
        // 1. 计算启动顺序（基于依赖关系）
        let startup_order = self.calculate_startup_order();
        
        // 2. 按顺序初始化
        for subsystem_id in &startup_order {
            let subsystem = self.subsystems.get_mut(*subsystem_id)
                .ok_or_else(|| AppError::subsystem_not_found(*subsystem_id))?;
            
            let context = SubsystemContext {
                framework: self.create_context(),
                config: self.config_manager.get_config_for_subsystem(*subsystem_id),
                event_bus: self.event_bus.create_sender(),
                state_manager: self.state_manager.create_accessor(),
            };
            
            // 初始化子系统
            if let Err(err) = subsystem.init(&context) {
                self.handle_subsystem_error(*subsystem_id, err)?;
                // 根据错误策略决定是否继续
            }
        }
        
        // 3. 按顺序启动
        for subsystem_id in &startup_order {
            let subsystem = self.subsystems.get_mut(*subsystem_id)
                .ok_or_else(|| AppError::subsystem_not_found(*subsystem_id))?;
            
            // 启动子系统
            if let Err(err) = subsystem.start() {
                self.handle_subsystem_error(*subsystem_id, err)?;
            }
        }
        
        Ok(())
    }
    
    /// 计算启动顺序（拓扑排序）
    fn calculate_startup_order(&self) -> Vec<SubsystemId> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();
        
        for subsystem_id in self.subsystems.keys() {
            if !visited.contains(subsystem_id) {
                self.topological_sort(
                    *subsystem_id,
                    &mut visited,
                    &mut temp_visited,
                    &mut order,
                ).unwrap_or_else(|_| {
                    // 检测到循环依赖，使用默认顺序
                    order.extend(self.subsystems.keys().copied());
                });
            }
        }
        
        order.reverse();
        order
    }
    
    fn topological_sort(
        &self,
        node: SubsystemId,
        visited: &mut HashSet<SubsystemId>,
        temp_visited: &mut HashSet<SubsystemId>,
        order: &mut Vec<SubsystemId>,
    ) -> Result<(), ()> {
        if temp_visited.contains(&node) {
            // 检测到循环依赖
            return Err(());
        }
        
        if visited.contains(&node) {
            return Ok(());
        }
        
        temp_visited.insert(node);
        
        // 先处理依赖
        if let Some(dependencies) = self.subsystems.dependencies.get(&node) {
            for &dep in dependencies {
                self.topological_sort(dep, visited, temp_visited, order)?;
            }
        }
        
        temp_visited.remove(&node);
        visited.insert(node);
        order.push(node);
        
        Ok(())
    }
}
```

### 3. 状态同步算法
```rust
impl ApplicationFrameworkImpl {
    /// 同步所有子系统状态
    fn synchronize_states(&mut self) -> Result<(), AppError> {
        // 1. 收集状态变更
        let state_changes = self.collect_state_changes()?;
        
        if state_changes.is_empty() {
            return Ok(());
        }
        
        // 2. 合并状态变更
        let merged_change = self.merge_state_changes(state_changes)?;
        
        // 3. 应用状态变更到全局状态
        self.apply_state_change(&merged_change)?;
        
        // 4. 通知状态监听器
        self.notify_state_listeners(&merged_change)?;
        
        // 5. 触发相关更新
        self.trigger_updates_based_on_state(&merged_change)?;
        
        Ok(())
    }
    
    fn collect_state_changes(&self) -> Result<Vec<StateChange>, AppError> {
        let mut changes = Vec::new();
        
        // 从所有子系统收集状态变更
        for (subsystem_id, subsystem) in &self.subsystems.subsystems {
            let subsystem_state = subsystem.get_state();
            
            // 检查是否有状态变更
            if let Some(change) = self.detect_state_change(*subsystem_id, subsystem_state) {
                changes.push(change);
            }
        }
        
        Ok(changes)
    }
    
    fn merge_state_changes(&self, changes: Vec<StateChange>) -> Result<StateChange, AppError> {
        if changes.is_empty() {
            return Err(AppError::no_state_changes());
        }
        
        if changes.len() == 1 {
            return Ok(changes[0].clone());
        }
        
        // 合并多个状态变更
        let mut merged = changes[0].clone();
        
        for change in &changes[1..] {
            // 合并变更字段
            for field in &change.changed_fields {
                if !merged.changed_fields.contains(field) {
                    merged.changed_fields.push(field.clone());
                }
            }
            
            // 更新新状态（旧状态保持不变）
            merged.new_state = self.merge_states(&merged.new_state, &change.new_state)?;
        }
        
        Ok(merged)
    }
}
```

### 4. 错误恢复算法
```rust
impl ApplicationFrameworkImpl {
    /// 处理错误队列
    fn handle_errors(&mut self) -> Result<(), AppError> {
        let max_errors_per_frame = self.runtime_config.read().max_errors_per_frame;
        let mut processed = 0;
        
        while let Some(error) = self.error_handler.error_queue.pop_front() {
            // 选择错误处理策略
            let strategy = self.select_error_strategy(&error);
            
            match strategy {
                ErrorStrategy::RetryImmediately { max_retries } => {
                    self.handle_with_retry(&error, max_retries, Duration::ZERO)?;
                }
                
                ErrorStrategy::RetryWithBackoff { max_retries, backoff } => {
                    self.handle_with_retry(&error, max_retries, backoff)?;
                }
                
                ErrorStrategy::Degrade { fallback } => {
                    // 执行降级处理
                    if let Err(fallback_err) = fallback() {
                        // 降级也失败，升级错误级别
                        self.escalate_error(&error, fallback_err)?;
                    }
                }
                
                ErrorStrategy::Ignore => {
                    // 仅记录日志
                    self.log_error(&error);
                }
                
                ErrorStrategy::Fatal => {
                    // 致命错误，启动关闭流程
                    self.initiate_shutdown(&error)?;
                    break;
                }
            }
            
            processed += 1;
            if processed >= max_errors_per_frame {
                break;
            }
        }
        
        Ok(())
    }
    
    fn select_error_strategy(&self, error: &AppError) -> &ErrorStrategy {
        // 1. 根据错误类型选择策略
        if let Some(strategy) = self.error_handler.error_strategies.get(&error.error_type) {
            return strategy;
        }
        
        // 2. 根据严重程度选择策略
        match error.severity {
            ErrorSeverity::Fatal => &ErrorStrategy::Fatal,
            ErrorSeverity::Error => &ErrorStrategy::RetryWithBackoff {
                max_retries: 3,
                backoff: Duration::from_millis(100),
            },
            ErrorSeverity::Warning => &ErrorStrategy::Degrade {
                fallback: Box::new(|| Ok(())), // 空降级
            },
            ErrorSeverity::Info => &ErrorStrategy::Ignore,
        }
    }
    
    fn handle_with_retry(
        &mut self,
        error: &AppError,
        max_retries: u32,
        backoff: Duration,
    ) -> Result<(), AppError> {
        let mut retry_count = 0;
        let mut current_backoff = backoff;
        
        while retry_count < max_retries {
            // 重试原操作
            match self.retry_operation(error) {
                Ok(_) => {
                    // 重试成功
                    self.log_recovery(&format!("错误恢复成功，重试次数：{}", retry_count + 1));
                    return Ok(());
                }
                Err(retry_error) => {
                    retry_count += 1;
                    
                    if retry_count < max_retries {
                        // 等待后重试
                        std::thread::sleep(current_backoff);
                        current_backoff *= 2; // 指数退避
                    } else {
                        // 重试失败，升级错误
                        return self.escalate_error(error, retry_error);
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

## 🧩 子系统实现

### 1. Slint集成器模块
**位置**：`src/app/slint_integrator.rs`
**职责**：
- 管理Slint窗口和事件循环
- 转换Slint事件为应用事件
- 执行渲染命令
- 处理平台特定集成

**关键设计**：
```rust
struct SlintIntegrator {
    /// Slint窗口
    window: slint::Window,
    
    /// UI组件树
    ui_components: UiComponentTree,
    
    /// 事件转换器
    event_converter: EventConverter,
    
    /// 渲染队列
    render_queue: RenderQueue,
    
    /// 平台适配器
    platform_adapter: PlatformAdapter,
    
    /// 性能监控
    rendering_stats: RenderingStatistics,
}

impl SlintIntegrator {
    /// 运行Slint事件循环
    fn run_event_loop<F>(&mut self, mut frame_callback: F) -> Result<(), AppError>
    where
        F: FnMut(SlintEvent) -> Result<(), AppError>,
    {
        self.window.run_event_loop(move |event| {
            // 1. 转换Slint事件
            let app_event = self.event_converter.convert(event);
            
            // 2. 调用框架回调
            if let Err(err) = frame_callback(app_event) {
                // 错误处理
                self.handle_frame_error(err);
                return slint::EventLoopResult::Exit;
            }
            
            // 3. 处理渲染队列
            self.process_render_queue()?;
            
            slint::EventLoopResult::Continue
        })
    }
    
    /// 处理渲染命令
    fn process_render_queue(&mut self) -> Result<(), AppError> {
        while let Some(command) = self.render_queue.pop_front() {
            match command {
                RenderCommand::UpdateText { component_id, text } => {
                    self.update_text_component(component_id, text)?;
                }
                
                RenderCommand::UpdateStyle { component_id, style } => {
                    self.update_component_style(component_id, style)?;
                }
                
                RenderCommand::InvalidateRegion { rect } => {
                    self.invalidate_region(rect)?;
                }
                
                RenderCommand::RequestRedraw => {
                    self.request_redraw()?;
                }
                
                // 其他渲染命令...
            }
        }
        
        Ok(())
    }
}
```

### 2. 事件总线模块
**位置**：`src/app/event_bus.rs`
**设计特点**：
- 多优先级事件队列
- 事件过滤和路由
- 事件统计和监控
- 事件重放支持（用于调试）

**事件路由**：
```rust
struct EventBusImpl {
    // 按优先级的事件队列
    queues: [VecDeque<AppEvent>; 4],
    
    // 事件路由表
    routing_table: HashMap<EventType, Vec<SubsystemId>>,
    
    // 事件过滤器链
    filter_chain: Vec<Box<dyn EventFilter>>,
    
    // 事件分发器
    dispatcher: EventDispatcher,
    
    // 事件缓存（用于重放）
    event_cache: Option<EventCache>,
}

impl EventBusImpl {
    /// 发布事件
    fn publish(&mut self, event: AppEvent, priority: EventPriority) {
        // 应用过滤器
        let event = self.apply_filters(event);
        
        // 添加到对应优先级的队列
        self.queues[priority as usize].push_back(event);
        
        // 更新统计
        self.update_statistics(&event);
        
        // 缓存事件（如果启用）
        if let Some(cache) = &mut self.event_cache {
            cache.add_event(event);
        }
    }
    
    /// 订阅事件
    fn subscribe(
        &mut self,
        event_type: EventType,
        subsystem_id: SubsystemId,
        callback: EventCallback,
        filter: Option<EventFilter>,
    ) {
        let listener = EventListener {
            subsystem_id,
            callback: Box::new(callback),
            filter,
        };
        
        self.routing_table
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(subsystem_id);
            
        // 添加到监听器列表
        self.dispatcher.add_listener(event_type, listener);
    }
    
    /// 分发事件到所有订阅者
    fn dispatch(&mut self, event: &AppEvent) -> Result<(), AppError> {
        let event_type = event.get_type();
        
        if let Some(subscriber_ids) = self.routing_table.get(&event_type) {
            for &subscriber_id in subscriber_ids {
                // 获取子系统
                // 在实际实现中，这里需要通过框架获取子系统
                
                // 调用回调
                // self.dispatcher.dispatch_to(subscriber_id, event)?;
            }
        }
        
        Ok(())
    }
}
```

### 3. 状态管理器模块
**位置**：`src/app/state_manager.rs`
**设计特点**：
- 全局状态原子更新
- 状态变更监听
- 状态历史记录
- 状态验证和回滚

**状态同步**：
```rust
struct StateManagerImpl {
    /// 当前全局状态
    current_state: Arc<RwLock<GlobalState>>,
    
    /// 状态监听器
    listeners: HashMap<StateField, Vec<StateListener>>,
    
    /// 状态验证器
    validators: Vec<Box<dyn StateValidator>>,
    
    /// 状态历史（用于撤销/重做或调试）
    history: StateHistory,
    
    /// 状态同步锁
    sync_lock: StateSyncLock,
}

impl StateManagerImpl {
    /// 更新状态
    fn update_state<F>(&mut self, updater: F) -> Result<StateChange, AppError>
    where
        F: FnOnce(&mut GlobalState) -> Result<(), StateError>,
    {
        // 获取写锁
        let mut state = self.current_state.write()
            .map_err(|_| AppError::state_lock_failed())?;
        
        // 保存旧状态
        let old_state = (*state).clone();
        
        // 应用更新
        if let Err(err) = updater(&mut state) {
            return Err(AppError::state_update_failed(err));
        }
        
        // 验证新状态
        for validator in &self.validators {
            if let Err(err) = validator.validate(&state) {
                // 状态无效，回滚
                *state = old_state.clone();
                return Err(AppError::state_validation_failed(err));
            }
        }
        
        // 创建状态变更记录
        let change = StateChange {
            old_state: old_state.clone(),
            new_state: (*state).clone(),
            changed_fields: self.detect_changed_fields(&old_state, &state),
            timestamp: Instant::now(),
        };
        
        // 添加到历史
        self.history.add_change(change.clone());
        
        // 通知监听器
        self.notify_listeners(&change)?;
        
        Ok(change)
    }
    
    /// 检测变更的字段
    fn detect_changed_fields(&self, old: &GlobalState, new: &GlobalState) -> Vec<StateField> {
        let mut changed = Vec::new();
        
        if old.app_state != new.app_state {
            changed.push(StateField::AppState);
        }
        
        if old.editor_state != new.editor_state {
            changed.push(StateField::EditorState);
        }
        
        if old.viewport_state != new.viewport_state {
            changed.push(StateField::ViewportState);
        }
        
        if old.config_state != new.config_state {
            changed.push(StateField::ConfigState);
        }
        
        // 其他字段比较...
        
        changed
    }
}
```

### 4. 配置管理器模块
**位置**：`src/app/config_manager.rs`
**设计特点**：
- 配置热重载
- 配置验证
- 配置版本管理
- 子系统配置隔离

**配置热重载**：
```rust
struct ConfigManagerImpl {
    /// 配置文件监视器
    watcher: ConfigWatcher,
    
    /// 当前配置
    current: Arc<RwLock<AppConfig>>,
    
    /// 配置监听器
    listeners: Vec<ConfigListener>,
    
    /// 配置验证器
    validator: ConfigValidator,
    
    /// 配置备份
    backups: ConfigBackupManager,
}

impl ConfigManagerImpl {
    /// 启动配置热重载
    fn start_hot_reload(&mut self) -> Result<(), AppError> {
        self.watcher.watch(&self.config_path, move |event| {
            match event {
                ConfigWatchEvent::Created(path) => {
                    // 新配置文件
                    self.handle_config_created(path)?;
                }
                
                ConfigWatchEvent::Modified(path) => {
                    // 配置文件修改
                    self.handle_config_modified(path)?;
                }
                
                ConfigWatchEvent::Deleted(path) => {
                    // 配置文件删除
                    self.handle_config_deleted(path)?;
                }
                
                ConfigWatchEvent::Error(err) => {
                    // 监视错误
                    self.handle_watch_error(err)?;
                }
            }
            
            Ok(())
        })?;
        
        Ok(())
    }
    
    /// 处理配置文件修改
    fn handle_config_modified(&mut self, path: PathBuf) -> Result<(), AppError> {
        // 1. 加载新配置
        let new_config = self.load_config(&path)?;
        
        // 2. 验证配置
        if let Err(err) = self.validator.validate(&new_config) {
            self.handle_invalid_config(err, &new_config)?;
            return Ok(());
        }
        
        // 3. 备份当前配置
        self.backups.create_backup(&self.current.read().clone())?;
        
        // 4. 应用新配置
        let old_config = self.current.read().clone();
        *self.current.write()? = new_config.clone();
        
        // 5. 通知监听器
        let change = ConfigChange {
            old_config,
            new_config,
            changed_keys: self.detect_changed_keys(&old_config, &new_config),
            source: ConfigChangeSource::FileWatch,
        };
        
        self.notify_listeners(&change)?;
        
        Ok(())
    }
    
    /// 检测变更的配置键
    fn detect_changed_keys(&self, old: &AppConfig, new: &AppConfig) -> Vec<ConfigKey> {
        let mut changed = Vec::new();
        
        if old.window != new.window {
            changed.push(ConfigKey::Window);
        }
        
        if old.editor != new.editor {
            changed.push(ConfigKey::Editor);
        }
        
        if old.theme != new.theme {
            changed.push(ConfigKey::Theme);
        }
        
        // 其他配置比较...
        
        changed
    }
}
```

## 🧪 测试策略

### 集成测试覆盖
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_full_application_lifecycle() {
        // 1. 创建应用框架
        let mut app = ApplicationFramework::new();
        
        // 2. 注册所有子系统
        app.register_subsystem(Box::new(MockInputSystem::new()));
        app.register_subsystem(Box::new(MockEditorCore::new()));
        app.register_subsystem(Box::new(MockViewportSystem::new()));
        // ... 其他子系统
        
        // 3. 启动应用
        assert!(app.start().is_ok());
        
        // 4. 模拟用户交互
        app.send_event(AppEvent::simulate_key_press('a')).unwrap();
        app.send_event(AppEvent::simulate_mouse_click(100, 100)).unwrap();
        
        // 5. 验证状态变更
        let state = app.get_global_state();
        assert!(state.editor_state.is_some());
        
        // 6. 停止应用
        assert!(app.stop().is_ok());
    }
    
    #[test]
    fn test_error_recovery_scenarios() {
        let mut app = ApplicationFramework::new();
        
        // 测试各种错误恢复场景
        test_scenarios! {
            // 场景1：子系统初始化失败
            "subsystem_init_failure" => {
                let mut faulty_system = MockSubsystem::new();
                faulty_system.set_init_result(Err(SubsystemError::InitFailed));
                app.register_subsystem(Box::new(faulty_system));
                
                // 应用应该仍然能启动（跳过故障子系统）
                assert!(app.start().is_ok());
            },
            
            // 场景2：事件处理失败
            "event_processing_failure" => {
                // 发送导致错误的事件
                app.send_event(AppEvent::cause_error()).unwrap();
                
                // 错误应该被捕获和处理
                let errors = app.get_error_count();
                assert!(errors > 0);
                
                // 应用应该仍然运行
                assert!(app.is_running());
            },
            
            // 场景3：状态同步失败
            "state_sync_failure" => {
                // 注入损坏的状态
                app.inject_corrupted_state();
                
                // 应用应该检测到并恢复
                assert!(app.recover_from_state_corruption().is_ok());
            },
        }
    }
    
    #[test]
    fn test_performance_benchmarks() {
        let mut app = ApplicationFramework::new();
        
        bench! {
            // 基准测试1：事件处理吞吐量
            "event_processing_throughput" => {
                // 发送大量事件
                for i in 0..1000 {
                    app.send_event(AppEvent::test_event(i)).unwrap();
                }
                
                // 测量处理时间
                let start = Instant::now();
                app.process_event_queue().unwrap();
                let duration = start.elapsed();
                
                assert!(duration < Duration::from_millis(100));
            },
            
            // 基准测试2：状态同步延迟
            "state_sync_latency" => {
                // 触发状态变更
                app.trigger_state_change();
                
                // 测量同步延迟
                let latency = app.measure_state_sync_latency();
                assert!(latency < Duration::from_millis(10));
            },
            
            // 基准测试3：启动时间
            "startup_time" => {
                let start = Instant::now();
                app.start().unwrap();
                let duration = start.elapsed();
                
                assert!(duration < Duration::from_millis(500));
            },
        }
    }
}
```

### 压力测试
```rust
#[test]
fn stress_test_high_event_load() {
    let mut app = ApplicationFramework::new();
    app.start().unwrap();
    
    // 创建大量并发事件
    let event_count = 10_000;
    let start = Instant::now();
    
    // 并行发送事件
    (0..event_count).into_par_iter().for_each(|i| {
        app.send_event(AppEvent::stress_test_event(i)).unwrap();
    });
    
    // 处理所有事件
    while app.get_pending_event_count() > 0 {
        app.process_event_queue().unwrap();
        std::thread::sleep(Duration::from_micros(100));
    }
    
    let duration = start.elapsed();
    let events_per_second = event_count as f64 / duration.as_secs_f64();
    
    println!("压力测试结果：");
    println!("  总事件数：{}", event_count);
    println!("  总时间：{:?}", duration);
    println!("  事件/秒：{:.2}", events_per_second);
    println!("  内存使用：{} MB", app.get_memory_usage_mb());
    
    // 验证没有内存泄漏或状态损坏
    assert!(app.is_state_consistent());
    assert!(events_per_second > 1000.0); // 至少1000事件/秒
}
```

## 🔄 维护指南

### 启动流程检查清单
```rust
impl ApplicationFrameworkImpl {
    fn verify_startup_readiness(&self) -> Result<(), Vec<StartupIssue>> {
        let mut issues = Vec::new();
        
        // 检查1：所有必需子系统已注册
        let required_subsystems = ["InputSystem", "EditorCore", "ViewportSystem"];
        for &name in &required_subsystems {
            if !self.has_subsystem(name) {
                issues.push(StartupIssue::MissingRequiredSubsystem(name));
            }
        }
        
        // 检查2：配置有效
        if let Err(err) = self.config_manager.validate_current_config() {
            issues.push(StartupIssue::InvalidConfiguration(err));
        }
        
        // 检查3：资源可用
        if !self.check_resources_available() {
            issues.push(StartupIssue::InsufficientResources);
        }
        
        // 检查4：依赖关系无循环
        if let Some(cycle) = self.detect_dependency_cycle() {
            issues.push(StartupIssue::CircularDependency(cycle));
        }
        
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}
```

### 监控和日志
```rust
// 性能监控
struct ApplicationMonitor {
    frame_times: CircularBuffer<Duration>,
    event_counts: HashMap<EventType, u64>,
    state_change_counts: HashMap<StateField, u64>,
    error_counts: HashMap<ErrorType, u64>,
    subsystem_status: HashMap<SubsystemId, SubsystemStatus>,
}

impl ApplicationMonitor {
    fn generate_health_report(&self) -> HealthReport {
        HealthReport {
            // 性能指标
            average_frame_time: self.calculate_average_frame_time(),
            event_throughput: self.calculate_event_throughput(),
            state_change_rate: self.calculate_state_change_rate(),
            
            // 稳定性指标
            error_rate: self.calculate_error_rate(),
            subsystem_health: self.assess_subsystem_health(),
            memory_usage: self.get_memory_usage(),
            
            // 建议
            recommendations: self.generate_recommendations(),
            
            // 警告
            warnings: self.generate_warnings(),
        }
    }
    
    fn generate_warnings(&self) -> Vec<HealthWarning> {
        let mut warnings = Vec::new();
        
        // 警告1：帧时间过长
        if self.average_frame_time > Duration::from_millis(16) {
            warnings.push(HealthWarning::HighFrameTime(self.average_frame_time));
        }
        
        // 警告2：错误率过高
        if self.error_rate > 0.01 {
            warnings.push(HealthWarning::HighErrorRate(self.error_rate));
        }
        
        // 警告3：子系统不健康
        for (subsystem_id, status) in &self.subsystem_status {
            if !status.is_healthy {
                warnings.push(HealthWarning::UnhealthySubsystem(
                    *subsystem_id,
                    status.last_error.clone(),
                ));
            }
        }
        
        warnings
    }
}
```

### 调试工具
```rust
/// 应用调试器
struct ApplicationDebugger {
    /// 事件记录器
    event_recorder: EventRecorder,
    
    /// 状态检查点
    state_checkpoints: Vec<StateCheckpoint>,
    
    /// 性能分析器
    profiler: ApplicationProfiler,
    
    /// 内存分析器
    memory_analyzer: MemoryAnalyzer,
    
    /// 依赖关系可视化器
    dependency_visualizer: DependencyVisualizer,
}

impl ApplicationDebugger {
    /// 诊断应用问题
    fn diagnose_issue(&self, symptom: Symptom) -> Diagnosis {
        match symptom {
            Symptom::HighCpuUsage => {
                self.diagnose_high_cpu_usage()
            }
            
            Symptom::HighMemoryUsage => {
                self.diagnose_high_memory_usage()
            }
            
            Symptom::SlowResponse => {
                self.diagnose_slow_response()
            }
            
            Symptom::FrequentCrashes => {
                self.diagnose_frequent_crashes()
            }
            
            Symptom::UIFreezes => {
                self.diagnose_ui_freezes()
            }
        }
    }
    
    fn diagnose_high_cpu_usage(&self) -> Diagnosis {
        // 分析CPU使用情况
        let cpu_profile = self.profiler.capture_cpu_profile();
        
        Diagnosis {
            issue: Issue::HighCpuUsage,
            root_cause: self.find_root_cause(&cpu_profile),
            severity: Severity::High,
            recommendations: vec![
                Recommendation::OptimizeEventProcessing,
                Recommendation::AddEventRateLimiting,
                Recommendation::ProfileSubsystems,
            ],
            affected_subsystems: self.identify_affected_subsystems(&cpu_profile),
        }
    }
}
```

---

*本文档是Application Framework的实现指南，实施时可进行优化但不违反架构约束。*
```

由于篇幅限制，我先提供核心的设计文档。完整的实现代码将包括：

1. **ApplicationFramework主结构**（子系统协调中心）
2. **EventBus实现**（事件路由和分发）
3. **StateManager实现**（全局状态管理）
4. **ConfigManager实现**（配置热重载）
5. **SlintIntegrator实现**（Slint主循环集成）
6. **完整的测试套件和调试工具**

请告诉我您希望我继续提供哪个部分的完整代码实现。