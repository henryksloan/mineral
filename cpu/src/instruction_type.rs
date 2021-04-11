// https://www.ecs.csun.edu/~smirzaei/docs/ece425/arm7tdmi_instruction_set_reference.pdf page 1
#[derive(Debug)]
pub enum InstructionType {
    Multiply,
    MultiplyLong,
    BranchExchange,
    SingleSwap,
    HalfwordTransferReg,
    HalfwordTransferImm,
    SingleTransfer,
    DataProcessing,
    Undefined,
    BlockTransfer,
    Branch,
    CoprocDataTransfer,
    CoprocOperation,
    CoprocRegOperation,
    SoftwareInterrupt,
}

impl InstructionType {
    pub fn from_encoding(encoding: u32) -> Self {
        let hi_8 = (encoding >> 20) & 0xFF;
        let lo_8 = (encoding >> 4) & 0xFF;
        let lo_4 = lo_8 & 0xF;

        if (hi_8 | 0b11 == 0b00000011) && (lo_4 == 0b1001) {
            Self::Multiply
        } else if (hi_8 | 0b111 == 0b00001111) && (lo_4 == 0b1001) {
            Self::MultiplyLong
        } else if (encoding >> 4) & 0xFFFFFF == 0x12fff1 {
            Self::BranchExchange
        } else if (hi_8 | 0b100 == 0b00010100) && (lo_8 == 0b00001001) {
            Self::SingleSwap
        } else if (hi_8 | 0b11011 == 0b00011011) && (lo_8 == 0b00001011) {
            Self::HalfwordTransferReg
        } else if (hi_8 | 0b11011 == 0b00011111) && (lo_4 == 0b1011) {
            Self::HalfwordTransferImm
        } else if hi_8 | 0b111111 == 0b00111111 {
            Self::DataProcessing
        } else if hi_8 | 0b111111 == 0b01111111 {
            if (encoding >> 25) & 1 == 1 && (encoding >> 4) & 1 == 1 {
                Self::Undefined
            } else {
                Self::SingleTransfer
            }
        } else if hi_8 | 0b11011 == 0b10011011 {
            Self::BlockTransfer
        } else if hi_8 | 0b11111 == 0b10111111 {
            Self::Branch
        } else if hi_8 | 0b11111 == 0b11011111 {
            Self::CoprocDataTransfer
        } else if hi_8 | 0b1111 == 0b11101111 {
            if (encoding >> 4) & 1 == 0 {
                Self::CoprocOperation
            } else {
                Self::CoprocRegOperation
            }
        } else if hi_8 | 0b1111 == 0b11111111 {
            Self::SoftwareInterrupt
        } else {
            Self::Undefined
        }
    }
}
