use super::MBC;

#[derive(Debug)]
pub struct NoMBC;

impl MBC for NoMBC {
    fn mbc_info(&self) -> String {
        "No MBC".to_string()
    }

    fn rom_offset(&self, addr: u16) -> usize {
        addr as usize
    }

    fn ram_offset(&self, _addr: u16) -> Option<usize> {
        None
    }

    fn write_register(&mut self, _addr: u16, _value: u8) {
        
    }
}