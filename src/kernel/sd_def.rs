#![allow(dead_code)]

use core::ffi::CStr;
use core::ptr;
use crate::aarch64::intrinsic::addr::*;
use crate::aarch64::intrinsic::{delay_us, get_u32, put_u32};
use crate::driver::mbox::get_clock_rate;
use crate::{dsb_sy, println};

// EMMC command flags
pub const CMD_TYPE_NORMAL: u32 = 0x00000000;
pub const CMD_TYPE_SUSPEND: u32 = 0x00400000;
pub const CMD_TYPE_RESUME: u32 = 0x00800000;
pub const CMD_TYPE_ABORT: u32 = 0x00c00000;
pub const CMD_IS_DATA: u32 = 0x00200000;
pub const CMD_IXCHK_EN: u32 = 0x00100000;
pub const CMD_CRCCHK_EN: u32 = 0x00080000;
pub const CMD_RSPNS_NO: u32 = 0x00000000;
pub const CMD_RSPNS_136: u32 = 0x00010000;
pub const CMD_RSPNS_48: u32 = 0x00020000;
pub const CMD_RSPNS_48B: u32 = 0x00030000;
pub const TM_MULTI_BLOCK: u32 = 0x00000020;
pub const TM_DAT_DIR_HC: u32 = 0x00000000;
pub const TM_DAT_DIR_CH: u32 = 0x00000010;
pub const TM_AUTO_CMD23: u32 = 0x00000008;
pub const TM_AUTO_CMD12: u32 = 0x00000004;
pub const TM_BLKCNT_EN: u32 = 0x00000002;
pub const TM_MULTI_DATA: u32 = CMD_IS_DATA | TM_MULTI_BLOCK | TM_BLKCNT_EN;

// INTERRUPT register settings
pub const INT_AUTO_ERROR: u32 = 0x01000000;
pub const INT_DATA_END_ERR: u32 = 0x00400000;
pub const INT_DATA_CRC_ERR: u32 = 0x00200000;
pub const INT_DATA_TIMEOUT: u32 = 0x00100000;
pub const INT_INDEX_ERROR: u32 = 0x00080000;
pub const INT_END_ERROR: u32 = 0x00040000;
pub const INT_CRC_ERROR: u32 = 0x00020000;
pub const INT_CMD_TIMEOUT: u32 = 0x00010000;
pub const INT_ERR: u32 = 0x00008000;
pub const INT_ENDBOOT: u32 = 0x00004000;
pub const INT_BOOTACK: u32 = 0x00002000;
pub const INT_RETUNE: u32 = 0x00001000;
pub const INT_CARD: u32 = 0x00000100;
pub const INT_READ_RDY: u32 = 0x00000020;
pub const INT_WRITE_RDY: u32 = 0x00000010;
pub const INT_BLOCK_GAP: u32 = 0x00000004;
pub const INT_DATA_DONE: u32 = 0x00000002;
pub const INT_CMD_DONE: u32 = 0x00000001;
pub const INT_ERROR_MASK: u32 =
    INT_CRC_ERROR | INT_END_ERROR | INT_INDEX_ERROR | INT_DATA_TIMEOUT |
        INT_DATA_CRC_ERR | INT_DATA_END_ERR | INT_ERR | INT_AUTO_ERROR;
pub const INT_ALL_MASK: u32 =
    INT_CMD_DONE | INT_DATA_DONE | INT_READ_RDY | INT_WRITE_RDY |
        INT_ERROR_MASK;

// CONTROL register settings
pub const C0_SPI_MODE_EN: u32 = 0x00100000;
pub const C0_HCTL_HS_EN: u32 = 0x00000004;
pub const C0_HCTL_DWITDH: u32 = 0x00000002;

pub const C1_SRST_DATA: u32 = 0x04000000;
pub const C1_SRST_CMD: u32 = 0x02000000;
pub const C1_SRST_HC: u32 = 0x01000000;
pub const C1_TOUNIT_DIS: u32 = 0x000f0000;
pub const C1_TOUNIT_MAX: u32 = 0x000e0000;
pub const C1_CLK_GENSEL: u32 = 0x00000020;
pub const C1_CLK_EN: u32 = 0x00000004;
pub const C1_CLK_STABLE: u32 = 0x00000002;
pub const C1_CLK_INTLEN: u32 = 0x00000001;

pub const FREQ_SETUP: u32 = 400000;
// 400 Khz;
pub const FREQ_NORMAL: u32 = 25000000;  // 25 Mhz;

// CONTROL2 values;
pub const C2_VDD_18: u32 = 0x00080000;
pub const C2_UHSMODE: u32 = 0x00070000;
pub const C2_UHS_SDR12: u32 = 0x00000000;
pub const C2_UHS_SDR25: u32 = 0x00010000;
pub const C2_UHS_SDR50: u32 = 0x00020000;
pub const C2_UHS_SDR104: u32 = 0x00030000;
pub const C2_UHS_DDR50: u32 = 0x00040000;

// SLOTISR_VER values;
pub const HOST_SPEC_NUM: u32 = 0x00ff0000;
pub const HOST_SPEC_NUM_SHIFT: u8 = 16;
pub const HOST_SPEC_V3: u32 = 2;
pub const HOST_SPEC_V2: u32 = 1;
pub const HOST_SPEC_V1: u32 = 0;

// STATUS register settings;
pub const SR_DAT_LEVEL1: u32 = 0x1e000000;
pub const SR_CMD_LEVEL: u32 = 0x01000000;
pub const SR_DAT_LEVEL0: u32 = 0x00f00000;
pub const SR_DAT3: u32 = 0x00800000;
pub const SR_DAT2: u32 = 0x00400000;
pub const SR_DAT1: u32 = 0x00200000;
pub const SR_DAT0: u32 = 0x00100000;
pub const SR_WRITE_PROT: u32 = 0x00080000;
// From SDHC spec v2, BCM says reserved
pub const SR_READ_AVAILABLE: u32 = 0x00000800;
// ???? undocumented
pub const SR_WRITE_AVAILABLE: u32 = 0x00000400;
// ???? undocumented
pub const SR_READ_TRANSFER: u32 = 0x00000200;
pub const SR_WRITE_TRANSFER: u32 = 0x00000100;
pub const SR_DAT_ACTIVE: u32 = 0x00000004;
pub const SR_DAT_INHIBIT: u32 = 0x00000002;
pub const SR_CMD_INHIBIT: u32 = 0x00000001;

// Arguments for specific commands.
// TODO: What's the correct voltage window for the RPi SD interface?
// 2.7v-3.6v (given by 0x00ff8000) or something narrower?
// TODO: For now, don't offer to switch voltage.
pub const ACMD41_HCS: u32 = 0x40000000;
pub const ACMD41_SDXC_POWER: u32 = 0x10000000;
pub const ACMD41_S18R: u32 = 0x01000000;
pub const ACMD41_VOLTAGE: u32 = 0x00ff8000;
pub const ACMD41_ARG_HC: u32 =
    ACMD41_HCS | ACMD41_SDXC_POWER | ACMD41_VOLTAGE | ACMD41_S18R;
pub const ACMD41_ARG_SC: u32 = ACMD41_VOLTAGE | ACMD41_S18R;

// R1 (Status) values
pub const ST_OUT_OF_RANGE: u32 = 0x80000000;
// 31   E
pub const ST_ADDRESS_ERROR: u32 = 0x40000000;
// 30   E
pub const ST_BLOCK_LEN_ERROR: u32 = 0x20000000;
// 29   E
pub const ST_ERASE_SEQ_ERROR: u32 = 0x10000000;
// 28   E
pub const ST_ERASE_PARAM_ERROR: u32 = 0x08000000;
// 27   E
pub const ST_WP_VIOLATION: u32 = 0x04000000;
// 26   E
pub const ST_CARD_IS_LOCKED: u32 = 0x02000000;
// 25   E
pub const ST_LOCK_UNLOCK_FAIL: u32 = 0x01000000;
// 24   E
pub const ST_COM_CRC_ERROR: u32 = 0x00800000;
// 23   E
pub const ST_ILLEGAL_COMMAND: u32 = 0x00400000;
// 22   E
pub const ST_CARD_ECC_FAILED: u32 = 0x00200000;
// 21   E
pub const ST_CC_ERROR: u32 = 0x00100000;
// 20   E
pub const ST_ERROR: u32 = 0x00080000;
// 19   E
pub const ST_CSD_OVERWRITE: u32 = 0x00010000;
// 16   E
pub const ST_WP_ERASE_SKIP: u32 = 0x00008000;
// 15   E
pub const ST_CARD_ECC_DISABLED: u32 = 0x00004000;
// 14   E
pub const ST_ERASE_RESET: u32 = 0x00002000;
// 13   E
pub const ST_CARD_STATE: u32 = 0x00001e00;
// 12:9
pub const ST_READY_FOR_DATA: u32 = 0x00000100;
// 8
pub const ST_APP_CMD: u32 = 0x00000020;
// 5
pub const ST_AKE_SEQ_ERROR: u32 = 0x00000004;      // 3    E

pub const R1_CARD_STATE_SHIFT: u8 = 9;
pub const R1_ERRORS_MASK: u32 = 0xfff9c004;  // All above bits which indicate errors.

// R3 (ACMD41 APP_SEND_OP_COND)
pub const R3_COMPLETE: u32 = 0x80000000;
pub const R3_CCS: u32 = 0x40000000;
pub const R3_S18A: u32 = 0x01000000;

// R6 (CMD3 SEND_REL_ADDR)
pub const R6_RCA_MASK: u32 = 0xffff0000;
pub const R6_ERR_MASK: u32 = 0x0000e000;
pub const R6_STATE_MASK: u32 = 0x00001e00;

// Card state values as they appear in the status register.
pub const CS_IDLE: u8 = 0;
// 0x00000000
pub const CS_READY: u8 = 1;
// 0x00000200
pub const CS_IDENT: u8 = 2;
// 0x00000400
pub const CS_STBY: u8 = 3;
// 0x00000600
pub const CS_TRAN: u8 = 4;
// 0x00000800
pub const CS_DATA: u8 = 5;
// 0x00000a00
pub const CS_RCV: u8 = 6;
// 0x00000c00
pub const CS_PRG: u8 = 7;
// 0x00000e00
pub const CS_DIS: u8 = 8;   // 0x00001000

// Response types.
// Note that on the PI, the index and CRC are dropped, leaving 32 bits in RESP0.
pub const RESP_NO: u8 = 0;
// No response
pub const RESP_R1: u8 = 1;
// 48  RESP0    contains card status
pub const RESP_R1B: u8 = 11;
// 48  RESP0    contains card status, data line indicates busy
pub const RESP_R2I: u8 = 2;
// 136 RESP0..3 contains 128 bit CID shifted down by 8 bits as no CRC
pub const RESP_R2S: u8 = 12;
// 136 RESP0..3 contains 128 bit CSD shifted down by 8 bits as no CRC
pub const RESP_R3: u8 = 3;
// 48  RESP0    contains OCR register
pub const RESP_R6: u8 = 6;
// 48  RESP0    contains RCA and status bits 23,22,19,12:0
pub const RESP_R7: u8 = 7;// 48  RESP0    contains voltage acceptance and check pattern

pub const RCA_NO: u8 = 1;
pub const RCA_YES: u8 = 2;

struct EMMCCommand {
    pub name: &'static str,
    pub code: u32,
    pub resp: u8,
    pub rca: u8,
    pub delay: u32,
}

// Command table.
// TODO: TM_DAT_DIR_CH required in any of these?
static SD_COMMAND_TABLE: [EMMCCommand; 39] = [
    EMMCCommand { name: "GO_IDLE_STATE", code: 0x00000000 | CMD_RSPNS_NO, resp: RESP_NO, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "ALL_SEND_CID", code: 0x02000000 | CMD_RSPNS_136, resp: RESP_R2I, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SEND_REL_ADDR", code: 0x03000000 | CMD_RSPNS_48, resp: RESP_R6, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SET_DSR", code: 0x04000000 | CMD_RSPNS_NO, resp: RESP_NO, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SWITCH_FUNC", code: 0x06000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "CARD_SELECT", code: 0x07000000 | CMD_RSPNS_48B, resp: RESP_R1B, rca: RCA_YES, delay: 0 },
    EMMCCommand { name: "SEND_IF_COND", code: 0x08000000 | CMD_RSPNS_48, resp: RESP_R7, rca: RCA_NO, delay: 100 },
    EMMCCommand { name: "SEND_CSD", code: 0x09000000 | CMD_RSPNS_136, resp: RESP_R2S, rca: RCA_YES, delay: 0 },
    EMMCCommand { name: "SEND_CID", code: 0x0A000000 | CMD_RSPNS_136, resp: RESP_R2I, rca: RCA_YES, delay: 0 },
    EMMCCommand { name: "VOLT_SWITCH", code: 0x0B000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "STOP_TRANS", code: 0x0C000000 | CMD_RSPNS_48B, resp: RESP_R1B, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SEND_STATUS", code: 0x0D000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_YES, delay: 0 },
    EMMCCommand { name: "GO_INACTIVE", code: 0x0F000000 | CMD_RSPNS_NO, resp: RESP_NO, rca: RCA_YES, delay: 0 },
    EMMCCommand { name: "SET_BLOCKLEN", code: 0x10000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "READ_SINGLE", code: 0x11000000 | CMD_RSPNS_48 | CMD_IS_DATA | TM_DAT_DIR_CH, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "READ_MULTI", code: 0x12000000 | CMD_RSPNS_48 | TM_MULTI_DATA | TM_DAT_DIR_CH, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SEND_TUNING", code: 0x13000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SPEED_CLASS", code: 0x14000000 | CMD_RSPNS_48B, resp: RESP_R1B, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SET_BLOCKCNT", code: 0x17000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "WRITE_SINGLE", code: 0x18000000 | CMD_RSPNS_48 | CMD_IS_DATA | TM_DAT_DIR_HC, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "WRITE_MULTI", code: 0x19000000 | CMD_RSPNS_48 | TM_MULTI_DATA | TM_DAT_DIR_HC, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "PROGRAM_CSD", code: 0x1B000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SET_WRITE_PR", code: 0x1C000000 | CMD_RSPNS_48B, resp: RESP_R1B, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "CLR_WRITE_PR", code: 0x1D000000 | CMD_RSPNS_48B, resp: RESP_R1B, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SND_WRITE_PR", code: 0x1E000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "ERASE_WR_ST", code: 0x20000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "ERASE_WR_END", code: 0x21000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "ERASE", code: 0x26000000 | CMD_RSPNS_48B, resp: RESP_R1B, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "LOCK_UNLOCK", code: 0x2A000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "APP_CMD", code: 0x37000000 | CMD_RSPNS_NO, resp: RESP_NO, rca: RCA_NO, delay: 100 },
    EMMCCommand { name: "APP_CMD", code: 0x37000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_YES, delay: 0 },
    EMMCCommand { name: "GEN_CMD", code: 0x38000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },

// APP commands must be prefixed by an APP_CMD.
    EMMCCommand { name: "SET_BUS_WIDTH", code: 0x06000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SD_STATUS", code: 0x0D000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_YES, delay: 0 },  // RCA???
    EMMCCommand { name: "SEND_NUM_WRBL", code: 0x16000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SEND_NUM_ERS", code: 0x17000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SD_SENDOPCOND", code: 0x29000000 | CMD_RSPNS_48, resp: RESP_R3, rca: RCA_NO, delay: 1000 },
    EMMCCommand { name: "SET_CLR_DET", code: 0x2A000000 | CMD_RSPNS_48, resp: RESP_R1, rca: RCA_NO, delay: 0 },
    EMMCCommand { name: "SEND_SCR", code: 0x33000000 | CMD_RSPNS_48 | CMD_IS_DATA | TM_DAT_DIR_CH, resp: RESP_R1, rca: RCA_NO, delay: 0 },
];

// Command indexes in the command table
pub const IX_GO_IDLE_STATE: usize = 0;
pub const IX_ALL_SEND_CID: usize = 1;
pub const IX_SEND_REL_ADDR: usize = 2;
pub const IX_SET_DSR: usize = 3;
pub const IX_SWITCH_FUNC: usize = 4;
pub const IX_CARD_SELECT: usize = 5;
pub const IX_SEND_IF_COND: usize = 6;
pub const IX_SEND_CSD: usize = 7;
pub const IX_SEND_CID: usize = 8;
pub const IX_VOLTAGE_SWITCH: usize = 9;
pub const IX_STOP_TRANS: usize = 10;
pub const IX_SEND_STATUS: usize = 11;
pub const IX_GO_INACTIVE: usize = 12;
pub const IX_SET_BLOCKLEN: usize = 13;
pub const IX_READ_SINGLE: usize = 14;
pub const IX_READ_MULTI: usize = 15;
pub const IX_SEND_TUNING: usize = 16;
pub const IX_SPEED_CLASS: usize = 17;
pub const IX_SET_BLOCKCNT: usize = 18;
pub const IX_WRITE_SINGLE: usize = 19;
pub const IX_WRITE_MULTI: usize = 20;
pub const IX_PROGRAM_CSD: usize = 21;
pub const IX_SET_WRITE_PR: usize = 22;
pub const IX_CLR_WRITE_PR: usize = 23;
pub const IX_SND_WRITE_PR: usize = 24;
pub const IX_ERASE_WR_ST: usize = 25;
pub const IX_ERASE_WR_END: usize = 26;
pub const IX_ERASE: usize = 27;
pub const IX_LOCK_UNLOCK: usize = 28;
pub const IX_APP_CMD: usize = 29;
pub const IX_APP_CMD_RCA: usize = 30;
// APP_CMD used once we have the RCA.
pub const IX_GEN_CMD: usize = 31;

// Commands hereafter require APP_CMD.
pub const IX_APP_CMD_START: usize = 32;
pub const IX_SET_BUS_WIDTH: usize = 32;
pub const IX_SD_STATUS: usize = 33;
pub const IX_SEND_NUM_WRBL: usize = 34;
pub const IX_SEND_NUM_ERS: usize = 35;
pub const IX_APP_SEND_OP_COND: usize = 36;
pub const IX_SET_CLR_DET: usize = 37;
pub const IX_SEND_SCR: usize = 38;

// CSD flags
// Note: all flags are shifted down by 8 bits as the CRC is not included.
// Most flags are common:
// in V1 the size is 12 bits with a 3 bit multiplier.
// in V1 currents for read and write are specified.
// in V2 the size is 22 bits, no multiplier, no currents.
pub const CSD0_VERSION: u32 = 0x00c00000;
pub const CSD0_V1: u32 = 0x00000000;
pub const CSD0_V2: u32 = 0x00400000;

// CSD Version 1 and 2 flags
pub const CSD1VN_TRAN_SPEED: u32 = 0xff000000;

pub const CSD1VN_CCC: u32 = 0x00fff000;
pub const CSD1VN_READ_BL_LEN: u32 = 0x00000f00;
pub const CSD1VN_READ_BL_LEN_SHIFT: u8 = 8;
pub const CSD1VN_READ_BL_PARTIAL: u32 = 0x00000080;
pub const CSD1VN_WRITE_BLK_MISALIGN: u32 = 0x00000040;
pub const CSD1VN_READ_BLK_MISALIGN: u32 = 0x00000020;
pub const CSD1VN_DSR_IMP: u32 = 0x00000010;

pub const CSD2VN_ERASE_BLK_EN: u32 = 0x00000040;
pub const CSD2VN_ERASE_SECTOR_SIZEH: u32 = 0x0000003f;
pub const CSD3VN_ERASE_SECTOR_SIZEL: u32 = 0x80000000;

pub const CSD3VN_WP_GRP_SIZE: u32 = 0x7f000000;

pub const CSD3VN_WP_GRP_ENABLE: u32 = 0x00800000;
pub const CSD3VN_R2W_FACTOR: u32 = 0x001c0000;
pub const CSD3VN_WRITE_BL_LEN: u32 = 0x0003c000;
pub const CSD3VN_WRITE_BL_LEN_SHIFT: u8 = 14;
pub const CSD3VN_WRITE_BL_PARTIAL: u32 = 0x00002000;
pub const CSD3VN_FILE_FORMAT_GROUP: u32 = 0x00000080;
pub const CSD3VN_COPY: u32 = 0x00000040;
pub const CSD3VN_PERM_WRITE_PROT: u32 = 0x00000020;
pub const CSD3VN_TEMP_WRITE_PROT: u32 = 0x00000010;
pub const CSD3VN_FILE_FORMAT: u32 = 0x0000000c;
pub const CSD3VN_FILE_FORMAT_HDD: u32 = 0x00000000;
pub const CSD3VN_FILE_FORMAT_DOSFAT: u32 = 0x00000004;
pub const CSD3VN_FILE_FORMAT_UFF: u32 = 0x00000008;
pub const CSD3VN_FILE_FORMAT_UNKNOWN: u32 = 0x0000000c;

// CSD Version 1 flags.
pub const CSD1V1_C_SIZEH: u32 = 0x00000003;
pub const CSD1V1_C_SIZEH_SHIFT: u8 = 10;

pub const CSD2V1_C_SIZEL: u32 = 0xffc00000;
pub const CSD2V1_C_SIZEL_SHIFT: u8 = 22;
pub const CSD2V1_VDD_R_CURR_MIN: u32 = 0x00380000;
pub const CSD2V1_VDD_R_CURR_MAX: u32 = 0x00070000;
pub const CSD2V1_VDD_W_CURR_MIN: u32 = 0x0000e000;
pub const CSD2V1_VDD_W_CURR_MAX: u32 = 0x00001c00;
pub const CSD2V1_C_SIZE_MULT: u32 = 0x00000380;
pub const CSD2V1_C_SIZE_MULT_SHIFT: u8 = 7;

// CSD Version 2 flags.
pub const CSD2V2_C_SIZE: u32 = 0x3fffff00;
pub const CSD2V2_C_SIZE_SHIFT: u8 = 8;

// SCR flags
// NOTE: SCR is big-endian, so flags appear byte-wise reversed from the spec.
pub const SCR_STRUCTURE: u32 = 0x000000f0;
pub const SCR_STRUCTURE_V1: u32 = 0x00000000;

pub const SCR_SD_SPEC: u32 = 0x0000000f;
pub const SCR_SD_SPEC_1_101: u32 = 0x00000000;
pub const SCR_SD_SPEC_11: u32 = 0x00000001;
pub const SCR_SD_SPEC_2_3: u32 = 0x00000002;

pub const SCR_DATA_AFTER_ERASE: u32 = 0x00008000;

pub const SCR_SD_SECURITY: u32 = 0x00007000;
pub const SCR_SD_SEC_NONE: u32 = 0x00000000;
pub const SCR_SD_SEC_NOT_USED: u32 = 0x00001000;
pub const SCR_SD_SEC_101: u32 = 0x00002000;
// SDSC
pub const SCR_SD_SEC_2: u32 = 0x00003000;
// SDHC
pub const SCR_SD_SEC_3: u32 = 0x00004000;    // SDXC

pub const SCR_SD_BUS_WIDTHS: u32 = 0x00000f00;
pub const SCR_SD_BUS_WIDTH_1: u32 = 0x00000100;
pub const SCR_SD_BUS_WIDTH_4: u32 = 0x00000400;

pub const SCR_SD_SPEC3: u32 = 0x00800000;
pub const SCR_SD_SPEC_2: u32 = 0x00000000;
pub const SCR_SD_SPEC_3: u32 = 0x00100000;

pub const SCR_EX_SECURITY: u32 = 0x00780000;

pub const SCR_CMD_SUPPORT: u32 = 0x03000000;
pub const SCR_CMD_SUPP_SET_BLKCNT: u32 = 0x02000000;
pub const SCR_CMD_SUPP_SPEED_CLASS: u32 = 0x01000000;

// SD card types
pub const SD_TYPE_MMC: u8 = 1;
pub const SD_TYPE_1: u8 = 2;
pub const SD_TYPE_2_SC: u8 = 3;
pub const SD_TYPE_2_HC: u8 = 4;

pub const SD_TYPE_NAME: [&'static str; 5] = ["Unknown", "MMC", "Type 1", "Type 2 SC", "Type 2 HC"];

// SD card functions supported values.
pub const SD_SUPP_SET_BLOCK_COUNT: u32 = 0x80000000;
pub const SD_SUPP_SPEED_CLASS: u32 = 0x40000000;
pub const SD_SUPP_BUS_WIDTH_4: u32 = 0x20000000;
pub const SD_SUPP_BUS_WIDTH_1: u32 = 0x10000000;

pub const SD_OK: u32 = 0;
pub const SD_ERROR: u32 = 1;
pub const SD_TIMEOUT: u32 = 2;
pub const SD_BUSY: u32 = 3;
pub const SD_NO_RESP: u32 = 5;
pub const SD_ERROR_RESET: u32 = 6;
pub const SD_ERROR_CLOCK: u32 = 7;
pub const SD_ERROR_VOLTAGE: u32 = 8;
pub const SD_ERROR_APP_CMD: u32 = 9;
pub const SD_CARD_CHANGED: u32 = 10;
pub const SD_CARD_ABSENT: u32 = 11;
pub const SD_CARD_REINSERTED: u32 = 12;

pub const SD_READ_BLOCKS: u32 = 0;
pub const SD_WRITE_BLOCKS: u32 = 1;

pub struct SDDescriptor {
    // Static information about the SD Card.
    pub capacity: u64,
    pub cid: [u32; 4],
    pub csd: [u32; 4],
    pub scr: [u32; 4],
    pub ocr: u32,
    pub support: u32,
    pub file_format: u32,
    pub typ: u8,
    pub uhsi: u8,
    pub init: bool,
    pub absent: bool,

    // Dynamic information.
    pub rca: u32,
    pub card_state: u32,
    pub status: u32,

    last_cmd: *const EMMCCommand,
    pub last_arg: u32,
}

pub static mut SD_CARD: SDDescriptor = SDDescriptor {
    capacity: 0,
    cid: [0; 4],
    csd: [0; 4],
    scr: [0; 4],
    ocr: 0,
    support: 0,
    file_format: 0,
    typ: 0,
    uhsi: 0,
    init: false,
    absent: true,
    rca: 0,
    card_state: 0,
    status: 0,
    last_cmd: ptr::null_mut(),
    last_arg: 0,
};
static mut SD_HOST_VER: u32 = 0;
static SD_DEBUG: i32 = 0;
static mut SD_BASE_CLOCK: u32 = 0;

fn sd_delay_us(us: u32) {
    delay_us((us as u64) * 3);
}

pub fn sd_wait_for_interrupt(mask: u32) -> Result<u32, u32> {
    let mut timeout = 1_000_000;
    let wait_mask = mask | INT_ERROR_MASK;
    while get_u32(EMMC_INTERRUPT) & wait_mask == 0 && timeout > 0 {
        timeout -= 1;
        sd_delay_us(1);
    }
    let ival = get_u32(EMMC_INTERRUPT);
    if timeout <= 0 || (ival & INT_CMD_TIMEOUT) != 0 || (ival & INT_DATA_TIMEOUT) != 0 {
        put_u32(EMMC_INTERRUPT, ival);
        return Err(SD_TIMEOUT);
    } else if ival & INT_ERROR_MASK != 0 {
        put_u32(EMMC_INTERRUPT, ival);
        return Err(SD_ERROR);
    }
    put_u32(EMMC_INTERRUPT, mask);
    Ok(SD_OK)
}

fn sd_wait_for_command() -> Result<u32, u32> {
    let mut timeout = 1_000_000;
    while get_u32(EMMC_STATUS) & SR_CMD_INHIBIT != 0 &&
        get_u32(EMMC_INTERRUPT) & INT_ERROR_MASK == 0 &&
        timeout > 0 {
        timeout -= 1;
        sd_delay_us(1);
    }
    if timeout <= 0 || get_u32(EMMC_INTERRUPT) & INT_ERROR_MASK != 0 {
        Err(SD_BUSY)
    } else {
        Ok(SD_OK)
    }
}

fn sd_wait_for_data() -> Result<u32, u32> {
    let mut timeout = 500_000;
    while get_u32(EMMC_STATUS) & SR_DAT_INHIBIT != 0 &&
        get_u32(EMMC_INTERRUPT) & INT_ERROR_MASK == 0 &&
        timeout > 0 {
        timeout -= 1;
        sd_delay_us(1);
    }
    if timeout <= 0 || get_u32(EMMC_INTERRUPT) & INT_ERROR_MASK != 0 {
        Err(SD_BUSY)
    } else {
        Ok(SD_OK)
    }
}

unsafe fn sd_send_command_p(cmd: &EMMCCommand, arg: u32) -> Result<u32, u32> {
    let mut status = sd_wait_for_command()?;
    SD_CARD.last_cmd = cmd;
    SD_CARD.last_arg = arg;

    put_u32(EMMC_INTERRUPT, get_u32(EMMC_INTERRUPT));
    put_u32(EMMC_ARG1, arg);
    put_u32(EMMC_CMDTM, cmd.code);
    if cmd.delay != 0 {
        sd_delay_us(cmd.delay);
    }
    status = sd_wait_for_interrupt(INT_CMD_DONE)?;
    let resp0 = get_u32(EMMC_RESP0);
    match cmd.resp {
        RESP_NO => {
            return Ok(SD_OK);
        }
        RESP_R1 | RESP_R1B => {
            SD_CARD.status = resp0;
            SD_CARD.card_state = (resp0 & ST_CARD_STATE) >> R1_CARD_STATE_SHIFT;
            return match resp0 & R1_ERRORS_MASK {
                SD_OK => Ok(SD_OK),
                code @ _ => Err(code),
            };
        }
        RESP_R2I | RESP_R2S => {
            SD_CARD.status = 0;
            let data = if cmd.resp == RESP_R2I {
                &mut SD_CARD.cid
            } else {
                &mut SD_CARD.csd
            };
            data[0] = get_u32(EMMC_RESP3);
            data[1] = get_u32(EMMC_RESP2);
            data[2] = get_u32(EMMC_RESP1);
            data[3] = resp0;
            return Ok(SD_OK);
        }
        RESP_R3 => {
            SD_CARD.status = 0;
            SD_CARD.ocr = resp0;
            return Ok(SD_OK);
        }
        RESP_R6 => {
            SD_CARD.rca = resp0 & R6_RCA_MASK;
            SD_CARD.status = (resp0 & 0x00001fff)
                |
                ((resp0 & 0x00002000) << 6)
                |
                ((resp0 & 0x00004000) << 8)
                |
                ((resp0 & 0x00008000) << 8);
            SD_CARD.card_state = (resp0 & ST_CARD_STATE) >> R1_CARD_STATE_SHIFT;
            return match SD_CARD.card_state & R6_ERR_MASK {
                SD_OK => Ok(SD_OK),
                code @ _ => Err(code),
            };
        }
        RESP_R7 => {
            SD_CARD.status = 0;
            return if resp0 == arg {
                Ok(SD_OK)
            } else {
                Err(SD_ERROR)
            };
        }
        _ => unreachable!("Unknown response type"),
    };
}

unsafe fn sd_send_app_command() -> Result<u32, u32> {
    if SD_CARD.rca == 0 {
        let _ = sd_send_command_p(&SD_COMMAND_TABLE[IX_APP_CMD], 0);
    } else {
        let _ = sd_send_command_p(&SD_COMMAND_TABLE[IX_APP_CMD_RCA], SD_CARD.rca)?;
        if SD_CARD.status & ST_APP_CMD == 0 {
            return Err(SD_ERROR);
        }
    }
    Ok(SD_OK)
}

unsafe fn sd_send_command(index: usize) -> Result<u32, u32> {
    if index >= IX_APP_CMD_START {
        let _ = sd_send_app_command()?;
    }

    let cmd = &SD_COMMAND_TABLE[index];
    let mut arg = 0;
    if cmd.rca == RCA_YES {
        arg = SD_CARD.rca;
    }
    let resp = sd_send_command_p(cmd, arg)?;
    if index >= IX_APP_CMD_START && SD_CARD.rca != 0 && SD_CARD.status & ST_APP_CMD == 0 {
        return Err(SD_ERROR);
    }
    Ok(resp)
}

pub unsafe fn sd_send_command_a(index: usize, arg: u32) -> Result<u32, u32> {
    if index >= IX_APP_CMD_START {
        let _ = sd_send_app_command()?;
    }

    let cmd = &SD_COMMAND_TABLE[index];
    let resp = sd_send_command_p(cmd, arg)?;
    if index >= IX_APP_CMD_START && SD_CARD.rca != 0 && SD_CARD.status & ST_APP_CMD == 0 {
        return Err(SD_ERROR_APP_CMD);
    }
    Ok(resp)
}

unsafe fn sd_read_scr() -> Result<u32, u32> {
    if sd_wait_for_data().is_err() {
        return Err(SD_TIMEOUT);
    }
    put_u32(EMMC_BLKSIZECNT, (1 << 16) | 8);
    let mut resp = sd_send_command(IX_SEND_SCR)?;

    resp = sd_wait_for_interrupt(INT_READ_RDY)?;

    let mut num_read = 0;
    let mut timeout = 100_000;
    while num_read < 2 && timeout > 0 {
        if get_u32(EMMC_STATUS) & SR_READ_AVAILABLE != 0 {
            SD_CARD.scr[num_read] = get_u32(EMMC_DATA);
            num_read += 1;
        } else {
            timeout -= 1;
            sd_delay_us(1);
        }
    }

    if num_read != 2 {
        return Err(SD_TIMEOUT);
    }

    if SD_CARD.scr[0] & SCR_SD_BUS_WIDTH_4 != 0 { SD_CARD.support |= SD_SUPP_BUS_WIDTH_4; }
    if SD_CARD.scr[0] & SCR_SD_BUS_WIDTH_1 != 0 { SD_CARD.support |= SD_SUPP_BUS_WIDTH_1; }
    if SD_CARD.scr[0] & SCR_CMD_SUPP_SET_BLKCNT != 0 { SD_CARD.support |= SD_SUPP_SET_BLOCK_COUNT; }
    if SD_CARD.scr[0] & SCR_CMD_SUPP_SPEED_CLASS != 0 { SD_CARD.support |= SD_SUPP_SPEED_CLASS; }

    Ok(SD_OK)
}

fn fls_long(mut x: u32) -> i32 {
    let mut r = 32;
    if x == 0 {
        return 0;
    }
    if (x & 0xffff0000) == 0 {
        x <<= 16;
        r -= 16;
    }
    if (x & 0xff000000) == 0 {
        x <<= 8;
        r -= 8;
    }
    if (x & 0xf0000000) == 0 {
        x <<= 4;
        r -= 4;
    }
    if (x & 0xc0000000) == 0 {
        x <<= 2;
        r -= 2;
    }
    if (x & 0x80000000) == 0 {
        x <<= 1;
        r -= 1;
    }
    r
}

fn roundup_pow_of_two(x: u32) -> u32 {
    1 << fls_long(x - 1)
}

unsafe fn sd_get_clock_divider(freq: u32) -> u32 {
    let mut divisor = 0u32;
    let closet = 41_666_666 / freq;
    let mut shift_count = fls_long(closet - 1);

    if shift_count > 0 {
        shift_count -= 1;
    }
    if shift_count > 7 {
        shift_count = 7;
    }

    divisor = if SD_HOST_VER > HOST_SPEC_V2 {
        closet
    } else {
        1 << shift_count
    };
    if divisor <= 2 {
        divisor = 2;
        shift_count = 0;
    }

    let mut hi: u32 = 0;
    if SD_HOST_VER > HOST_SPEC_V2 {
        hi = (divisor & 0x300) >> 2;
    }
    let lo = divisor & 0xff;
    (lo << 8) + hi
}

unsafe fn sd_set_clock(freq: u32) -> Result<u32, u32> {
    let mut timeout = 100_000;
    while get_u32(EMMC_STATUS) & (SR_CMD_INHIBIT | SR_DAT_INHIBIT) != 0 && timeout > 0 {
        timeout -= 1;
        sd_delay_us(1);
    }
    if timeout <= 0 {
        return Err(SD_ERROR_CLOCK);
    }

    let ctrl = get_u32(EMMC_CONTROL1);
    put_u32(EMMC_CONTROL1, ctrl & !C1_CLK_EN);
    sd_delay_us(10);

    let cdiv = sd_get_clock_divider(freq);
    let ctrl = get_u32(EMMC_CONTROL1);
    put_u32(EMMC_CONTROL1, (ctrl & 0xffff003f) | cdiv);
    sd_delay_us(10);

    let ctrl = get_u32(EMMC_CONTROL1);
    put_u32(EMMC_CONTROL1, ctrl | C1_CLK_EN);
    sd_delay_us(10);

    let mut timeout = 10_000;
    while get_u32(EMMC_CONTROL1) & C1_CLK_STABLE == 0 && timeout > 0 {
        timeout -= 1;
        sd_delay_us(10);
    }
    if timeout <= 0 {
        return Err(SD_ERROR_CLOCK);
    }
    Ok(SD_OK)
}

unsafe fn sd_reset_card(reset_type: u32) -> Result<u32, u32> {
    put_u32(EMMC_CONTROL0, 0);
    let ctrl = get_u32(EMMC_CONTROL1);
    put_u32(EMMC_CONTROL1, ctrl | reset_type);
    sd_delay_us(10);

    let mut timeout = 10_000;
    while get_u32(EMMC_CONTROL1) & reset_type != 0 && timeout > 0 {
        timeout -= 1;
        sd_delay_us(10);
    }
    if timeout <= 0 {
        return Err(SD_ERROR_RESET);
    }

    let ctrl = get_u32(EMMC_CONTROL1);
    put_u32(EMMC_CONTROL1, ctrl | C1_CLK_INTLEN | C1_TOUNIT_MAX);
    sd_delay_us(10);

    let _ = sd_set_clock(FREQ_SETUP)?;

    put_u32(EMMC_IRPT_EN, 0xffffffff & (!INT_CMD_DONE) & (!INT_WRITE_RDY));
    put_u32(EMMC_IRPT_MASK, 0xffffffff);

    SD_CARD.rca = 0;
    SD_CARD.ocr = 0;
    SD_CARD.last_arg = 0;
    SD_CARD.last_cmd = ptr::null_mut();
    SD_CARD.status = 0;
    SD_CARD.typ = 0;
    SD_CARD.uhsi = 0;

    sd_send_command(IX_GO_IDLE_STATE)
}

unsafe fn sd_app_send_op_cond(arg: u32) -> Result<u32, u32> {
    let resp = sd_send_command_a(IX_APP_SEND_OP_COND, arg);
    if resp.is_err_and(|e| *e != SD_TIMEOUT) {
        return resp;
    }
    let mut count = 6;
    while SD_CARD.ocr & R3_COMPLETE == 0 && count > 0 {
        sd_delay_us(50_000);
        let resp = sd_send_command_a(IX_APP_SEND_OP_COND, arg);
        if resp.is_err_and(|e| *e != SD_TIMEOUT) {
            return resp;
        }
        count -= 1;
    }

    if SD_CARD.ocr & R3_COMPLETE == 0 {
        return Err(SD_TIMEOUT);
    }

    if SD_CARD.ocr & ACMD41_VOLTAGE == 0 {
        return Err(SD_ERROR_VOLTAGE);
    }

    Ok(SD_OK)
}

fn sd_switch_voltage() -> Result<u32,u32> {
    Ok(SD_OK)
}

fn sd_init_gpio() {
    let mut r = get_u32(GPFSEL4);
    r &= !(7 << 21);
    put_u32(GPFSEL4, r);
    put_u32(GPPUD, 2);
    delay_us(150);
    put_u32(GPPUDCLK1, 1 << 15);
    delay_us(150);
    put_u32(GPPUD, 0);
    put_u32(GPPUDCLK1, 0);

    r = get_u32(GPHEN1);
    r |= 1 << 15;
    put_u32(GPHEN1, r);

    r = get_u32(GPFSEL4);
    r |= (7 << (8 * 3)) | (7 << (9 * 3));
    put_u32(GPFSEL4, r);
    put_u32(GPPUD, 2);
    delay_us(150);
    put_u32(GPPUDCLK1, (1 << 16) | (1 << 17));
    delay_us(150);
    put_u32(GPPUD, 0);
    put_u32(GPPUDCLK1, 0);

    r = get_u32(GPFSEL5);
    r |= (7 << (0 * 3)) | (7 << (1 * 3)) | (7 << (2 * 3)) | (7 << (3 * 3));
    put_u32(GPFSEL5, r);
    put_u32(GPPUD, 2);
    delay_us(150);
    put_u32(GPPUDCLK1, (1 << 18) | (1 << 19) | (1 << 20) | (1 << 21));
    delay_us(150);
    put_u32(GPPUD, 0);
    put_u32(GPPUDCLK1, 0);
}

unsafe fn sd_get_base_clock() -> Result<u32, u32> {
    SD_BASE_CLOCK = get_clock_rate();
    if SD_BASE_CLOCK == u32::MAX {
        return Err(SD_ERROR);
    }
    Ok(SD_OK)
}

pub unsafe fn sd_init() -> Result<u32, u32> {
    if !SD_CARD.init {
        sd_init_gpio();
    }

    let card_absent = 0;
    let card_ejected = get_u32(GPEDS1) & (1 << 15);
    let mut old_cid: [u32; 4] = [0; 4];
    if card_absent != 0 {
        return Err(SD_CARD_ABSENT);
    }

    SD_CARD.absent = false;
    if card_ejected != 0 && SD_CARD.init {
        SD_CARD.init = false;
        old_cid[0] = SD_CARD.cid[0];
        old_cid[1] = SD_CARD.cid[1];
        old_cid[2] = SD_CARD.cid[2];
        old_cid[3] = SD_CARD.cid[3];
    };

    if SD_CARD.init {
        return Ok(SD_OK);
    }

    SD_HOST_VER = (get_u32(EMMC_SLOTISR_VER) & HOST_SPEC_NUM) >> HOST_SPEC_NUM_SHIFT;

    let mut resp = sd_get_base_clock()?;

    resp = sd_reset_card(C1_SRST_HC)?;

    dsb_sy();
    resp = sd_send_command_a(IX_SEND_IF_COND, 0x1aa).unwrap_or_else(|e| e);
    match resp {
        SD_OK => {
            delay_us(50);
            let _ = sd_app_send_op_cond(ACMD41_ARG_HC)?;
            if SD_CARD.ocr & R3_CCS != 0 {
                SD_CARD.typ = SD_TYPE_2_HC;
            } else {
                SD_CARD.typ = SD_TYPE_2_SC;
            }
        }
        SD_BUSY => {
            return Err(resp);
        }
        _ => {
            if get_u32(EMMC_STATUS) & SR_DAT_INHIBIT != 0 {
                let _ = sd_reset_card(C1_SRST_HC)?;
            }
            let _ = sd_app_send_op_cond(ACMD41_ARG_SC)?;
            SD_CARD.typ = SD_TYPE_1;
        }
    };

    if SD_CARD.ocr & R3_S18A != 0 {
        let _ = sd_switch_voltage()?;
    }

    resp = sd_send_command(IX_ALL_SEND_CID)?;
    resp = sd_send_command(IX_SEND_REL_ADDR)?;
    resp = sd_send_command(IX_SEND_CSD)?;

    sd_parse_csd();

    if SD_CARD.file_format != CSD3VN_FILE_FORMAT_DOSFAT && SD_CARD.file_format != CSD3VN_FILE_FORMAT_HDD {
        return Err(SD_ERROR);
    }

    resp = sd_set_clock(FREQ_NORMAL)?;

    resp = sd_send_command(IX_CARD_SELECT)?;

    resp = sd_read_scr()?;

    if SD_CARD.support & SD_SUPP_BUS_WIDTH_4 != 0 {
        let _ = sd_send_command_a(IX_SET_BUS_WIDTH, SD_CARD.rca | 2)?;
        let ctrl = get_u32(EMMC_CONTROL0);
        put_u32(EMMC_CONTROL0, ctrl | C0_HCTL_DWITDH);
    }

    resp = sd_send_command_a(IX_SET_BLOCKLEN, 512)?;

    sd_parse_cid();

    SD_CARD.init = true;

    if old_cid[0] != SD_CARD.cid[0] || old_cid[1] != SD_CARD.cid[1] || old_cid[2] != SD_CARD.cid[2] || old_cid[3] != SD_CARD.cid[3] {
        Ok(SD_CARD_CHANGED)
    } else {
        Ok(SD_CARD_REINSERTED)
    }
}

unsafe fn sd_parse_cid() {
    let man_id = (SD_CARD.cid[0] & 0x00ff0000) >> 16;
    let app_id: [u8; 3] = [
        ((SD_CARD.cid[0] & 0x0000ff00) >> 8) as u8,
        (SD_CARD.cid[0] & 0x000000ff) as u8,
        0,
    ];
    let name: [u8; 6] = [
        ((SD_CARD.cid[1] & 0xff000000) >> 24) as u8,
        ((SD_CARD.cid[1] & 0x00ff0000) >> 16) as u8,
        ((SD_CARD.cid[1] & 0x0000ff00) >> 8) as u8,
        (SD_CARD.cid[1] & 0x000000ff) as u8,
        ((SD_CARD.cid[2] & 0xff000000) >> 24) as u8,
        0,
    ];
    let rev_h = (SD_CARD.cid[2] & 0x00f00000) >> 20;
    let rev_l = (SD_CARD.cid[2] & 0x000f0000) >> 16;
    let serial = ((SD_CARD.cid[2] & 0x0000ffff) << 16) +
        ((SD_CARD.cid[3] & 0xffff0000) >> 16);

    // For some reason cards I have looked at seem to have the Y/M in
    // bits 11:0 whereas the spec says they should be in bits 19:8
    let date_y = ((SD_CARD.cid[3] & 0x00000ff0) >> 4) + 2000;
    let date_m = SD_CARD.cid[3] & 0x0000000f;

    println!("CMMD: SD Card {}, {}Mb, UHS-I {}, mfr {}, '{}:{}' r{}.{} {}/{}, #{} RCA {}",
             SD_TYPE_NAME[SD_CARD.typ as usize], SD_CARD.capacity >> 20, SD_CARD.uhsi, man_id,
             CStr::from_bytes_until_nul(&app_id).unwrap().to_str().unwrap(),
             CStr::from_bytes_until_nul(&name).unwrap().to_str().unwrap(),
             rev_h, rev_l, date_m, date_y, serial, SD_CARD.rca >> 16);
}

unsafe fn sd_parse_csd() {
    let csd_version = SD_CARD.csd[0] & CSD0_VERSION;

    if csd_version == CSD0_V1 {
        let csize =
            ((SD_CARD.csd[1] & CSD1V1_C_SIZEH) << CSD1V1_C_SIZEH_SHIFT) +
                ((SD_CARD.csd[2] & CSD2V1_C_SIZEL) >> CSD2V1_C_SIZEL_SHIFT);
        let mult = 1 << (((SD_CARD.csd[2] & CSD2V1_C_SIZE_MULT) >>
            CSD2V1_C_SIZE_MULT_SHIFT) + 2);
        let block_size = 1u64 << ((SD_CARD.csd[1] & CSD1VN_READ_BL_LEN) >>
            CSD1VN_READ_BL_LEN_SHIFT);
        let num_blocks = (csize as u64 + 1) * mult;
        SD_CARD.capacity = num_blocks * block_size;
    } else {
        let csize =
            (SD_CARD.csd[2] & CSD2V2_C_SIZE) >> CSD2V2_C_SIZE_SHIFT;
        SD_CARD.capacity = (csize as u64 + 1) * 512 * 1024;
    }
    SD_CARD.file_format = SD_CARD.csd[3] & CSD3VN_FILE_FORMAT;
}

