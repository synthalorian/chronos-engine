#[cfg(feature = "asset-pipeline")]
pub mod audio_import;

#[cfg(feature = "asset-pipeline")]
pub mod font_import;

#[cfg(feature = "asset-pipeline")]
pub mod gltf_import;

#[cfg(feature = "asset-pipeline")]
pub mod hotreload;

#[cfg(feature = "asset-pipeline")]
pub mod image_import;

#[cfg(feature = "asset-pipeline")]
pub mod metadata;

#[cfg(feature = "asset-pipeline")]
pub mod processor;

#[cfg(feature = "asset-pipeline")]
pub mod registry;

#[cfg(feature = "asset-pipeline")]
pub mod thumbnail;

// Re-exports for convenient access.
#[cfg(feature = "asset-pipeline")]
pub use metadata::{AssetGuid, AssetMeta, AssetType, ImportSettings, MetaManager};

#[cfg(feature = "asset-pipeline")]
pub use registry::{AssetCatalog, AssetKind, Guid};
