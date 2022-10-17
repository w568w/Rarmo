#[derive(Debug)]
pub struct MBR {
    pub partitions: [MBRPartition; 4],
}

impl MBR {
    pub fn parse(buf: &[u8]) -> Self {
        Self {
            partitions: [
                MBRPartition::parse(&buf[0x1BE..0x1CE]),
                MBRPartition::parse(&buf[0x1CE..0x1DE]),
                MBRPartition::parse(&buf[0x1DE..0x1EE]),
                MBRPartition::parse(&buf[0x1EE..0x1FE]),
            ],
        }
    }
}
#[derive(Debug)]
pub struct MBRPartition {
    pub bootable: bool,
    pub start: u32,
    pub size: u32,
}

impl MBRPartition {
    pub fn parse(buf: &[u8]) -> Self {
        let bootable = buf[0] != 0;
        let start = u32::from_le_bytes([buf[0x8], buf[0x9], buf[0xA], buf[0xB]]);
        let size = u32::from_le_bytes([buf[0xC], buf[0xD], buf[0xE], buf[0xF]]);
        Self {
            bootable,
            start,
            size,
        }
    }
}
