好的，按照统一格式，提供IO System的完整文档体系和实现代码：

# **IO System 文档体系**

```
docs/
├── 架构层/
│   └── io-system-design.md      # 设计理念和架构位置
├── 实现层/
│   └── io-system-implementation.md  # 实现细节和优化
├── API层/
│   └── io-system-api.md         # API参考和使用示例
└── 优化层/
    └── io-system-optimizations.md  # 性能优化记录
```

---

## 1. **架构层文档**：IO System设计理念

```markdown
# IO System 架构设计文档

## 📋 文档信息
- **版本**：1.0
- **状态**：已冻结
- **关联文档**：[架构宪法] · [PieceTable文档] · [Editor Core文档] · [数据流规范]

## 🎯 设计目标

### 核心定位
IO System是zedit编辑器的**文件访问枢纽**，负责：
1. **文件抽象**：提供统一、高效的文件访问接口
2. **内存管理**：支持内存映射、分段读取等大文件优化
3. **编码处理**：自动检测和转换文本编码
4. **性能优化**：零拷贝读取、异步IO、缓存策略
5. **错误处理**：健壮的文件操作错误恢复

### 设计哲学
1. **性能优先**：大文件（100MB+）访问必须高效
2. **内存友好**：最小化内存占用，支持虚拟内存
3. **编码透明**：自动处理编码，用户无需关心
4. **平台兼容**：跨平台文件访问抽象
5. **安全第一**：防止文件损坏，支持事务性写入

## 🏗️ 架构位置

### 在系统中的作用
```
┌─────────────────┐   FileRequest   ┌─────────────────┐
│   Editor Core   │ ───────────────▶ │     IO System   │ ← 本文档对象
│   (PieceTable)  │                  ├─────────────────┤
│                 │ ◀────────────────│  文件访问枢纽   │
│                 │   FileContent    │                 │
└─────────────────┘                  └─────────────────┘
                                            │
                                            ▼
┌─────────────────┐                  ┌─────────────────┐
│   Operating     │                  │   File System   │
│     System      │                  │    (物理存储)    │
└─────────────────┘                  └─────────────────┘
```

### 数据流角色
- **输入**：接收`FileRequest`（打开、保存、读取等）
- **输出**：提供`FileContent`（原始字节或解码文本）
- **内部**：编码检测、内存映射、缓存管理、错误处理
- **特点**：**无状态服务层**，所有操作幂等

## 📊 核心设计决策

### 已冻结决策
1. **内存映射优先**：大文件使用mmap，小文件使用缓冲读取
2. **编码自动检测**：支持UTF-8/16/32、GBK等常见编码
3. **滑动窗口映射**：超大文件使用窗口映射减少内存
4. **零拷贝读取**：直接返回内存映射区域，避免复制
5. **事务性写入**：先写入临时文件，成功后原子替换

### 与其他组件的关系
| 组件 | 与IO System的关系 | 通信方式 |
|------|-----------------|----------|
| PieceTable | 数据提供者 | 原始字节流 / 内存映射区域 |
| Editor Core | 主要客户端 | FileRequest / FileContent |
| Search System | 文件搜索支持 | 分段读取接口 |
| Config System | 配置文件读写 | ConfigFile接口 |
| Application Framework | 错误处理和监控 | ErrorEvent / PerformanceMetric |

## 🔧 设计约束

### 必须遵守的约束
1. **大文件友好**：必须支持1GB+文件的高效访问
2. **内存安全**：文件大小与内存占用解耦
3. **编码正确性**：必须正确处理BOM和编码检测
4. **线程安全**：支持多线程并发读取
5. **平台兼容**：Windows/macOS/Linux行为一致

### 性能目标
| 操作 | 目标时间 | 备注 |
|------|---------|------|
| 打开10MB文件 | <50ms | 内存映射 |
| 打开100MB文件 | <200ms | 延迟映射 |
| 打开1GB文件 | <1s | 滑动窗口 |
| 编码检测 | <10ms | 启发式检测 |
| 分段读取 | <5ms | 零拷贝 |
| 保存修改 | <100ms | 增量写入 |

## 📈 演进原则

### 允许的演进
1. **新编码支持**：添加更多文本编码
2. **压缩支持**：直接读取压缩文件
3. **网络支持**：HTTP/FTP等协议支持
4. **缓存优化**：更智能的预读策略
5. **监控增强**：更详细的IO性能统计

### 禁止的演进
1. **业务逻辑**：不包含任何编辑器逻辑
2. **状态存储**：不缓存文件内容（除必要缓存）
3. **同步阻塞**：不阻塞主线程的IO操作
4. **平台耦合**：不引入平台特定的文件语义

## 🔗 核心接口定义

### 必须实现的接口
```rust
// IO系统核心接口
trait IOSystem {
    /// 打开文件（内存映射或缓冲）
    fn open_file(&self, path: &Path, options: OpenOptions) -> Result<FileHandle, IOError>;
    
    /// 读取文件内容
    fn read_file(&self, handle: &FileHandle, range: Option<ByteRange>) -> Result<FileContent, IOError>;
    
    /// 写入文件
    fn write_file(&self, path: &Path, content: &[u8], options: WriteOptions) -> Result<(), IOError>;
    
    /// 关闭文件
    fn close_file(&self, handle: FileHandle) -> Result<(), IOError>;
    
    /// 检测文件编码
    fn detect_encoding(&self, data: &[u8]) -> Result<TextEncoding, IOError>;
    
    /// 转换文本编码
    fn convert_encoding(&self, data: &[u8], from: TextEncoding, to: TextEncoding) -> Result<Vec<u8>, IOError>;
    
    /// 获取文件信息
    fn get_file_info(&self, path: &Path) -> Result<FileInfo, IOError>;
}

// 文件内容抽象
enum FileContent {
    /// 内存映射区域（零拷贝）
    MemoryMapped(MmapRegion),
    
    /// 缓冲数据（小文件）
    Buffered(Vec<u8>),
    
    /// 已解码文本
    DecodedText(String),
}

// 文件句柄
struct FileHandle {
    id: FileId,
    path: PathBuf,
    size: u64,
    encoding: TextEncoding,
    mapping_strategy: MappingStrategy,
    access_mode: AccessMode,
    last_modified: SystemTime,
}
```

### 禁止的接口
```rust
// 禁止直接暴露底层文件描述符
fn get_raw_fd(&self) -> RawFd  // ❌

// 禁止全局文件缓存
static mut FILE_CACHE: HashMap<PathBuf, Vec<u8>>  // ❌

// 禁止阻塞式IO（除非明确指定）
fn read_file_blocking(&self, path: &Path) -> Vec<u8>  // ❌
```

---

*本文档定义了IO System的架构角色和设计约束，所有实现必须遵守。*
```

---

## 2. **实现层文档**：IO System实现细节

```markdown
# IO System 实现规范文档

## 📋 文档信息
- **版本**：1.0
- **状态**：实施指南（可优化）
- **关联代码**：`src/io/` 目录下所有文件

## 🏗️ 核心数据结构

### 1. 文件系统抽象层
```rust
/// IO系统核心实现
pub struct IOSystemImpl {
    /// 内存映射管理器
    mmap_manager: MmapManager,
    
    /// 编码检测器
    encoding_detector: EncodingDetector,
    
    /// 文件缓存（小文件）
    file_cache: FileCache,
    
    /// 异步IO执行器
    async_executor: AsyncExecutor,
    
    /// 性能监控
    performance_monitor: IOPerformanceMonitor,
    
    /// 平台适配器
    platform_adapter: PlatformAdapter,
    
    /// 错误处理器
    error_handler: IOErrorHandler,
}

/// 内存映射管理器
struct MmapManager {
    /// 活动映射表
    active_mappings: HashMap<FileId, MmapEntry>,
    
    /// 映射策略
    mapping_strategies: HashMap<MappingStrategy, Box<dyn MappingStrategyImpl>>,
    
    /// 内存统计
    memory_stats: MemoryStatistics,
    
    /// 清理线程
    cleanup_thread: Option<CleanupThread>,
}

/// 内存映射条目
struct MmapEntry {
    /// 映射区域
    region: MmapRegion,
    
    /// 引用计数
    ref_count: AtomicUsize,
    
    /// 最后访问时间
    last_access: Instant,
    
    /// 映射策略
    strategy: MappingStrategy,
    
    /// 文件信息
    file_info: FileInfo,
}

/// 内存映射区域（平台抽象）
enum MmapRegion {
    #[cfg(unix)]
    Unix(memmap2::Mmap),
    
    #[cfg(windows)]
    Windows(winapi_mmap::MemoryMappedFile),
    
    /// 模拟映射（用于测试）
    #[cfg(test)]
    Mock(Vec<u8>),
}

/// 映射策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MappingStrategy {
    /// 完整映射（适合小文件）
    FullMapping,
    
    /// 延迟映射（按需映射页面）
    LazyMapping,
    
    /// 滑动窗口（超大文件）
    SlidingWindow { window_size: usize },
    
    /// 缓冲读取（不适合映射）
    Buffered,
}
```

### 2. 编码系统
```rust
/// 编码检测器
pub struct EncodingDetector {
    /// 检测器集合
    detectors: Vec<Box<dyn EncodingDetectorTrait>>,
    
    /// BOM检测器
    bom_detector: BomDetector,
    
    /// 统计检测器
    statistical_detector: StatisticalDetector,
    
    /// 启发式规则
    heuristic_rules: Vec<HeuristicRule>,
    
    /// 配置
    config: EncodingConfig,
}

/// 文本编码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextEncoding {
    // Unicode家族
    Utf8,
    Utf8WithBom,
    Utf16Le,
    Utf16LeWithBom,
    Utf16Be,
    Utf16BeWithBom,
    Utf32Le,
    Utf32Be,
    
    // 单字节编码
    Ascii,
    Latin1,
    
    // 中文编码
    Gbk,
    Gb18030,
    Big5,
    
    // 日文编码
    ShiftJis,
    EucJp,
    
    // 韩文编码
    EucKr,
    
    // 其他
    Windows1252,
    Iso8859_1,
    
    // 未知编码（按二进制处理）
    Binary,
}

/// BOM（字节顺序标记）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrderMark {
    Utf8 = 0xEFBBBF,      // EF BB BF
    Utf16Le = 0xFFFE,     // FF FE
    Utf16Be = 0xFEFF,     // FE FF
    Utf32Le = 0xFFFE0000, // FF FE 00 00
    Utf32Be = 0x0000FEFF, // 00 00 FE FF
}

/// 编码检测结果
#[derive(Debug, Clone)]
pub struct EncodingDetection {
    /// 检测到的编码
    pub encoding: TextEncoding,
    
    /// 置信度 (0.0 ~ 1.0)
    pub confidence: f32,
    
    /// BOM长度（如果有）
    pub bom_length: usize,
    
    /// 检测方法
    pub method: DetectionMethod,
    
    /// 备选编码
    pub alternatives: Vec<(TextEncoding, f32)>,
}

/// 检测方法
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetectionMethod {
    Bom,           // BOM检测
    Statistical,   // 统计分析
    Heuristic,     // 启发式规则
    UserSpecified, // 用户指定
    Fallback,      // 回退方案
}
```

### 3. 文件操作接口
```rust
/// 文件打开选项
#[derive(Debug, Clone, Copy)]
pub struct OpenOptions {
    /// 访问模式
    pub access_mode: AccessMode,
    
    /// 映射策略
    pub mapping_strategy: Option<MappingStrategy>,
    
    /// 编码覆盖（如果指定，跳过自动检测）
    pub encoding_override: Option<TextEncoding>,
    
    /// 缓存策略
    pub cache_policy: CachePolicy,
    
    /// 共享模式
    pub share_mode: ShareMode,
    
    /// 性能优化选项
    pub performance_hints: PerformanceHints,
}

/// 访问模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccessMode {
    ReadOnly,      // 只读
    ReadWrite,     // 读写
    WriteOnly,     // 只写（新建文件）
    Append,        // 追加
}

/// 缓存策略
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CachePolicy {
    Default,       // 系统默认
    NoCache,       // 不缓存
    Aggressive,    // 积极缓存
    MemoryMapped,  // 内存映射
}

/// 共享模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShareMode {
    Exclusive,     // 独占访问
    ReadShare,     // 共享读取
    WriteShare,    // 共享写入
}

/// 文件内容
pub struct FileContent {
    /// 数据表示
    data: FileData,
    
    /// 编码信息
    encoding: TextEncoding,
    
    /// 大小信息
    size: usize,
    
    /// 元数据
    metadata: FileMetadata,
}

/// 文件数据表示
enum FileData {
    /// 内存映射（零拷贝）
    MemoryMapped {
        region: Arc<MmapRegion>,
        offset: usize,
        length: usize,
    },
    
    /// 堆分配数据
    HeapAllocated(Vec<u8>),
    
    /// 引用计数数据
    Shared(Arc<[u8]>),
    
    /// 空数据
    Empty,
}

/// 文件信息
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// 文件路径
    pub path: PathBuf,
    
    /// 文件大小（字节）
    pub size: u64,
    
    /// 创建时间
    pub created: Option<SystemTime>,
    
    /// 修改时间
    pub modified: Option<SystemTime>,
    
    /// 访问时间
    pub accessed: Option<SystemTime>,
    
    /// 文件属性
    pub attributes: FileAttributes,
    
    /// 文件类型
    pub file_type: FileType,
}

/// 文件类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileType {
    RegularFile,
    Directory,
    SymbolicLink,
    CharacterDevice,
    BlockDevice,
    Fifo,
    Socket,
    Unknown,
}
```

### 4. 错误处理
```rust
/// IO错误类型
#[derive(Debug, Error)]
pub enum IOError {
    #[error("文件未找到: {path}")]
    FileNotFound { path: PathBuf },
    
    #[error("权限不足: {path}")]
    PermissionDenied { path: PathBuf },
    
    #[error("文件已存在: {path}")]
    FileAlreadyExists { path: PathBuf },
    
    #[error("磁盘空间不足")]
    DiskFull,
    
    #[error("IO错误: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
    
    #[error("编码检测失败: {data_len}字节数据")]
    EncodingDetectionFailed { data_len: usize },
    
    #[error("编码转换失败: {from:?} -> {to:?}")]
    EncodingConversionFailed {
        from: TextEncoding,
        to: TextEncoding,
    },
    
    #[error("内存映射失败: {path}, 大小: {size}")]
    MemoryMapFailed {
        path: PathBuf,
        size: u64,
        source: Option<Box<dyn std::error::Error>>,
    },
    
    #[error("文件太大: {path}, 大小: {size} (最大支持: {max})")]
    FileTooLarge {
        path: PathBuf,
        size: u64,
        max: u64,
    },
    
    #[error("不支持的编码: {encoding:?}")]
    UnsupportedEncoding { encoding: TextEncoding },
    
    #[error("异步操作超时")]
    AsyncTimeout,
    
    #[error("文件已被其他进程锁定: {path}")]
    FileLocked { path: PathBuf },
}

/// 错误恢复策略
struct IOErrorHandler {
    /// 错误队列
    error_queue: VecDeque<IOError>,
    
    /// 恢复策略
    recovery_strategies: HashMap<IOErrorType, RecoveryStrategy>,
    
    /// 重试配置
    retry_config: RetryConfig,
    
    /// 错误统计
    error_stats: ErrorStatistics,
}

/// 恢复策略
enum RecoveryStrategy {
    /// 立即重试
    RetryImmediately { max_attempts: u32 },
    
    /// 用户干预（显示对话框）
    UserIntervention { message: String, options: Vec<String> },
    
    /// 降级处理（如使用缓冲代替内存映射）
    Degrade { fallback_strategy: MappingStrategy },
    
    /// 跳过并继续
    SkipAndContinue,
    
    /// 致命错误，必须停止
    Fatal,
}
```

## ⚙️ 核心算法实现

### 1. 内存映射算法
```rust
impl MmapManager {
    /// 创建内存映射
    fn create_mapping(
        &mut self,
        path: &Path,
        strategy: MappingStrategy,
        access_mode: AccessMode,
    ) -> Result<MmapEntry, IOError> {
        // 1. 获取文件信息
        let file_info = self.get_file_info(path)?;
        
        // 2. 检查文件大小限制
        if file_info.size > self.config.max_mapped_file_size {
            return Err(IOError::FileTooLarge {
                path: path.to_path_buf(),
                size: file_info.size,
                max: self.config.max_mapped_file_size,
            });
        }
        
        // 3. 选择映射策略实现
        let strategy_impl = self.get_strategy_impl(strategy);
        
        // 4. 执行映射
        let region = strategy_impl.map_file(path, access_mode, &file_info)?;
        
        // 5. 创建映射条目
        let entry = MmapEntry {
            region,
            ref_count: AtomicUsize::new(1),
            last_access: Instant::now(),
            strategy,
            file_info,
        };
        
        // 6. 添加到活动映射表
        let file_id = FileId::new();
        self.active_mappings.insert(file_id, entry);
        
        // 7. 更新内存统计
        self.update_memory_stats();
        
        Ok(entry)
    }
    
    /// 获取映射策略实现
    fn get_strategy_impl(&self, strategy: MappingStrategy) -> &dyn MappingStrategyImpl {
        self.mapping_strategies
            .get(&strategy)
            .map(|b| &**b)
            .unwrap_or_else(|| &self.default_strategy_impl)
    }
}

/// 完整映射策略
struct FullMappingStrategy;

impl MappingStrategyImpl for FullMappingStrategy {
    fn map_file(
        &self,
        path: &Path,
        access_mode: AccessMode,
        file_info: &FileInfo,
    ) -> Result<MmapRegion, IOError> {
        #[cfg(unix)]
        {
            use std::fs::OpenOptions;
            
            // 打开文件
            let file = OpenOptions::new()
                .read(true)
                .write(access_mode == AccessMode::ReadWrite)
                .open(path)?;
            
            // 创建内存映射
            let mmap = unsafe {
                memmap2::MmapOptions::new()
                    .len(file_info.size as usize)
                    .map(&file)?
            };
            
            Ok(MmapRegion::Unix(mmap))
        }
        
        #[cfg(windows)]
        {
            // Windows内存映射实现
            let mapping = winapi_mmap::MemoryMappedFile::open(
                path,
                winapi_mmap::AccessMode::ReadOnly, // 根据access_mode调整
            )?;
            
            Ok(MmapRegion::Windows(mapping))
        }
        
        #[cfg(not(any(unix, windows)))]
        {
            // 其他平台的模拟实现
            let data = std::fs::read(path)?;
            Ok(MmapRegion::Mock(data))
        }
    }
}

/// 滑动窗口映射策略
struct SlidingWindowMappingStrategy {
    window_size: usize,
    prefetch_size: usize,
}

impl SlidingWindowMappingStrategy {
    /// 映射文件窗口
    fn map_window(
        &self,
        path: &Path,
        offset: usize,
        length: usize,
    ) -> Result<MmapRegion, IOError> {
        let actual_length = length.min(self.window_size);
        
        #[cfg(unix)]
        {
            use std::fs::OpenOptions;
            
            let file = OpenOptions::new().read(true).open(path)?;
            
            // 映射指定范围
            let mmap = unsafe {
                memmap2::MmapOptions::new()
                    .offset(offset as u64)
                    .len(actual_length)
                    .map(&file)?
            };
            
            Ok(MmapRegion::Unix(mmap))
        }
        
        #[cfg(not(unix))]
        {
            // 非Unix平台使用缓冲读取
            let mut file = std::fs::File::open(path)?;
            let mut buffer = vec![0; actual_length];
            
            file.seek(std::io::SeekFrom::Start(offset as u64))?;
            file.read_exact(&mut buffer)?;
            
            #[cfg(test)]
            return Ok(MmapRegion::Mock(buffer));
            
            #[cfg(not(test))]
            return Ok(MmapRegion::HeapAllocated(buffer));
        }
    }
}
```

### 2. 编码检测算法
```rust
impl EncodingDetector {
    /// 检测数据编码
    fn detect_encoding(&self, data: &[u8]) -> EncodingDetection {
        let mut detections = Vec::new();
        
        // 1. BOM检测（最高优先级）
        if let Some(bom_detection) = self.bom_detector.detect(data) {
            detections.push(bom_detection);
        }
        
        // 2. 统计检测
        let statistical_detection = self.statistical_detector.detect(data);
        detections.push(statistical_detection);
        
        // 3. 应用启发式规则
        for rule in &self.heuristic_rules {
            if let Some(heuristic_detection) = rule.apply(data) {
                detections.push(heuristic_detection);
            }
        }
        
        // 4. 合并检测结果
        let merged = self.merge_detections(detections);
        
        // 5. 应用置信度阈值
        if merged.confidence >= self.config.min_confidence {
            merged
        } else {
            // 置信度过低，使用回退方案
            self.fallback_detection(data)
        }
    }
    
    /// BOM检测器
    fn bom_detector_detect(&self, data: &[u8]) -> Option<EncodingDetection> {
        if data.len() >= 3 && data[0..3] == [0xEF, 0xBB, 0xBF] {
            // UTF-8 BOM
            Some(EncodingDetection {
                encoding: TextEncoding::Utf8WithBom,
                confidence: 1.0,
                bom_length: 3,
                method: DetectionMethod::Bom,
                alternatives: Vec::new(),
            })
        } else if data.len() >= 2 {
            match (data[0], data[1]) {
                (0xFF, 0xFE) if data.len() >= 4 && data[2] == 0x00 && data[3] == 0x00 => {
                    // UTF-32 LE BOM
                    Some(EncodingDetection {
                        encoding: TextEncoding::Utf32Le,
                        confidence: 1.0,
                        bom_length: 4,
                        method: DetectionMethod::Bom,
                        alternatives: Vec::new(),
                    })
                }
                (0xFF, 0xFE) => {
                    // UTF-16 LE BOM
                    Some(EncodingDetection {
                        encoding: TextEncoding::Utf16Le,
                        confidence: 1.0,
                        bom_length: 2,
                        method: DetectionMethod::Bom,
                        alternatives: Vec::new(),
                    })
                }
                (0xFE, 0xFF) => {
                    // UTF-16 BE BOM
                    Some(EncodingDetection {
                        encoding: TextEncoding::Utf16Be,
                        confidence: 1.0,
                        bom_length: 2,
                        method: DetectionMethod::Bom,
                        alternatives: Vec::new(),
                    })
                }
                (0x00, 0x00) if data.len() >= 4 && data[2] == 0xFE && data[3] == 0xFF => {
                    // UTF-32 BE BOM
                    Some(EncodingDetection {
                        encoding: TextEncoding::Utf32Be,
                        confidence: 1.0,
                        bom_length: 4,
                        method: DetectionMethod::Bom,
                        alternatives: Vec::new(),
                    })
                }
                _ => None,
            }
        } else {
            None
        }
    }
    
    /// 统计检测器（基于字符分布）
    fn statistical_detector_detect(&self, data: &[u8]) -> EncodingDetection {
        // 收集统计特征
        let features = self.extract_statistical_features(data);
        
        // 计算与每种编码的匹配度
        let mut scores = Vec::new();
        
        for &encoding in &self.config.supported_encodings {
            let score = self.calculate_encoding_score(&features, encoding);
            scores.push((encoding, score));
        }
        
        // 按置信度排序
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // 最佳匹配
        let (best_encoding, best_score) = scores[0];
        let confidence = best_score.min(1.0).max(0.0);
        
        EncodingDetection {
            encoding: best_encoding,
            confidence,
            bom_length: 0,
            method: DetectionMethod::Statistical,
            alternatives: scores[1..].iter().take(3).map(|&(e, s)| (e, s)).collect(),
        }
    }
    
    /// 提取统计特征
    fn extract_statistical_features(&self, data: &[u8]) -> StatisticalFeatures {
        let mut features = StatisticalFeatures::new();
        
        // 分析字节值分布
        let mut byte_histogram = [0u32; 256];
        for &byte in data {
            byte_histogram[byte as usize] += 1;
        }
        
        // 计算零字节比例（UTF-16/32特征）
        let zero_byte_ratio = byte_histogram[0] as f32 / data.len() as f32;
        
        // 检查UTF-8有效性
        let utf8_validity = self.check_utf8_validity(data);
        
        // 检查常见控制字符
        let control_char_ratio = self.calculate_control_char_ratio(data);
        
        StatisticalFeatures {
            byte_histogram,
            zero_byte_ratio,
            utf8_validity,
            control_char_ratio,
            data_length: data.len(),
            common_patterns: self.detect_common_patterns(data),
        }
    }
    
    /// 检查UTF-8有效性
    fn check_utf8_validity(&self, data: &[u8]) -> f32 {
        let mut valid_bytes = 0;
        let mut i = 0;
        
        while i < data.len() {
            let byte = data[i];
            
            // 单字节字符 (0x00-0x7F)
            if byte <= 0x7F {
                valid_bytes += 1;
                i += 1;
            }
            // 双字节字符
            else if byte >= 0xC2 && byte <= 0xDF {
                if i + 1 < data.len() && (data[i + 1] & 0xC0) == 0x80 {
                    valid_bytes += 2;
                    i += 2;
                } else {
                    break;
                }
            }
            // 三字节字符
            else if byte >= 0xE0 && byte <= 0xEF {
                if i + 2 < data.len() 
                    && (data[i + 1] & 0xC0) == 0x80 
                    && (data[i + 2] & 0xC0) == 0x80 {
                    valid_bytes += 3;
                    i += 3;
                } else {
                    break;
                }
            }
            // 四字节字符
            else if byte >= 0xF0 && byte <= 0xF4 {
                if i + 3 < data.len() 
                    && (data[i + 1] & 0xC0) == 0x80 
                    && (data[i + 2] & 0xC0) == 0x80 
                    && (data[i + 3] & 0xC0) == 0x80 {
                    valid_bytes += 4;
                    i += 4;
                } else {
                    break;
                }
            }
            // 无效的UTF-8字节
            else {
                break;
            }
        }
        
        valid_bytes as f32 / data.len() as f32
    }
}
```

### 3. 文件读取算法（零拷贝优化）
```rust
impl IOSystemImpl {
    /// 读取文件内容（支持零拷贝）
    fn read_file(
        &self,
        handle: &FileHandle,
        range: Option<ByteRange>,
    ) -> Result<FileContent, IOError> {
        match handle.mapping_strategy {
            MappingStrategy::FullMapping | MappingStrategy::LazyMapping => {
                // 使用内存映射
                self.read_memory_mapped(handle, range)
            }
            
            MappingStrategy::SlidingWindow { window_size } => {
                // 滑动窗口读取
                self.read_sliding_window(handle, range, window_size)
            }
            
            MappingStrategy::Buffered => {
                // 缓冲读取
                self.read_buffered(handle, range)
            }
        }
    }
    
    /// 内存映射读取（零拷贝）
    fn read_memory_mapped(
        &self,
        handle: &FileHandle,
        range: Option<ByteRange>,
    ) -> Result<FileContent, IOError> {
        // 1. 获取内存映射条目
        let entry = self.mmap_manager.get_entry(handle.id)?;
        
        // 2. 计算读取范围
        let (offset, length) = match range {
            Some(range) => (range.start, range.end - range.start),
            None => (0, handle.size as usize),
        };
        
        // 3. 边界检查
        if offset >= handle.size as usize {
            return Err(IOError::OutOfBounds {
                offset,
                size: handle.size as usize,
            });
        }
        
        let actual_length = length.min(handle.size as usize - offset);
        
        // 4. 创建零拷贝引用
        match &entry.region {
            MmapRegion::Unix(mmap) => {
                let slice = &mmap[offset..offset + actual_length];
                let data = FileData::MemoryMapped {
                    region: Arc::new(MmapRegion::Unix(mmap.clone())),
                    offset,
                    length: actual_length,
                };
                
                Ok(FileContent {
                    data,
                    encoding: handle.encoding,
                    size: actual_length,
                    metadata: FileMetadata::from_info(&handle.info),
                })
            }
            
            // 其他平台的实现...
            
            _ => {
                // 回退到缓冲读取
                self.read_buffered(handle, range)
            }
        }
    }
    
    /// 滑动窗口读取（大文件优化）
    fn read_sliding_window(
        &self,
        handle: &FileHandle,
        range: Option<ByteRange>,
        window_size: usize,
    ) -> Result<FileContent, IOError> {
        // 1. 计算窗口位置
        let (offset, length) = match range {
            Some(range) => (range.start, range.end - range.start),
            None => (0, handle.size as usize),
        };
        
        // 2. 如果请求范围小于窗口，直接映射整个窗口
        if length <= window_size {
            let window_offset = self.align_to_window(offset, window_size);
            let window_length = window_size.min(handle.size as usize - window_offset);
            
            // 3. 映射窗口
            let window_data = self.mmap_manager.map_window(
                &handle.path,
                window_offset,
                window_length,
            )?;
            
            // 4. 提取请求范围
            let data_offset = offset - window_offset;
            let data_slice = &window_data[data_offset..data_offset + length];
            
            Ok(FileContent {
                data: FileData::HeapAllocated(data_slice.to_vec()),
                encoding: handle.encoding,
                size: length,
                metadata: FileMetadata::from_info(&handle.info),
            })
        } else {
            // 5. 大范围读取，使用缓冲
            self.read_buffered(handle, range)
        }
    }
    
    /// 对齐到窗口边界
    fn align_to_window(&self, offset: usize, window_size: usize) -> usize {
        offset / window_size * window_size
    }
}
```

### 4. 编码转换算法
```rust
impl IOSystemImpl {
    /// 转换文本编码
    fn convert_encoding(
        &self,
        data: &[u8],
        from: TextEncoding,
        to: TextEncoding,
    ) -> Result<Vec<u8>, IOError> {
        if from == to {
            return Ok(data.to_vec());
        }
        
        match (from, to) {
            // UTF-8相关转换
            (TextEncoding::Utf8, TextEncoding::Utf8WithBom) => {
                self.add_utf8_bom(data)
            }
            (TextEncoding::Utf8WithBom, TextEncoding::Utf8) => {
                self.remove_bom(data, 3)
            }
            
            // UTF-16相关转换
            (TextEncoding::Utf16Le, TextEncoding::Utf8) => {
                self.utf16le_to_utf8(data)
            }
            (TextEncoding::Utf16Be, TextEncoding::Utf8) => {
                self.utf16be_to_utf8(data)
            }
            
            // GBK相关转换
            (TextEncoding::Gbk, TextEncoding::Utf8) => {
                self.gbk_to_utf8(data)
            }
            (TextEncoding::Utf8, TextEncoding::Gbk) => {
                self.utf8_to_gbk(data)
            }
            
            // 不支持的转换
            _ => Err(IOError::UnsupportedEncodingConversion { from, to }),
        }
    }
    
    /// UTF-16 LE 转 UTF-8
    fn utf16le_to_utf8(&self, data: &[u8]) -> Result<Vec<u8>, IOError> {
        // 确保数据长度是偶数（UTF-16是2字节对齐）
        if data.len() % 2 != 0 {
            return Err(IOError::InvalidDataLength {
                expected: "偶数".to_string(),
                actual: data.len(),
            });
        }
        
        let mut result = Vec::with_capacity(data.len());
        let mut i = 0;
        
        while i < data.len() {
            // 读取UTF-16代码点（小端序）
            let low = data[i] as u16;
            let high = data[i + 1] as u16;
            let code_point = (high << 8) | low;
            
            // 转换到UTF-8
            if code_point <= 0x7F {
                // 单字节
                result.push(code_point as u8);
            } else if code_point <= 0x7FF {
                // 双字节
                result.push(0xC0 | ((code_point >> 6) as u8));
                result.push(0x80 | ((code_point & 0x3F) as u8));
            } else if code_point >= 0xD800 && code_point <= 0xDFFF {
                // 代理对（Surrogate Pair）
                if i + 4 > data.len() {
                    return Err(IOError::InvalidUtf16Sequence);
                }
                
                // 读取高代理和低代理
                let high_surrogate = code_point;
                let low_low = data[i + 2] as u16;
                let low_high = data[i + 3] as u16;
                let low_surrogate = (low_high << 8) | low_low;
                
                // 计算Unicode标量值
                let scalar = 0x10000 
                    + ((high_surrogate as u32 - 0xD800) << 10) 
                    + (low_surrogate as u32 - 0xDC00);
                
                // 转换为UTF-8（4字节）
                result.push(0xF0 | ((scalar >> 18) as u8));
                result.push(0x80 | (((scalar >> 12) & 0x3F) as u8));
                result.push(0x80 | (((scalar >> 6) & 0x3F) as u8));
                result.push(0x80 | ((scalar & 0x3F) as u8));
                
                i += 2; // 额外跳过低代理的2个字节
            } else {
                // 三字节
                result.push(0xE0 | ((code_point >> 12) as u8));
                result.push(0x80 | (((code_point >> 6) & 0x3F) as u8));
                result.push(0x80 | ((code_point & 0x3F) as u8));
            }
            
            i += 2;
        }
        
        Ok(result)
    }
    
    /// GBK 转 UTF-8（使用编码库）
    fn gbk_to_utf8(&self, data: &[u8]) -> Result<Vec<u8>, IOError> {
        use encoding_rs::GBK;
        
        // 使用encoding_rs库进行转换
        let (result, _, had_errors) = GBK.decode(data);
        
        if had_errors {
            // 记录转换错误但不失败（尝试最大程度转换）
            self.log_conversion_errors("GBK", "UTF-8", data.len());
        }
        
        Ok(result.into_owned().into_bytes())
    }
    
    /// 添加UTF-8 BOM
    fn add_utf8_bom(&self, data: &[u8]) -> Result<Vec<u8>, IOError> {
        let mut result = Vec::with_capacity(data.len() + 3);
        result.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        result.extend_from_slice(data);
        Ok(result)
    }
}
```

## 🧩 子系统实现

### 1. 内存映射管理器模块
**位置**：`src/io/mmap_manager.rs`
**职责**：
- 管理所有内存映射
- 实现不同映射策略
- 监控内存使用
- 自动清理未使用映射

**关键设计**：
```rust
pub struct MmapManager {
    // 线程安全的活动映射表
    active_mappings: Arc<RwLock<HashMap<FileId, Arc<MmapEntry>>>>,
    
    // 配置
    config: MmapConfig,
    
    // 清理器
    cleaner: MmapCleaner,
    
    // 统计
    stats: MmapStatistics,
}

impl MmapManager {
    /// 创建或获取现有映射
    pub fn get_or_create_mapping(
        &self,
        path: &Path,
        strategy: MappingStrategy,
    ) -> Result<Arc<MmapEntry>, IOError> {
        // 1. 检查是否存在现有映射
        if let Some(entry) = self.get_existing_mapping(path) {
            // 增加引用计数
            entry.increment_ref_count();
            return Ok(entry);
        }
        
        // 2. 创建新映射
        let entry = self.create_mapping(path, strategy)?;
        let entry_arc = Arc::new(entry);
        
        // 3. 添加到活动映射表
        self.add_to_active_mappings(path, Arc::clone(&entry_arc))?;
        
        Ok(entry_arc)
    }
    
    /// 清理未使用的映射
    pub fn cleanup_unused(&self) -> CleanupResult {
        let mut result = CleanupResult::default();
        
        let mut mappings = self.active_mappings.write().unwrap();
        let mut to_remove = Vec::new();
        
        for (file_id, entry) in mappings.iter() {
            if entry.should_cleanup() {
                result.memory_freed += entry.size();
                to_remove.push(*file_id);
            }
        }
        
        // 移除待清理的映射
        for file_id in to_remove {
            mappings.remove(&file_id);
            result.entries_removed += 1;
        }
        
        result
    }
    
    /// 获取内存使用统计
    pub fn get_memory_stats(&self) -> MemoryStats {
        let mappings = self.active_mappings.read().unwrap();
        
        let mut stats = MemoryStats::default();
        for entry in mappings.values() {
            stats.total_memory += entry.size();
            stats.mapped_files += 1;
            
            match entry.strategy {
                MappingStrategy::FullMapping => stats.full_mappings += 1,
                MappingStrategy::LazyMapping => stats.lazy_mappings += 1,
                MappingStrategy::SlidingWindow { .. } => stats.window_mappings += 1,
                MappingStrategy::Buffered => stats.buffered_files += 1,
            }
        }
        
        stats
    }
}
```

### 2. 编码检测器模块
**位置**：`src/io/encoding_detector.rs`
**设计特点**：
- 多检测器投票机制
- 置信度评估
- 启发式规则
- 可扩展的检测器接口

**检测器注册**：
```rust
pub struct EncodingDetectorRegistry {
    // 按优先级排序的检测器
    detectors: Vec<Box<dyn EncodingDetectorTrait>>,
    
    // 检测器配置
    config: DetectorConfig,
    
    // 缓存最近检测结果
    detection_cache: LruCache<DetectionCacheKey, EncodingDetection>,
}

impl EncodingDetectorRegistry {
    /// 注册检测器
    pub fn register_detector(&mut self, detector: Box<dyn EncodingDetectorTrait>) {
        self.detectors.push(detector);
        // 按优先级排序
        self.detectors.sort_by_key(|d| d.priority());
    }
    
    /// 检测编码（带缓存）
    pub fn detect_with_cache(&mut self, data: &[u8]) -> EncodingDetection {
        // 检查缓存
        let cache_key = DetectionCacheKey::from_data(data);
        if let Some(cached) = self.detection_cache.get(&cache_key) {
            return cached.clone();
        }
        
        // 执行检测
        let detection = self.detect_impl(data);
        
        // 缓存结果
        self.detection_cache.put(cache_key, detection.clone());
        
        detection
    }
    
    /// 实际检测实现
    fn detect_impl(&self, data: &[u8]) -> EncodingDetection {
        let mut all_detections = Vec::new();
        
        // 运行所有检测器
        for detector in &self.detectors {
            if let Some(detection) = detector.detect(data) {
                all_detections.push(detection);
            }
        }
        
        // 合并检测结果
        self.merge_detections(all_detections)
    }
    
    /// 合并多个检测结果
    fn merge_detections(&self, detections: Vec<EncodingDetection>) -> EncodingDetection {
        if detections.is_empty() {
            return self.fallback_detection();
        }
        
        // 按检测方法加权投票
        let mut votes: HashMap<TextEncoding, f32> = HashMap::new();
        
        for detection in &detections {
            let weight = match detection.method {
                DetectionMethod::Bom => 2.0,      // BOM检测最可靠
                DetectionMethod::Statistical => 1.5,
                DetectionMethod::Heuristic => 1.0,
                DetectionMethod::UserSpecified => 3.0,
                DetectionMethod::Fallback => 0.5,
            };
            
            let vote = detection.confidence * weight;
            *votes.entry(detection.encoding).or_insert(0.0) += vote;
            
            // 同时考虑备选编码
            for (alt_encoding, alt_confidence) in &detection.alternatives {
                let alt_vote = alt_confidence * weight * 0.5; // 备选权重减半
                *votes.entry(*alt_encoding).or_insert(0.0) += alt_vote;
            }
        }
        
        // 找出最佳编码
        let (best_encoding, best_score) = votes
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap_or((TextEncoding::Utf8, 0.0));
        
        // 计算最终置信度
        let total_possible = detections.len() as f32 * 3.0; // 最大可能得分
        let confidence = (best_score / total_possible).min(1.0);
        
        EncodingDetection {
            encoding: best_encoding,
            confidence,
            bom_length: 0, // 需要从原始检测中提取
            method: DetectionMethod::Statistical, // 标记为合并结果
            alternatives: Vec::new(),
        }
    }
}
```

### 3. 异步IO执行器模块
**位置**：`src/io/async_executor.rs`
**设计特点**：
- 基于Tokio的异步IO
- 任务优先级调度
- IO并发限制
- 进度回调支持

**异步任务管理**：
```rust
pub struct AsyncIOExecutor {
    // Tokio运行时
    runtime: tokio::runtime::Runtime,
    
    // 任务管理器
    task_manager: TaskManager,
    
    // 并发限制器
    concurrency_limiter: ConcurrencyLimiter,
    
    // 进度跟踪器
    progress_tracker: ProgressTracker,
}

impl AsyncIOExecutor {
    /// 异步读取文件
    pub async fn read_file_async(
        &self,
        path: PathBuf,
        options: AsyncReadOptions,
    ) -> Result<Vec<u8>, IOError> {
        // 应用并发限制
        let permit = self.concurrency_limiter.acquire().await?;
        
        // 创建进度跟踪
        let progress_token = self.progress_tracker.start_task(
            &path,
            ProgressTaskType::FileRead,
        );
        
        // 执行异步读取
        let result = tokio::fs::read(&path).await;
        
        // 更新进度
        self.progress_tracker.complete_task(progress_token);
        
        // 释放并发许可
        drop(permit);
        
        result.map_err(|e| IOError::from(e))
    }
    
    /// 异步写入文件（事务性）
    pub async fn write_file_atomic(
        &self,
        path: PathBuf,
        data: Vec<u8>,
        options: AtomicWriteOptions,
    ) -> Result<(), IOError> {
        // 1. 写入临时文件
        let temp_path = self.create_temp_path(&path);
        
        let write_result = tokio::fs::write(&temp_path, &data).await;
        
        if write_result.is_err() {
            // 清理临时文件
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(IOError::from(write_result.err().unwrap()));
        }
        
        // 2. 验证写入的数据（可选）
        if options.verify_written {
            let verify_result = self.verify_written_data(&temp_path, &data).await;
            if verify_result.is_err() {
                let _ = tokio::fs::remove_file(&temp_path).await;
                return verify_result;
            }
        }
        
        // 3. 原子替换
        let replace_result = tokio::fs::rename(&temp_path, &path).await;
        
        if replace_result.is_err() {
            // 重命名失败，尝试复制回退
            let copy_result = self.fallback_copy(&temp_path, &path, &data).await;
            if copy_result.is_err() {
                return copy_result;
            }
        }
        
        // 4. 清理临时文件（如果还存在）
        let _ = tokio::fs::remove_file(&temp_path).await;
        
        Ok(())
    }
    
    /// 创建临时文件路径
    fn create_temp_path(&self, original_path: &Path) -> PathBuf {
        let file_name = original_path.file_name().unwrap_or_default();
        let temp_name = format!(".zedit_tmp_{}_{}", 
            file_name.to_string_lossy(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        
        original_path.with_file_name(temp_name)
    }
}
```

### 4. 文件缓存模块
**位置**：`src/io/file_cache.rs`
**设计特点**：
- LRU缓存策略
- 内存使用限制
- 文件变更检测
- 智能预读

**缓存管理**：
```rust
pub struct FileCache {
    // LRU缓存
    cache: LruCache<PathBuf, CachedFile>,
    
    // 缓存配置
    config: CacheConfig,
    
    // 内存使用跟踪
    memory_usage: AtomicUsize,
    
    // 文件监视器（检测外部修改）
    file_watcher: Option<FileWatcher>,
}

impl FileCache {
    /// 获取文件（优先从缓存）
    pub fn get_or_load(&mut self, path: &Path) -> Result<&CachedFile, IOError> {
        // 检查缓存是否存在且有效
        if let Some(cached) = self.cache.get(&path.to_path_buf()) {
            if self.is_cache_valid(cached) {
                // 更新访问时间
                self.cache.promote(&path.to_path_buf());
                return Ok(cached);
            } else {
                // 缓存失效，移除
                self.remove(path);
            }
        }
        
        // 加载文件到缓存
        self.load_to_cache(path)
    }
    
    /// 检查缓存有效性
    fn is_cache_valid(&self, cached: &CachedFile) -> bool {
        // 1. 检查文件是否被外部修改
        if let Some(current_mtime) = self.get_file_mtime(&cached.path) {
            if current_mtime != cached.metadata.modified {
                return false;
            }
        }
        
        // 2. 检查缓存是否过期
        if let Some(expiry) = cached.expiry {
            if expiry < Instant::now() {
                return false;
            }
        }
        
        // 3. 检查内存压力（如果需要释放内存）
        if self.memory_pressure_high() && cached.last_access.elapsed() > Duration::from_secs(30) {
            return false;
        }
        
        true
    }
    
    /// 智能预读
    pub fn prefetch(&mut self, path: &Path, hints: PrefetchHints) -> Result<(), IOError> {
        match hints.pattern {
            PrefetchPattern::Sequential => {
                // 顺序读取模式，预读接下来的内容
                self.prefetch_sequential(path, hints.range)
            }
            
            PrefetchPattern::RandomAccess => {
                // 随机访问模式，预读整个文件或建立索引
                self.prefetch_random(path)
            }
            
            PrefetchPattern::WorkingSet => {
                // 工作集模式，基于历史访问模式预读
                self.prefetch_working_set(path)
            }
        }
    }
    
    /// 顺序预读
    fn prefetch_sequential(&mut self, path: &Path, range: ByteRange) -> Result<(), IOError> {
        let prefetch_size = self.config.sequential_prefetch_size;
        let start = range.end;
        let end = (start + prefetch_size).min(self.get_file_size(path)?);
        
        if start < end {
            // 异步预读
            self.async_prefetch(path, start..end)?;
        }
        
        Ok(())
    }
}
```

## 🧪 测试策略

### 单元测试覆盖
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_memory_mapping_small_file() {
        // 创建测试文件
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, Memory Mapping!";
        temp_file.write_all(test_data).unwrap();
        
        // 测试内存映射
        let io_system = IOSystem::new();
        let handle = io_system.open_file(
            temp_file.path(),
            OpenOptions {
                mapping_strategy: Some(MappingStrategy::FullMapping),
                ..Default::default()
            },
        ).unwrap();
        
        // 读取数据
        let content = io_system.read_file(&handle, None).unwrap();
        
        // 验证数据
        match content.data {
            FileData::MemoryMapped { .. } => {
                // 应该是内存映射
                assert_eq!(content.size, test_data.len());
            }
            _ => panic!("Expected memory mapped data"),
        }
    }
    
    #[test]
    fn test_encoding_detection_utf8_bom() {
        let detector = EncodingDetector::new();
        
        // UTF-8 with BOM
        let data_with_bom = b"\xEF\xBB\xBFHello, UTF-8!";
        let detection = detector.detect_encoding(data_with_bom);
        
        assert_eq!(detection.encoding, TextEncoding::Utf8WithBom);
        assert_eq!(detection.bom_length, 3);
        assert!(detection.confidence > 0.9);
    }
    
    #[test]
    fn test_encoding_conversion_gbk_to_utf8() {
        let io_system = IOSystem::new();
        
        // 简体中文GBK编码（"你好"）
        let gbk_data: [u8; 4] = [0xC4, 0xE3, 0xBA, 0xC3];
        
        let utf8_data = io_system.convert_encoding(
            &gbk_data,
            TextEncoding::Gbk,
            TextEncoding::Utf8,
        ).unwrap();
        
        // UTF-8编码的"你好"应该是6字节
        assert_eq!(utf8_data.len(), 6);
    }
    
    #[test]
    fn test_sliding_window_large_file() {
        // 创建大测试文件（10MB）
        let temp_file = create_large_test_file(10 * 1024 * 1024);
        
        let io_system = IOSystem::new();
        let handle = io_system.open_file(
            temp_file.path(),
            OpenOptions {
                mapping_strategy: Some(MappingStrategy::SlidingWindow {
                    window_size: 1024 * 1024, // 1MB窗口
                }),
                ..Default::default()
            },
        ).unwrap();
        
        // 测试不同位置的读取
        let test_ranges = [
            0..1000,                    // 开头
            5_000_000..5_001_000,       // 中间
            9_999_000..10_000_000,      // 结尾附近
        ];
        
        for range in test_ranges {
            let content = io_system.read_file(&handle, Some(range.clone())).unwrap();
            assert_eq!(content.size, range.end - range.start);
        }
    }
    
    #[test]
    fn test_atomic_write_recovery() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let io_system = IOSystem::new();
        
        // 模拟写入过程中的崩溃
        std::panic::catch_unwind(|| {
            // 开始原子写入
            let write_future = io_system.write_file_atomic(
                file_path.clone(),
                b"Important data".to_vec(),
                AtomicWriteOptions::default(),
            );
            
            // 在写入过程中panic（模拟崩溃）
            panic!("Simulated crash during write");
        });
        
        // 检查临时文件应该被清理
        let temp_files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".zedit_tmp_"))
            .collect();
        
        assert!(temp_files.is_empty(), "Temporary files should be cleaned up");
        
        // 目标文件应该不存在或为空（因为写入未完成）
        if file_path.exists() {
            let file_size = std::fs::metadata(&file_path).unwrap().len();
            assert_eq!(file_size, 0, "File should be empty if write was interrupted");
        }
    }
}

#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_encoding_detection_properties(
            data in prop::collection::vec(any::<u8>(), 0..1000)
        ) {
            let detector = EncodingDetector::new();
            let detection = detector.detect_encoding(&data);
            
            // 属性1：检测结果必须在支持的编码列表中
            assert!(detector.config.supported_encodings.contains(&detection.encoding));
            
            // 属性2：置信度必须在0.0到1.0之间
            assert!(detection.confidence >= 0.0 && detection.confidence <= 1.0);
            
            // 属性3：如果有BOM，BOM长度必须正确
            if detection.bom_length > 0 {
                assert!(detection.bom_length == 2 || detection.bom_length == 3 || detection.bom_length == 4);
            }
        }
        
        #[test]
        fn test_encoding_conversion_reversible(
            text in prop::collection::vec(any::<char>(), 0..100)
        ) {
            let io_system = IOSystem::new();
            let utf8_text: String = text.into_iter().collect();
            let utf8_data = utf8_text.as_bytes();
            
            // UTF-8 -> UTF-16 LE -> UTF-8 应该可逆
            let utf16_data = io_system.convert_encoding(
                utf8_data,
                TextEncoding::Utf8,
                TextEncoding::Utf16Le,
            ).unwrap();
            
            let recovered_data = io_system.convert_encoding(
                &utf16_data,
                TextEncoding::Utf16Le,
                TextEncoding::Utf8,
            ).unwrap();
            
            // 应该能恢复原始数据（可能因为代理对等略有不同）
            let recovered_text = String::from_utf8_lossy(&recovered_data);
            assert_eq!(recovered_text, utf8_text);
        }
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::{Instant, Duration};
    
    #[bench]
    fn bench_memory_mapping_speed(b: &mut test::Bencher) {
        // 创建测试文件
        let temp_file = create_large_test_file(10 * 1024 * 1024); // 10MB
        
        let io_system = IOSystem::new();
        
        b.iter(|| {
            // 打开并映射文件
            let handle = io_system.open_file(
                temp_file.path(),
                OpenOptions {
                    mapping_strategy: Some(MappingStrategy::FullMapping),
                    ..Default::default()
                },
            ).unwrap();
            
            // 读取整个文件
            let content = io_system.read_file(&handle, None).unwrap();
            
            // 确保读取了数据
            test::black_box(content);
        });
    }
    
    #[bench]
    fn bench_encoding_detection_speed(b: &mut test::Bencher) {
        let detector = EncodingDetector::new();
        
        // 测试数据：各种编码的混合
        let test_data = include_bytes!("../test_data/mixed_encodings.bin");
        
        b.iter(|| {
            let detection = detector.detect_encoding(test_data);
            test::black_box(detection);
        });
    }
    
    #[bench]
    fn bench_sliding_window_random_access(b: &mut test::Bencher) {
        let temp_file = create_large_test_file(100 * 1024 * 1024); // 100MB
        
        let io_system = IOSystem::new();
        let handle = io_system.open_file(
            temp_file.path(),
            OpenOptions {
                mapping_strategy: Some(MappingStrategy::SlidingWindow {
                    window_size: 1024 * 1024, // 1MB窗口
                }),
                ..Default::default()
            },
        ).unwrap();
        
        let mut rng = rand::thread_rng();
        
        b.iter(|| {
            // 随机访问不同位置
            let offset = rng.gen_range(0..90 * 1024 * 1024); // 在90MB范围内
            let length = 4096; // 读取4KB
            
            let content = io_system.read_file(
                &handle,
                Some(offset..offset + length),
            ).unwrap();
            
            test::black_box(content);
        });
    }
}
```

### 集成测试
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_complete_file_workflow() {
        let temp_dir = tempfile::tempdir().unwrap();
        
        // 1. 创建IO系统
        let io_system = IOSystem::new();
        
        // 2. 创建测试文件
        let file_path = temp_dir.path().join("test.txt");
        let original_text = "Hello, 世界! 🌍\nThis is a test file.\n".repeat(100);
        
        // 3. 写入文件
        io_system.write_file(
            &file_path,
            original_text.as_bytes(),
            WriteOptions::default(),
        ).unwrap();
        
        // 4. 打开并检测编码
        let handle = io_system.open_file(&file_path, OpenOptions::default()).unwrap();
        assert_eq!(handle.encoding, TextEncoding::Utf8);
        
        // 5. 读取文件内容
        let content = io_system.read_file(&handle, None).unwrap();
        let read_text = match content.data {
            FileData::DecodedText(text) => text,
            _ => String::from_utf8_lossy(content.as_bytes()).to_string(),
        };
        
        // 6. 验证数据
        assert_eq!(read_text, original_text);
        
        // 7. 修改并保存
        let modified_text = read_text + "Modified!\n";
        io_system.write_file(
            &file_path,
            modified_text.as_bytes(),
            WriteOptions::default(),
        ).unwrap();
        
        // 8. 重新打开验证修改
        let handle2 = io_system.open_file(&file_path, OpenOptions::default()).unwrap();
        let content2 = io_system.read_file(&handle2, None).unwrap();
        let read_text2 = String::from_utf8_lossy(content2.as_bytes()).to_string();
        
        assert_eq!(read_text2, modified_text);
    }
    
    #[test]
    fn test_concurrent_file_access() {
        let temp_file = create_large_test_file(1024 * 1024); // 1MB
        
        let io_system = Arc::new(IOSystem::new());
        let path = temp_file.path().to_path_buf();
        
        // 创建多个并发读取任务
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let io_system = Arc::clone(&io_system);
                let path = path.clone();
                
                std::thread::spawn(move || {
                    // 每个线程读取不同的部分
                    let handle = io_system.open_file(&path, OpenOptions::default()).unwrap();
                    let offset = i * 1024 * 10; // 每个线程读10KB的不同部分
                    let range = offset..offset + 10240;
                    
                    let content = io_system.read_file(&handle, Some(range)).unwrap();
                    
                    (i, content.size)
                })
            })
            .collect();
        
        // 收集结果
        for handle in handles {
            let (thread_id, size) = handle.join().unwrap();
            assert_eq!(size, 10240, "Thread {} read wrong size", thread_id);
        }
    }
}
```

## 🔄 维护指南

### 配置优化建议
```rust
/// IO系统性能调优配置
pub struct IOPerformanceConfig {
    /// 内存映射阈值（小于此值使用缓冲）
    pub mmap_threshold: usize,
    
    /// 滑动窗口大小
    pub sliding_window_size: usize,
    
    /// 预读大小
    pub prefetch_size: usize,
    
    /// 最大并发IO操作数
    pub max_concurrent_io: usize,
    
    /// 文件缓存大小（MB）
    pub file_cache_size_mb: usize,
    
    /// 编码检测缓存大小
    pub encoding_cache_size: usize,
}

impl IOPerformanceConfig {
    /// 根据系统内存自动调整
    pub fn auto_adjust() -> Self {
        let total_memory = get_total_system_memory();
        
        Self {
            mmap_threshold: if total_memory < 2 * 1024 * 1024 * 1024 {
                // 内存小于2GB，降低阈值
                1 * 1024 * 1024 // 1MB
            } else {
                10 * 1024 * 1024 // 10MB
            },
            
            sliding_window_size: if total_memory < 4 * 1024 * 1024 * 1024 {
                512 * 1024 // 512KB
            } else {
                2 * 1024 * 1024 // 2MB
            },
            
            prefetch_size: 64 * 1024, // 64KB
            
            max_concurrent_io: num_cpus::get() * 2,
            
            file_cache_size_mb: (total_memory / (1024 * 1024) / 8).min(256), // 最多256MB
            
            encoding_cache_size: 1000,
        }
    }
}
```

### 监控和诊断
```rust
/// IO系统监控器
pub struct IOMonitor {
    /// 性能指标
    metrics: IOMetrics,
    
    /// 慢操作跟踪
    slow_operations: SlowOperationTracker,
    
    /// 错误统计
    error_statistics: ErrorStatistics,
    
    /// 资源使用
    resource_usage: ResourceUsage,
}

impl IOMonitor {
    /// 生成性能报告
    pub fn generate_report(&self) -> IOReport {
        IOReport {
            // 吞吐量
            read_throughput: self.metrics.read_bytes as f64 / self.metrics.read_time.as_secs_f64(),
            write_throughput: self.metrics.write_bytes as f64 / self.metrics.write_time.as_secs_f64(),
            
            // 延迟
            average_read_latency: self.metrics.read_time / self.metrics.read_operations.max(1) as u32,
            average_write_latency: self.metrics.write_time / self.metrics.write_operations.max(1) as u32,
            
            // 缓存命中率
            cache_hit_rate: self.metrics.cache_hits as f64 / (self.metrics.cache_hits + self.metrics.cache_misses) as f64,
            
            // 内存使用
            memory_usage_mb: self.resource_usage.memory_used / (1024 * 1024),
            mapped_files: self.resource_usage.mapped_files,
            
            // 错误率
            error_rate: self.error_statistics.total_errors as f64 / self.metrics.total_operations as f64,
            
            // 建议
            recommendations: self.generate_recommendations(),
            
            // 警告
            warnings: self.generate_warnings(),
        }
    }
    
    /// 生成优化建议
    fn generate_recommendations(&self) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();
        
        // 检查缓存命中率
        if self.metrics.cache_hit_rate < 0.7 {
            recommendations.push(Recommendation::IncreaseCacheSize {
                current_hit_rate: self.metrics.cache_hit_rate,
                suggested_increase: 50, // 增加50%
            });
        }
        
        // 检查内存映射使用
        if self.metrics.mmap_failures > self.metrics.mmap_successes * 0.1 {
            recommendations.push(Recommendation::AdjustMmapThreshold {
                current_threshold: "10MB".to_string(),
                suggested_threshold: "5MB".to_string(),
                reason: "内存映射失败率过高".to_string(),
            });
        }
        
        // 检查编码检测性能
        if self.metrics.encoding_detection_time > Duration::from_millis(100) {
            recommendations.push(Recommendation::OptimizeEncodingDetection {
                current_time: self.metrics.encoding_detection_time,
                target_time: Duration::from_millis(10),
            });
        }
        
        recommendations
    }
}
```

### 调试工具
```rust
/// IO系统调试器
pub struct IODebugger {
    /// 请求记录器
    request_logger: RequestLogger,
    
    /// 内存分析器
    memory_analyzer: MemoryAnalyzer,
    
    /// 性能分析器
    profiler: IOProfiler,
    
    /// 错误重现器
    error_reproducer: ErrorReproducer,
}

impl IODebugger {
    /// 诊断IO性能问题
    pub fn diagnose_performance_issue(&self, symptom: PerformanceSymptom) -> PerformanceDiagnosis {
        match symptom {
            PerformanceSymptom::SlowFileOpen => {
                self.analyze_file_open_performance()
            }
            
            PerformanceSymptom::HighMemoryUsage => {
                self.analyze_memory_usage()
            }
            
            PerformanceSymptom::SlowEncodingDetection => {
                self.analyze_encoding_detection()
            }
            
            PerformanceSymptom::FileLockContention => {
                self.analyze_file_locking()
            }
        }
    }
    
    /// 分析文件打开性能
    fn analyze_file_open_performance(&self) -> PerformanceDiagnosis {
        let recent_opens = self.request_logger.get_recent_operations(OperationType::FileOpen, 100);
        
        let mut analysis = PerformanceAnalysis::new();
        
        for open in recent_opens {
            analysis.add_metric("total_time", open.duration);
            
            if let Some(sub_ops) = &open.sub_operations {
                for sub_op in sub_ops {
                    match sub_op.name.as_str() {
                        "file_stat" => analysis.add_metric("stat_time", sub_op.duration),
                        "mmap_create" => analysis.add_metric("mmap_time", sub_op.duration),
                        "encoding_detect" => analysis.add_metric("encoding_time", sub_op.duration),
                        _ => {}
                    }
                }
            }
        }
        
        PerformanceDiagnosis {
            issue: PerformanceIssue::SlowFileOpen,
            root_cause: analysis.find_bottleneck(),
            suggested_fixes: vec![
                "启用文件信息缓存".to_string(),
                "调整内存映射策略".to_string(),
                "预加载常用文件".to_string(),
            ],
        }
    }
    
    /// 重现特定错误
    pub fn reproduce_error(&self, error: &IOError) -> Result<ReproductionResult, DebugError> {
        match error {
            IOError::MemoryMapFailed { path, size, .. } => {
                self.reproduce_mmap_failure(path, *size)
            }
            
            IOError::EncodingDetectionFailed { data_len } => {
                self.reproduce_encoding_failure(*data_len)
            }
            
            IOError::FileLocked { path } => {
                self.reproduce_file_lock(path)
            }
            
            _ => Err(DebugError::UnreproducibleError(error.clone())),
        }
    }
}
```

---

*本文档是IO System的实现指南，实施时可进行优化但不违反架构约束。*
```

由于篇幅限制，我先提供核心的设计文档和关键算法。完整的实现代码将包括：

1. **IOSystem主结构**（文件访问枢纽）
2. **MmapManager实现**（内存映射管理）
3. **EncodingDetector实现**（编码自动检测）
4. **AsyncIOExecutor实现**（异步文件操作）
5. **FileCache实现**（智能文件缓存）
6. **完整的测试套件和性能基准**

这个IO System完全遵循您的架构设计：
- 支持大文件内存映射
- 自动编码检测
- 零拷贝读取
- 事务性写入
- 跨平台兼容

您可以根据需要让我提供具体模块的完整代码实现。