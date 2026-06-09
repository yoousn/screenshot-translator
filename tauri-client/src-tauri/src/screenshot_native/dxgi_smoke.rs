mod selected_readback;
#[cfg(test)]
mod tests;
mod texture_acquisition;
mod types;

pub use selected_readback::run_dxgi_selected_readback_smoke;
pub use texture_acquisition::run_dxgi_texture_acquisition_smoke;
pub use types::{
    DxgiSelectedReadbackSmokeReport, DxgiSelectedReadbackSmokeStage, DxgiTextureSmokeReport,
    DxgiTextureSmokeStage,
};
