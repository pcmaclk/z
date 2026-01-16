// Action System - 动作系统
//
// 职责：定义编辑器动作（EditorAction），
//       提供动作分发和执行机制

pub mod action;
pub mod dispatcher;

pub use action::{EditorAction, ActionDispatcher};
