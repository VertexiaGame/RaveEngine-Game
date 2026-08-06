pub mod godot;
pub mod reader;
#[cfg(test)]
pub mod tests;
pub mod types;
pub mod writer;

pub use types::{
    FORMAT_VERSION, VrtxBrick, VrtxFileState, VrtxLighting, VrtxScript, VrtxSettings,
};

impl VrtxFileState {
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        writer::save_to_file(self, path)
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        reader::load_from_file(path)
    }
}
