#[derive(Debug, Copy, Clone)]
pub struct Port {
    pub pc: u32,
    pub reg_write: Option<(u8, u32)>,
}
