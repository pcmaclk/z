// Config System - 配置系统
//
// 职责：管理应用配置的纯数据，
//       不参与编辑逻辑

pub mod config;
pub mod persistence;

pub use config::Config;
