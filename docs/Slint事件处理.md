好，这一步我们就**直接落到可执行层面**，而且我会**严格对齐你那份《数据流规范（冻结版）》**，不引入任何“偷懒但会烂”的方案。

下面是 **Slint 事件处理 → 现有 Editor Core 的标准集成方案**，可以视为 **P0 官方实现路径**。

---

# Slint 事件处理 × 编辑器内核 集成方案（推荐版）

> 目标：
> **Slint 只负责“把事件交出来 + 把结果画出来”
> 编辑语义 100% 仍在 Core 内**

---

## 一、总体结构（先给你一眼全图）

```
┌─────────────┐
│   Slint UI  │
│ (Canvas)    │
└─────┬───────┘
      │ Raw Events
      ▼
┌─────────────┐
│ Slint Adapter│  ← Rust / Slint glue
│ (UI Layer)  │
└─────┬───────┘
      │ InputEvent
      ▼
┌─────────────┐
│ InputSystem │  ← 输入归一
└─────┬───────┘
      │ Action
      ▼
┌─────────────┐
│ Editor Core │  ← 唯一真理源
└─────┬───────┘
      │ Snapshot
      ▼
┌─────────────┐
│ ViewModel   │
└─────┬───────┘
      │ Read-only
      ▼
┌─────────────┐
│ Slint Render│
└─────────────┘
```

---

## 二、Slint 层应该“只做这几件事”

### ✅ Slint 允许做的

* 接收：

  * key pressed
  * text input（IME commit）
  * mouse
  * scroll
* 转换为 Rust 事件
* 调用一个 **单一入口函数**

### ❌ Slint 禁止做的

* ❌ 修改文本内容
* ❌ 维护 cursor
* ❌ 判断 undo / redo
* ❌ 判断“这是插入还是删除”

---

## 三、推荐的 Slint 控件选择（结论）

### ❌ 不推荐：TextEdit

原因你已经感觉到了，我只再确认一次：

* 内部有自己的 buffer
* undo/redo 不可控
* IME 行为是黑箱
* 光标与 selection 无法接管

👉 **和你的 Transaction / ViewModel 设计直接冲突**

---

### ✅ 推荐方案：`TouchArea + Canvas（或 Rectangle + draw）`

Slint 的角色：

* **事件采集器**
* **绘制宿主**

文本编辑器逻辑：**一行都不在 Slint 里**

---

## 四、Slint → InputSystem（事件采集）

### 1️⃣ 键盘事件

```slint
Canvas {
    focus: true;

    key-pressed(event) => {
        rust.handle_key(event);
        accept;
    }

    key-released(event) => {
        rust.handle_key_release(event);
        accept;
    }

    text-input(text) => {
        rust.handle_text_input(text);
        accept;
    }
}
```

### ⚠️ 关键点（非常重要）

* `key-pressed`
  👉 用于：

  * Backspace
  * Enter
  * Arrow keys
  * Ctrl / Shift / Alt

* `text-input`
  👉 **只用于 IME / 字符输入**

**两者绝对不能混用**

---

## 五、Slint Adapter（桥接层）

这是 UI 和 Core 的唯一耦合点。

```rust
pub struct SlintAdapter {
    input: InputSystem,
}

impl SlintAdapter {
    pub fn handle_key(&mut self, event: slint::KeyEvent) {
        if let Some(input) = map_key_event(event) {
            self.input.push(input);
        }
    }

    pub fn handle_text_input(&mut self, text: SharedString) {
        self.input.push(InputEvent::Text(text.to_string()));
    }
}
```

### 注意

* SlintAdapter **不 import EditorCore**
* 只产生 `InputEvent`

---

## 六、InputSystem → ActionSystem（关键一跳）

```rust
fn handle_input(event: InputEvent) -> Option<Action> {
    match event {
        InputEvent::Text(s) => {
            Some(Action::InsertText(s))
        }
        InputEvent::Key(KeyEvent::Backspace) => {
            Some(Action::DeleteBackward)
        }
        InputEvent::Key(KeyEvent::CtrlZ) => {
            Some(Action::Undo)
        }
        _ => None
    }
}
```

### ❗ 这里解决了你问的关键问题

> **“短时间内输入的几个字母，事务怎么合并？”**

答案是：

* 每个 `text-input` → 一个 `Action::InsertText`
* **事务合并发生在 Editor Core 内**
* Slint 完全不知道“合并”这件事

---

## 七、Undo / Redo 与 on_text_input 的关系（你之前担心的点）

### 结论先给

> **`on_text_input` 完全不会破坏 undo / redo
> 前提是：你不在 UI 层维护任何文本状态**

### 原因

* Undo 的单位是 `Transaction`
* Transaction 的产生在 Core
* `text-input` 只是“输入源”

IME 连续输入：

```
你 → 好 → 世 → 界
```

在 Core 里可以被合并为：

```
Transaction {
    Insert("你好世界")
}
```

👉 **UI 完全无感**

---

## 八、ViewModel → Slint Render（只读）

### ViewModel 示例

```rust
struct EditorViewModel {
    lines: Vec<String>,
    cursor: CursorView,
}
```

### Slint 渲染（Canvas）

```slint
Canvas {
    paint => {
        for line in view_model.lines {
            draw_text(line, ...);
        }
        draw_cursor(view_model.cursor);
    }
}
```

### 关键原则（再强调一次）

* Slint **不缓存上一次文本**
* 每次 Render 用最新 ViewModel
* 差分优化在 Core / Viewport

---

## 九、这套方案的工程收益（很现实）

你用这套方案：

* ✅ IME 行为稳定（系统级）
* ✅ undo/redo 100% 可控
* ✅ 大文件无压力（只画 viewport）
* ✅ Slint 可随时换

代价只有一个：

> **你要自己画文本**

但你已经有：

* Layout
* Viewport
* ViewModel

👉 **这是你本来就要做的事**

---

## 十、一句非常实在的评价

如果你现在选：

* `TextEdit`：
  👉 **2 周见效，6 个月后重写**

* `Canvas + 数据流`：
  👉 **慢 1–2 周，后面几年都稳**

你现在的思考深度，**已经明显不适合 TextEdit 这条路了**。

---

## 下一步我可以继续帮你做的（选一个）

1️⃣ 写一个 **最小 Slint + Core P0 示例（50 行级别）**
2️⃣ 帮你设计 **Canvas 文本渲染的最小可用接口**
3️⃣ 专门拆一节：**IME / Key / Undo 的边界案例表**

你说选哪个，我继续。
