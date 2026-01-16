// Render System (Slint适配层) - 渲染系统
//
// 职责：将 Layout 数据转换为 Slint 绘制命令，
//       管理渲染任务队列与资源

pub mod renderer;
pub mod slint_adapter;

pub use renderer::Renderer;
