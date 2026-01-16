// 内存映射缓冲区
//
// 职责：为大文件提供内存映射支持，避免一次性加载全部内容

use std::ops::Range;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use memmap2::Mmap;

/// 内存映射缓冲区（大文件支持）
#[derive(Debug, Clone)]
pub struct MmapBuffer {
    #[cfg(not(target_arch = "wasm32"))]
    mmap: Option<Arc<Mmap>>,

    #[cfg(target_arch = "wasm32")]
    data: Option<Arc<Vec<u8>>>,

    length: usize,
}

impl MmapBuffer {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        use std::fs::File;

        let file = File::open(path)
            .map_err(|e| format!("无法打开文件: {}", e))?;

        let metadata = file.metadata()
            .map_err(|e| format!("无法获取文件信息: {}", e))?;

        let mmap = unsafe {
            Mmap::map(&file)
                .map_err(|e| format!("内存映射失败: {}", e))?
        };

        Ok(Self {
            mmap: Some(Arc::new(mmap)),
            length: metadata.len() as usize,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn empty() -> Self {
        Self {
            mmap: None,
            length: 0,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_file(_path: &std::path::Path) -> Result<Self, String> {
        Err("WebAssembly环境不支持文件内存映射".to_string())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn empty() -> Self {
        Self {
            data: None,
            length: 0,
        }
    }

    /// 获取缓冲区长度（字节）
    pub fn len(&self) -> usize {
        self.length
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// 获取字节切片
    pub fn get_bytes(&self, range: Range<usize>) -> &[u8] {
        let start = range.start.min(self.length);
        let end = range.end.min(self.length);

        if start >= end {
            return &[];
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(ref mmap) = self.mmap {
                &mmap[start..end]
            } else {
                &[]
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(ref data) = self.data {
                &data[start..end]
            } else {
                &[]
            }
        }
    }

    /// 尝试获取文本（UTF-8验证）
    pub fn get_text(&self, range: Range<usize>) -> Result<&str, std::str::Utf8Error> {
        let bytes = self.get_bytes(range);
        std::str::from_utf8(bytes)
    }

    /// 获取文本（UTF-8损失转换）
    pub fn get_text_lossy(&self, range: Range<usize>) -> String {
        let bytes = self.get_bytes(range);
        String::from_utf8_lossy(bytes).into_owned()
    }
}

