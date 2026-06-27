//! 纯领域逻辑层：模型、SourceAdapter trait、归一去重。
//! 不依赖 IO（无 sqlx / reqwest / axum），保证核心逻辑可纯单元测试。

pub mod error;
pub mod models;
pub mod normalize;
pub mod source;
