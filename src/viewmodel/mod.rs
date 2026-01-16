// ViewModel Pipeline - 视图模型管道
//
// 职责：将 EditorStateSnapshot 与语法高亮、搜索结果融合，
//       生成渲染友好的 ViewModel

pub mod viewmodel;
pub mod decorator;

pub use viewmodel::ViewModel;
