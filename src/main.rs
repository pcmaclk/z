// zedit - A lightweight, modern text editor
//
// Copyright (c) 2025 zedit team
//
// Licensed under MIT License

#![allow(dead_code)]
#![allow(unused_imports)]

mod core;

use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
        )
        .init();

    info!("zedit v0.1.0 starting...");

    // TODO: 运行应用

    Ok(())
}

