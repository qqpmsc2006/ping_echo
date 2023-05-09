use std::ops::Deref;

pub struct MagicHead([u8; 4]);

impl MagicHead {
    pub fn new() -> Self {
        Self(*b"j-wy")
    }
}

impl Deref for MagicHead {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<[u8]> for MagicHead {
    fn eq(&self, other: &[u8]) -> bool {
        &self.0 == other
    }
}
