好的，按照统一格式，提供完整的Action System文档体系：

# **Action System 文档体系**

```
docs/
├── 架构层/
│   └── action-system-design.md      # 设计理念和架构位置
├── 实现层/
│   └── action-system-implementation.md  # 实现细节和优化
├── API层/
│   └── action-system-api.md         # API参考和使用示例
└── 优化层/
    └── action-system-optimizations.md  # 性能优化记录
```

---

## 1. **架构层文档**：Action System设计理念

```markdown
# Action System 架构设计文档

## 📋 文档信息
- **版本**：1.0
- **状态**：已冻结
- **关联文档**：[架构宪法] · [数据流规范] · [Editor Core文档] · [Input System文档]

## 🎯 设计目标

### 核心定位
Action System是zedit编辑器的**动作契约系统**，负责：
1. **动作定义**：定义所有可能的编辑操作
2. **动作验证**：验证动作的有效性和安全性
3. **动作转换**：将动作转换为可执行的事务
4. **动作合并**：优化连续动作的性能

### 设计哲学
1. **语义清晰**：每个动作都有明确的语义和边界
2. **安全第一**：所有动作必须经过验证才能执行
3. **可组合性**：复杂动作由简单动作组合而成
4. **可逆性**：每个动作都有明确的逆操作

## 🏗️ 架构位置

### 在系统中的作用
```
┌─────────────────┐   EditorAction   ┌─────────────────┐
│  Input System   │ ───────────────▶ │  Action System  │ ← 本文档对象
├─────────────────┤                  ├─────────────────┤
│  原始输入处理   │                  │  动作管理中枢   │
└─────────────────┘                  └─────────────────┘
                                            │ 已验证动作
                                            ▼
┌─────────────────┐                  ┌─────────────────┐
│  Editor Core    │ ◀────────────────│   执行引擎      │
├─────────────────┤                  └─────────────────┘
│  状态机引擎     │
└─────────────────┘
```

### 数据流角色
- **输入**：接收`EditorAction`（来自Input System或其他系统）
- **输出**：生成`EditTransaction`（给Editor Core执行）
- **内部**：验证、转换、合并动作
- **特点**：**纯函数式转换层**，不持有任何状态

## 📊 核心设计决策

### 已冻结决策
1. **动作分类**：按功能域分类（编辑、光标、文件等）
2. **验证机制**：所有动作必须通过前置验证
3. **事务转换**：动作 -> 原子事务
4. **合并策略**：智能合并连续相似动作

### 与其他组件的关系
| 组件 | 与Action System的关系 | 通信方式 |
|------|-------------------|----------|
| Input System | 动作提供者 | EditorAction |
| Editor Core | 动作执行者 | EditTransaction |
| Undo System | 动作历史记录 | ActionRecord |
| Config System | 动作配置提供者 | ActionConfig |

## 🔧 设计约束

### 必须遵守的约束
1. **无状态性**：动作处理不依赖编辑器状态（除验证外）
2. **原子性**：每个动作产生完整的事务
3. **可测试性**：所有动作可独立测试
4. **可扩展性**：新动作不破坏现有架构

### 性能目标
| 操作 | 目标时间复杂度 | 备注 |
|------|---------------|------|
| 动作验证 | O(1) | 简单状态检查 |
| 事务转换 | O(1) ~ O(n) | 依赖动作复杂度 |
| 动作合并 | O(1) | 检查相邻动作 |
| 批量处理 | O(n) | n=动作数量 |

## 📈 演进原则

### 允许的演进
1. **新动作类型**：添加新的编辑动作
2. **验证增强**：更严格的输入验证
3. **合并优化**：更智能的合并策略
4. **配置扩展**：动作行为可配置

### 禁止的演进
1. **架构变更**：不改变动作-事务转换模式
2. **语义破坏**：不改变现有动作的语义
3. **状态污染**：不引入动作处理状态
4. **平台耦合**：不引入平台特定动作

## 🔗 相关接口定义

### 必须实现的接口
```rust
// 核心接口
trait ActionSystem {
    /// 验证动作是否可执行
    fn validate_action(&self, action: &EditorAction, context: &ActionContext) -> ValidationResult;
    
    /// 将动作转换为事务
    fn action_to_transaction(&self, action: EditorAction, context: &ActionContext) -> EditTransaction;
    
    /// 批量处理动作
    fn batch_process(&self, actions: Vec<EditorAction>, context: &ActionContext) -> Vec<EditTransaction>;
    
    /// 合并连续动作
    fn merge_actions(&self, actions: &[EditorAction]) -> Vec<EditorAction>;
}
```

### 禁止的接口
```rust
// 禁止直接操作编辑器状态
fn modify_state_directly(state: &mut EditorState)  // ❌
fn bypass_validation(action: EditorAction)         // ❌
fn execute_without_transaction(action: EditorAction) // ❌
```

---

*本文档定义了Action System的架构角色和设计约束，所有实现必须遵守。*
```

---

## 2. **实现层文档**：Action System实现细节

```markdown
# Action System 实现规范文档

## 📋 文档信息
- **版本**：1.0
- **状态**：实施指南（可优化）
- **关联代码**：`src/core/action/`

## 🏗️ 核心数据结构

### 1. 动作定义体系
```rust
/// 编辑器动作 - 所有可能操作的完整枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditorAction {
    // === 文件操作 (File) ===
    FileNew,
    FileOpen { path: String },
    FileSave,
    FileSaveAs { path: String },
    FileClose,
    FileReload,
    
    // === 基本编辑 (Edit) ===
    InsertText { text: String },
    DeleteBackward,
    DeleteForward,
    DeleteSelection,
    DeleteLine,
    DeleteWord,
    DeleteToLineStart,
    DeleteToLineEnd,
    Paste { text: String, mode: PasteMode },
    Cut,
    Copy,
    
    // === 光标移动 (Cursor) ===
    MoveCursor { movement: CursorMove },
    SetCursor { position: LogicalPosition },
    Scroll { lines: isize },
    ScrollToCursor,
    ScrollTo { line: usize, column: usize },
    
    // === 选区操作 (Selection) ===
    StartSelection,
    ExtendSelection { movement: CursorMove },
    SetSelection { range: LogicalRange },
    ClearSelection,
    ToggleSelectionMode,
    SelectAll,
    SelectLine,
    SelectWord,
    
    // === 历史操作 (History) ===
    Undo,
    Redo,
    ClearHistory,
    
    // === 编辑模式 (Mode) ===
    EnterInsertMode,
    EnterNormalMode,
    EnterVisualMode,
    EnterCommandMode,
    EnterColumnMode,
    ExitColumnMode,
    ToggleOvertype,
    
    // === 查找替换 (Find) ===
    Find { pattern: String, options: FindOptions },
    FindNext,
    FindPrevious,
    ReplaceCurrent { replacement: String },
    ReplaceAll { pattern: String, replacement: String },
    ClearFindHighlights,
    
    // === 文本操作 (Text) ===
    Indent,
    Unindent,
    ToggleComment,
    ConvertCase { case_type: CaseType },
    SortLines { ascending: bool },
    DeduplicateLines,
    TrimTrailingSpaces,
    DeleteEmptyLines,
    JoinLines,
    SplitLine,
    
    // === 列编辑 (Column) ===
    ColumnDelete,
    ColumnInsert { text: String },
    ColumnReplace { text: String },
    
    // === IME支持 (IME) ===
    ImeStartComposition,
    ImeUpdateComposition { text: String, cursor: usize },
    ImeCancelComposition,
    ImeCommit { text: String },
    
    // === 宏和批量 (Macro) ===
    StartMacroRecording,
    StopMacroRecording,
    PlayMacro,
    ExecuteCommands { commands: Vec<String> },
    
    // === 视图和UI (View) ===
    ZoomIn,
    ZoomOut,
    ResetZoom,
    ToggleLineNumbers,
    ToggleWordWrap,
    ToggleWhitespace,
    ToggleSyntaxHighlighting,
    
    // === 高级编辑 (Advanced) ===
    FormatSelection,
    FormatDocument,
    TransformSelection { transform: TextTransform },
    ExecuteShellCommand { command: String },
    
    // === 配置操作 (Config) ===
    SetOption { key: String, value: ConfigValue },
    ChangeTheme { theme_name: String },
    ChangeFont { font_family: String, font_size: f32 },
}
```

### 2. 动作上下文
```rust
/// 动作执行上下文
#[derive(Debug, Clone)]
pub struct ActionContext {
    /// 当前编辑器状态（只读）
    pub editor_state: Arc<EditorState>,
    
    /// 当前编辑模式
    pub mode: EditMode,
    
    /// 动作来源
    pub source: ActionSource,
    
    /// 时间戳（用于动作合并）
    pub timestamp: Instant,
    
    /// 用户配置
    pub config: Arc<EditorConfig>,
    
    /// 平台信息
    pub platform: PlatformInfo,
}

/// 动作来源
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionSource {
    Keyboard,
    Mouse,
    Menu,
    Toolbar,
    CommandPalette,
    Macro,
    Script,
    Plugin,
}

/// 编辑模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditMode {
    Normal,
    Insert,
    Visual,
    Command,
    ColumnSelect,
    Replace,
}
```

### 3. 验证结果
```rust
/// 动作验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 是否有效
    pub is_valid: bool,
    
    /// 错误信息（如果无效）
    pub error: Option<ValidationError>,
    
    /// 警告信息
    pub warnings: Vec<ValidationWarning>,
    
    /// 建议的修正动作（如果可能）
    pub suggested_fix: Option<EditorAction>,
    
    /// 是否需要用户确认
    pub requires_confirmation: bool,
    
    /// 估计的影响范围
    pub estimated_impact: ImpactLevel,
}

/// 验证错误
#[derive(Debug, Clone)]
pub enum ValidationError {
    InvalidCursorPosition,
    InvalidSelection,
    OutOfBounds,
    ReadOnlyFile,
    FileTooLarge,
    UnsupportedOperation,
    MissingSelection,
    InvalidInput(String),
    ConfigurationError(String),
}

/// 影响级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactLevel {
    None,       // 无影响（如光标移动）
    Minor,      // 小影响（如删除字符）
    Moderate,   // 中等影响（如删除行）
    Major,      // 重大影响（如删除选区）
    Critical,   // 关键影响（如保存文件）
}
```

### 4. 事务定义
```rust
/// 编辑事务
#[derive(Debug, Clone)]
pub struct EditTransaction {
    /// 事务ID（用于跟踪）
    pub id: TransactionId,
    
    /// 原子操作序列
    pub operations: Vec<EditOperation>,
    
    /// 事务前状态
    pub before_state: TransactionState,
    
    /// 事务后状态
    pub after_state: TransactionState,
    
    /// 事务元数据
    pub metadata: TransactionMetadata,
}

/// 编辑操作类型
#[derive(Debug, Clone)]
pub enum EditOperation {
    Insert {
        offset: ByteOffset,
        text: String,
        original_text: Option<String>, // 用于撤销
    },
    Delete {
        range: ByteRange,
        deleted_text: String,
    },
    Replace {
        range: ByteRange,
        old_text: String,
        new_text: String,
    },
    MoveCursor {
        from: LogicalPosition,
        to: LogicalPosition,
    },
    ChangeSelection {
        from: Option<Selection>,
        to: Option<Selection>,
    },
}

/// 事务状态快照
#[derive(Debug, Clone)]
pub struct TransactionState {
    pub cursor: Cursor,
    pub selection: Option<Selection>,
    pub scroll_position: (usize, usize),
    pub version: u64,
}
```

## ⚙️ 核心算法实现

### 1. 动作验证流程
```
输入：EditorAction, ActionContext
输出：ValidationResult

步骤：
1. 基本验证：validate_basic(action, context)
   - 检查动作类型是否支持
   - 检查参数是否有效
   
2. 上下文验证：validate_context(action, context)
   - 检查当前模式是否允许该动作
   - 检查编辑器状态是否允许
   
3. 边界检查：validate_bounds(action, context)
   - 检查光标位置是否有效
   - 检查选区范围是否有效
   
4. 副作用评估：assess_impact(action, context)
   - 评估动作对文件的影响
   - 评估动作对性能的影响
   
5. 生成结果：combine_results(basic, context, bounds, impact)
```

### 2. 动作到事务的转换算法
```rust
fn action_to_transaction(
    action: EditorAction,
    context: &ActionContext,
) -> Result<EditTransaction, ActionError> {
    // 1. 验证动作
    let validation = self.validate_action(&action, context);
    if !validation.is_valid {
        return Err(ActionError::ValidationFailed(validation.error.unwrap()));
    }
    
    // 2. 根据动作类型创建事务
    let transaction = match action {
        EditorAction::InsertText { text } => {
            self.build_insert_transaction(text, context)
        }
        
        EditorAction::DeleteBackward => {
            self.build_delete_backward_transaction(context)
        }
        
        EditorAction::MoveCursor { movement } => {
            self.build_move_cursor_transaction(movement, context)
        }
        
        EditorAction::Undo => {
            self.build_undo_transaction(context)
        }
        
        // ... 其他动作类型
        
        _ => return Err(ActionError::UnsupportedAction),
    };
    
    // 3. 添加元数据
    let transaction = self.add_metadata(transaction, &action, context);
    
    Ok(transaction)
}

fn build_insert_transaction(
    &self,
    text: String,
    context: &ActionContext,
) -> EditTransaction {
    let cursor_pos = context.editor_state.cursor.position;
    let byte_offset = context.editor_state.buffer
        .position_to_offset(cursor_pos)
        .unwrap_or(0);
    
    // 如果有选区，先删除选区
    let mut operations = Vec::new();
    
    if let Some(selection) = &context.editor_state.selection {
        let range = selection.to_byte_range(&context.editor_state.buffer);
        if let Some(range) = range {
            let deleted_text = context.editor_state.buffer
                .get_text_range(range.clone())
                .to_string();
            
            operations.push(EditOperation::Delete {
                range,
                deleted_text,
            });
            
            // 更新插入位置到选区开始
            byte_offset = range.start;
        }
    }
    
    // 添加插入操作
    operations.push(EditOperation::Insert {
        offset: byte_offset,
        text: text.clone(),
        original_text: None,
    });
    
    // 计算新光标位置
    let new_cursor_pos = context.editor_state.buffer
        .offset_to_position(byte_offset + text.len())
        .unwrap_or(cursor_pos);
    
    // 添加光标移动操作
    operations.push(EditOperation::MoveCursor {
        from: cursor_pos,
        to: new_cursor_pos,
    });
    
    // 清除选区
    operations.push(EditOperation::ChangeSelection {
        from: context.editor_state.selection.clone(),
        to: None,
    });
    
    EditTransaction {
        id: TransactionId::new(),
        operations,
        before_state: self.capture_state(context.editor_state),
        after_state: self.compute_after_state(&operations, context),
        metadata: TransactionMetadata::default(),
    }
}
```

### 3. 动作合并算法
```rust
fn merge_actions(&self, actions: &[EditorAction]) -> Vec<EditorAction> {
    if actions.len() <= 1 {
        return actions.to_vec();
    }
    
    let mut merged = Vec::new();
    let mut current = actions[0].clone();
    
    for next in &actions[1..] {
        if self.can_merge(&current, next) {
            current = self.merge_two_actions(current, next.clone());
        } else {
            merged.push(current);
            current = next.clone();
        }
    }
    
    merged.push(current);
    merged
}

fn can_merge(&self, a: &EditorAction, b: &EditorAction) -> bool {
    match (a, b) {
        // 连续文本输入可以合并
        (
            EditorAction::InsertText { text: text_a },
            EditorAction::InsertText { text: text_b },
        ) => {
            // 检查时间间隔（应在外部上下文）
            // 检查是否在同一位置
            true
        }
        
        // 连续删除可以合并
        (
            EditorAction::DeleteBackward,
            EditorAction::DeleteBackward,
        ) => true,
        
        // 连续光标移动可以合并
        (
            EditorAction::MoveCursor { movement: mov_a },
            EditorAction::MoveCursor { movement: mov_b },
        ) => {
            self.are_consecutive_movements(mov_a, mov_b)
        }
        
        _ => false,
    }
}

fn merge_two_actions(&self, a: EditorAction, b: EditorAction) -> EditorAction {
    match (a, b) {
        (
            EditorAction::InsertText { text: mut text_a },
            EditorAction::InsertText { text: text_b },
        ) => {
            text_a.push_str(&text_b);
            EditorAction::InsertText { text: text_a }
        }
        
        // 其他合并逻辑...
        
        _ => a, // 无法合并，返回第一个
    }
}
```

### 4. 批量处理优化
```rust
fn batch_process(
    &self,
    actions: Vec<EditorAction>,
    context: &ActionContext,
) -> Vec<EditTransaction> {
    // 1. 合并连续动作
    let merged_actions = self.merge_actions(&actions);
    
    // 2. 分组处理（按是否可以并行）
    let groups = self.group_actions(&merged_actions);
    
    // 3. 并行验证和转换
    let mut transactions = Vec::with_capacity(groups.len());
    
    for group in groups {
        // 并行处理每个组
        let group_transactions = self.process_action_group(group, context);
        transactions.extend(group_transactions);
    }
    
    // 4. 排序事务（按依赖关系）
    self.sort_transactions(&mut transactions);
    
    transactions
}

fn group_actions(&self, actions: &[EditorAction]) -> Vec<Vec<EditorAction>> {
    let mut groups = Vec::new();
    let mut current_group = Vec::new();
    
    for action in actions {
        if self.can_be_in_same_group(&current_group, action) {
            current_group.push(action.clone());
        } else {
            if !current_group.is_empty() {
                groups.push(current_group);
                current_group = Vec::new();
            }
            current_group.push(action.clone());
        }
    }
    
    if !current_group.is_empty() {
        groups.push(current_group);
    }
    
    groups
}
```

## 🧩 子系统实现

### 1. 动作验证器模块
**位置**：`src/core/action/validator.rs`
**职责**：
- 验证动作的有效性和安全性
- 提供详细的错误信息
- 评估动作影响

**关键设计**：
```rust
struct ActionValidator {
    validators: HashMap<ActionType, Box<dyn ActionValidatorTrait>>,
    impact_analyzer: ImpactAnalyzer,
    safety_checker: SafetyChecker,
}

impl ActionValidator {
    fn validate(&self, action: &EditorAction, context: &ActionContext) -> ValidationResult {
        // 1. 查找对应的验证器
        let validator = self.validators.get(&action.action_type());
        
        // 2. 执行验证
        let result = validator.map_or_else(
            || ValidationResult::valid(),
            |v| v.validate(action, context),
        );
        
        // 3. 安全性检查
        if result.is_valid {
            let safety_result = self.safety_checker.check(action, context);
            if !safety_result.is_safe {
                return ValidationResult::invalid(safety_result.reason);
            }
        }
        
        // 4. 影响评估
        let impact = self.impact_analyzer.analyze(action, context);
        
        ValidationResult {
            is_valid: result.is_valid,
            error: result.error,
            warnings: result.warnings,
            suggested_fix: result.suggested_fix,
            requires_confirmation: impact >= ImpactLevel::Major,
            estimated_impact: impact,
        }
    }
}
```

### 2. 事务构建器模块
**位置**：`src/core/action/builder.rs`
**设计特点**：
- 将动作转换为原子事务
- 处理动作间的依赖关系
- 优化事务性能

**构建器模式**：
```rust
trait TransactionBuilder {
    fn build_transaction(
        &self,
        action: EditorAction,
        context: &ActionContext,
    ) -> Result<EditTransaction, BuildError>;
    
    fn can_build(&self, action: &EditorAction) -> bool;
    
    fn estimate_cost(&self, action: &EditorAction, context: &ActionContext) -> CostEstimate;
}

struct InsertTextBuilder;
impl TransactionBuilder for InsertTextBuilder {
    fn build_transaction(&self, action: EditorAction, context: &ActionContext) -> Result<EditTransaction, BuildError> {
        // 具体实现...
    }
}

struct TransactionBuilderFactory {
    builders: HashMap<ActionType, Box<dyn TransactionBuilder>>,
}

impl TransactionBuilderFactory {
    fn get_builder(&self, action: &EditorAction) -> Option<&dyn TransactionBuilder> {
        self.builders.get(&action.action_type()).map(|b| &**b)
    }
}
```

### 3. 动作合并器模块
**位置**：`src/core/action/merger.rs`
**设计**：智能合并策略 + 时间窗口

**合并策略**：
```rust
struct ActionMerger {
    // 合并配置
    config: MergeConfig,
    
    // 时间窗口管理器
    time_window: TimeWindowManager,
    
    // 合并规则
    rules: Vec<MergeRule>,
}

impl ActionMerger {
    fn merge(&self, actions: &[TimedAction]) -> Vec<EditorAction> {
        let mut merged = Vec::new();
        let mut buffer = Vec::new();
        
        for timed_action in actions {
            buffer.push(timed_action.clone());
            
            // 检查时间窗口
            if !self.time_window.is_within_window(&buffer) {
                // 处理当前缓冲区的动作
                let merged_chunk = self.merge_buffer(&buffer);
                merged.extend(merged_chunk);
                buffer.clear();
            }
        }
        
        // 处理剩余动作
        if !buffer.is_empty() {
            let merged_chunk = self.merge_buffer(&buffer);
            merged.extend(merged_chunk);
        }
        
        merged
    }
    
    fn merge_buffer(&self, buffer: &[TimedAction]) -> Vec<EditorAction> {
        // 应用所有合并规则
        let mut current = buffer.to_vec();
        
        for rule in &self.rules {
            if rule.can_apply(&current) {
                current = rule.apply(current);
            }
        }
        
        current.into_iter().map(|ta| ta.action).collect()
    }
}
```

### 4. 批量处理器模块
**位置**：`src/core/action/batch.rs`
**设计特点**：
- 并行处理能力
- 依赖关系分析
- 内存优化

**并行处理**：
```rust
struct BatchProcessor {
    thread_pool: ThreadPool,
    dependency_analyzer: DependencyAnalyzer,
    memory_manager: MemoryManager,
}

impl BatchProcessor {
    fn process_batch(
        &self,
        actions: Vec<EditorAction>,
        context: Arc<ActionContext>,
    ) -> Vec<EditTransaction> {
        // 1. 分析依赖关系
        let dependency_graph = self.dependency_analyzer.analyze(&actions);
        
        // 2. 分组（可并行执行的动作）
        let groups = self.group_by_dependencies(&actions, &dependency_graph);
        
        // 3. 并行处理每组
        let mut all_transactions = Vec::new();
        
        for group in groups {
            let transactions = self.process_group_parallel(group, context.clone());
            all_transactions.extend(transactions);
        }
        
        // 4. 按原始顺序排序
        self.sort_transactions(&mut all_transactions, &dependency_graph);
        
        all_transactions
    }
}
```

## 🧪 测试策略

### 单元测试覆盖
```rust
#[cfg(test)]
mod tests {
    // 1. 动作验证测试
    test_validation_happy_path()
    test_validation_edge_cases()
    test_validation_error_messages()
    
    // 2. 事务构建测试  
    test_transaction_building()
    test_transaction_integrity()
    test_transaction_undo_redo()
    
    // 3. 动作合并测试
    test_action_merging_rules()
    test_merge_time_windows()
    test_merge_performance()
    
    // 4. 批量处理测试
    test_batch_processing()
    test_concurrent_processing()
    test_memory_usage()
    
    // 5. 集成测试
    test_full_action_pipeline()
    test_action_error_recovery()
    test_action_performance_benchmarks()
}
```

### 属性测试
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_action_validation_properties(
        action in any::<EditorAction>(),
        context in any::<ActionContext>(),
    ) {
        let validator = ActionValidator::new();
        let result = validator.validate(&action, &context);
        
        // 属性1：验证结果必须一致
        let result2 = validator.validate(&action, &context);
        prop_assert_eq!(result.is_valid, result2.is_valid);
        
        // 属性2：有效动作必须有正确的影响评估
        if result.is_valid {
            prop_assert!(result.estimated_impact != ImpactLevel::Critical);
        }
    }
    
    #[test]
    fn test_transaction_building_properties(
        action in any::<EditorAction>(),
        context in any::<ActionContext>(),
    ) {
        let builder = TransactionBuilder::new();
        
        match builder.build_transaction(action.clone(), &context) {
            Ok(transaction) => {
                // 属性1：事务必须包含所有必要操作
                prop_assert!(!transaction.operations.is_empty());
                
                // 属性2：前后状态必须一致
                prop_assert_eq!(
                    transaction.before_state.version + 1,
                    transaction.after_state.version
                );
            }
            Err(_) => {
                // 属性3：构建失败时，动作应该无效
                let validator = ActionValidator::new();
                let result = validator.validate(&action, &context);
                prop_assert!(!result.is_valid);
            }
        }
    }
}
```

### 性能测试
```rust
#[bench]
fn bench_action_processing_single(b: &mut Bencher) {
    let action = EditorAction::InsertText { text: "test".to_string() };
    let context = create_test_context();
    let processor = ActionProcessor::new();
    
    b.iter(|| {
        processor.process_action(action.clone(), &context);
    });
}

#[bench]
fn bench_action_processing_batch(b: &mut Bencher) {
    let actions = create_test_actions(1000);
    let context = create_test_context();
    let processor = ActionProcessor::new();
    
    b.iter(|| {
        processor.batch_process(actions.clone(), &context);
    });
}

#[bench]
fn bench_action_merging(b: &mut Bencher) {
    let actions = create_consecutive_inserts(100);
    let merger = ActionMerger::new();
    
    b.iter(|| {
        merger.merge(&actions);
    });
}
```

## 🔄 维护指南

### 代码组织原则
1. **按功能模块化**：验证、构建、合并等独立模块
2. **纯函数设计**：尽可能使用不可变数据和纯函数
3. **错误处理**：详细的错误类型和恢复策略
4. **性能监控**：关键路径的性能指标

### 监控和日志
```rust
// 性能监控
struct PerformanceMonitor {
    validation_times: Vec<Duration>,
    building_times: Vec<Duration>,
    merging_times: Vec<Duration>,
}

impl PerformanceMonitor {
    fn record_validation(&mut self, duration: Duration) {
        self.validation_times.push(duration);
        
        if duration > Duration::from_millis(10) {
            log::warn!("验证耗时过高: {:?}", duration);
        }
    }
    
    fn generate_report(&self) -> PerformanceReport {
        PerformanceReport {
            avg_validation_time: average(&self.validation_times),
            avg_building_time: average(&self.building_times),
            avg_merging_time: average(&self.merging_times),
            p95_validation_time: percentile(&self.validation_times, 95),
            p95_building_time: percentile(&self.building_times, 95),
        }
    }
}

// 调试日志
log::debug!("处理动作: {:?}", action);
log::trace!("动作上下文: {:?}", context);
log::info!("生成事务: {} 个操作", transaction.operations.len());
```

---

*本文档是Action System的实现指南，实施时可进行优化但不违反架构约束。*
```

---

由于篇幅限制，我先提供核心的设计文档。完整的实现代码将包括：

1. **完整的EditorAction枚举**（100+个动作类型）
2. **ActionValidator实现**（所有动作的验证逻辑）
3. **TransactionBuilder实现**（动作到事务的转换）
4. **ActionMerger实现**（智能合并策略）
5. **BatchProcessor实现**（并行批量处理）
6. **完整的测试套件**

请告诉我您希望我继续提供哪个部分的完整代码实现。