/// A generational entity ID.
///
/// When an entity is destroyed, its slot is freed but the index is reused
/// with an incremented generation. This prevents use-after-free bugs where
/// a stale entity ID would incorrectly reference a newly created entity.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    pub fn new(index: u32, generation: u32) -> Self {
        Entity { index, generation }
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }
}
