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

    ThumbBranchPrefix,
    ThumbBranchSuffix,
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

    // Translates a Thumb encoding to an equivalent ARM encoding
    // Returns (instruction type, translated encoding)
    pub fn from_thumb_encoding(thumb_encoding: u16) -> (Self, u32) {
        // TODO: Some of these don't update flags!
        let mut allow_update_flags = true;
        // Allows for instructions without a direct translation in ARM mode
        let mut special_type_opt: Option<InstructionType> = None;

        let encoding = thumb_encoding as u32;
        let hi_n = |n: usize| (encoding >> (16 - n)) & ((1 << n) - 1);
        let translated = if hi_n(3) == 0b000 && (encoding >> 11) & 0b11 != 0b11 {
            // Shift by immediate
            let immed_5 = (encoding >> 6) & 0b11111;
            let rm = (encoding >> 3) & 0b111;
            let rd = encoding & 0b111;
            let shift_type = (encoding >> 11) & 0b11;
            // MOVS <Rd>, <Rm>, <shift_type> #<immed_5>
            (0b1110000110110000 << 16) | (rd << 12) | (immed_5 << 7) | (shift_type << 5) | rm
        } else if hi_n(6) == 0b000110 {
            // Add/subtract register
            let op = (encoding >> 9) & 1;
            let rm = (encoding >> 6) & 0b111;
            let rn = (encoding >> 3) & 0b111;
            let rd = encoding & 0b111;
            // ADDS/SUBS <Rd>, <Rn>, <Rm>
            (0b111000000001 << 20) | (1 << (23 - op)) | (rn << 16) | (rd << 12) | rm
        } else if hi_n(6) == 0b000111 {
            // Add/subtract immediate
            let op = (encoding >> 9) & 1;
            let immed_3 = (encoding >> 6) & 0b111;
            let rn = (encoding >> 3) & 0b111;
            let rd = encoding & 0b111;
            // ADDS/SUBS <Rd>, <Rn>, #<immed_3>
            (0b111000100001 << 20) | (1 << (23 - op)) | (rn << 16) | (rd << 12) | immed_3
        } else if hi_n(3) == 0b001 {
            // Add/subtract/compare/move immediate
            let reg = (encoding >> 8) & 0b111;
            let immed_8 = encoding & 0xFF;
            let (op, a, b) = match (encoding >> 11) & 0b11 {
                0b00 => (0b1101, 0, reg),       // MOV
                0b01 => (0b1010, reg, 0),       // CMP
                0b10 => (0b0100, reg, reg),     // ADD
                0b11 | _ => (0b0010, reg, reg), // SUB
            };
            // ADDS/SUBS/MOVS/CMP <Rd>|<Rn>, #<8_bit_immed>
            (0b111000100001 << 20) | (op << 21) | (a << 16) | (b << 12) | immed_8
        } else if hi_n(6) == 0b010000 {
            // Data-processing register
            let op = (encoding >> 6) & 0b1111;
            let rm_rs = (encoding >> 3) & 0b111;
            let rd_rn = encoding & 0b111;
            // The bottom 7 nybbles
            let (a, b, c, d, e, f, g) = match op {
                0b0000 => (0b0000, 0b0001, rd_rn, rd_rn, 0, 0, rm_rs), // AND
                0b0001 => (0b0000, 0b0011, rd_rn, rd_rn, 0, 0, rm_rs), // EOR
                0b0010 => (0b0001, 0b1011, 0, rd_rn, rm_rs, 1, rd_rn), // LSL
                0b0011 => (0b0001, 0b1011, 0, rd_rn, rm_rs, 3, rd_rn), // LSR
                0b0100 => (0b0001, 0b1011, 0, rd_rn, rm_rs, 5, rd_rn), // ASR
                0b0101 => (0b0000, 0b1011, rd_rn, rd_rn, 0, 0, rm_rs), // ADC
                0b0110 => (0b0000, 0b1101, rd_rn, rd_rn, 0, 0, rm_rs), // SBC
                0b0111 => (0b0001, 0b1011, 0, rd_rn, rm_rs, 7, rd_rn), // ROR
                0b1000 => (0b0001, 0b0001, rd_rn, 0, 0, 0, rm_rs),     // TST
                0b1001 => (0b0010, 0b0111, rm_rs, rd_rn, 0, 0, 0),     // NEG
                0b1010 => (0b0001, 0b0101, rd_rn, 0, 0, 0, rm_rs),     // CMP
                0b1011 => (0b0001, 0b0111, rd_rn, 0, 0, 0, rm_rs),     // CMN
                0b1100 => (0b0001, 0b1001, rd_rn, rd_rn, 0, 0, rm_rs), // ORR
                0b1101 => (0b0000, 0b0001, rd_rn, 0, rd_rn, 9, rm_rs), // MUL
                0b1110 => (0b0001, 0b1101, rd_rn, rd_rn, 0, 0, rm_rs), // BIC
                0b1111 | _ => (0b0001, 0b1111, 0, rd_rn, 0, 0, rm_rs), // MVN
            };
            (0b1110 << 28) | (a << 24) | (b << 20) | (c << 16) | (d << 12) | (e << 8) | (f << 4) | g
        } else if hi_n(6) == 0b010001 && (encoding >> 8) & 0b11 != 0b11 {
            // Special data processing
            let op = (encoding >> 8) & 0b11;
            let rm = (encoding >> 3) & 0b1111; // (H2 << 3) | Rm
            let rd_rn = (((encoding >> 7) & 1) << 3) | encoding & 0b111; // (H1 << 3) | (Rd or Rn)
            match op {
                0b00 => (0b111000001000 << 20) | (rd_rn << 16) | (rd_rn << 12) | rm,
                0b01 => (0b111000001001 << 20) | (rd_rn << 16) | rm,
                0b10 | _ => (0b111000011010 << 20) | (rd_rn << 12) | rm,
            }
        } else if hi_n(8) == 0b01000111 {
            // Branch/exchange instruction set
            let link = (encoding >> 7) & 1;
            let reg = (encoding >> 3) & 0b1111;
            // TODO: BX behaves differently when with reg=15
            (0b1110000100101111111111110001 << 4) | (link << 5) | reg
        } else if hi_n(5) == 0b01001 {
            // Load from literal pool
            let immed_8 = encoding & 0xFF;
            let reg = (encoding >> 8) & 0b111;
            // LDR <Rd>, [PC, #<immed_8> * 4]
            (0b1110010110011111 << 16) | (reg << 12) | (immed_8 << 2)
        } else if hi_n(4) == 0b0101 {
            // Load/store register offset
            let op = (encoding >> 9) & 0b111;
            let rm = (encoding >> 6) & 0b111;
            let rn = (encoding >> 3) & 0b111;
            let rd = encoding & 0b111;
            let (hi_8, lo_4) = match op {
                0b000 => (0b01111000, 0b0000),     // STR
                0b001 => (0b00011000, 0b1011),     // STRH
                0b010 => (0b01111100, 0b0000),     // STRB
                0b011 => (0b00011001, 0b1101),     // LDRSB
                0b100 => (0b01111001, 0b0000),     // LDR
                0b101 => (0b00011001, 0b1011),     // LDRH
                0b110 => (0b01111101, 0b0000),     // LDRB
                0b111 | _ => (0b00011001, 0b1111), // LDRSH
            };
            (0b1110 << 28) | (hi_8 << 20) | (rn << 16) | (rd << 12) | (lo_4 << 4) | rm
        } else if hi_n(3) == 0b011 {
            // Load/store word/byte immediate offset
            let byte = (encoding >> 12) & 1;
            let load = (encoding >> 11) & 1;
            let offset = (encoding >> 6) & 0b11111;
            let rn = (encoding >> 3) & 0b111;
            let rd = encoding & 0b111;
            (0b111001011000 << 20) | (load << 20) | (byte << 22) | (rn << 16) | (rd << 12) | offset
        } else if hi_n(4) == 0b1000 {
            // Load/store halfword immediate offset
            let load = (encoding >> 11) & 1;
            let immed_hi = (encoding >> 9) & 0b11;
            let immed_lo = (encoding >> 6) & 0b111;
            let rn = (encoding >> 3) & 0b111;
            let rd = encoding & 0b111;
            (0b111000011100 << 20)
                | (load << 20)
                | (rn << 16)
                | (rd << 12)
                | (immed_hi << 8)
                | (0b1011 << 4)
                | (immed_lo << 1)
        } else if hi_n(4) == 0b1001 {
            // Load/store to/from stack
            let load = (encoding >> 11) & 1;
            let rd = (encoding >> 8) & 0b111;
            let immed_8 = encoding & 0xFF;
            (0b1110010110001101 << 16) | (load << 20) | (rd << 12) | (immed_8 << 2)
        } else if hi_n(4) == 0b1010 {
            // Add to SP or PC
            let pc_flag = 1 - ((encoding >> 11) & 1);
            let rd = (encoding >> 8) & 0b111;
            let immed_8 = encoding & 0xFF;
            (0b1110001010001101 << 16) | (pc_flag << 17) | (rd << 12) | (0b1111 << 8) | immed_8
        } else if hi_n(4) == 0b1011 {
            // Miscellaneous
            let instr_type = (encoding >> 9) & 0b11;
            if instr_type == 0b00 {
                // Adjust stack pointer
                let op = (encoding >> 7) & 1;
                let immed_7 = encoding & 0b1111111;
                (0b1110001000001101110111110 << 7) | (1 << (23 - op)) | immed_7
            } else if instr_type == 0b10 {
                // Push/pop register list
                let pop_flag = (encoding >> 11) & 1;
                let lr_flag = (encoding >> 8) & 1;
                let reg_list = encoding & 0xFF;
                let hi_8 = if pop_flag == 0 {
                    0b10001011
                } else {
                    0b10010010
                };
                (0b1110 << 28)
                    | (hi_8 << 20)
                    | (0b1101 << 16)
                    | (lr_flag << (14 + pop_flag))
                    | reg_list
            } else {
                // Undefined
                (0b1110001 << 25) | (1 << 4)
            }
        } else if hi_n(4) == 0b1100 {
            // Load/store multiple
            let load_flag = (encoding >> 11) & 1;
            let rn = (encoding >> 8) & 0b111;
            let reg_list = encoding & 0xFF;
            // Don't write back if doing a load with rn in the register list
            let write_back = (load_flag == 0 || ((reg_list >> rn) == 0)) as u32;
            (0b111010001000 << 20) | (write_back << 21) | (load_flag << 20) | (rn << 16) | reg_list
        } else if hi_n(4) == 0b1101 && (encoding >> 9) & 0b111 != 0b111 {
            // Conditional branch
            let cond = (encoding >> 8) & 0b1111;
            let immed_8 = encoding & 0xFF;
            let sign_ext = if (immed_8 >> 7) & 1 == 1 { 0xFFFF } else { 0 };
            (cond << 28) | (0b1010 << 24) | (sign_ext << 8) | immed_8
        } else if hi_n(8) == 0b11011110 {
            // Undefined instruction
            (0b1110001 << 25) | (1 << 4)
        } else if hi_n(8) == 0b11011111 {
            // Software interrupt
            let immed_8 = encoding & 0xFF;
            (0b11101111 << 24) | immed_8
        } else if (hi_n(5) | 0b11) == 0b11111 {
            match (encoding >> 11) & 0b11 {
                0b00 => {
                    // Unconditional branch
                    let immed_11 = encoding & 0b11111111111;
                    let sign_ext = if (immed_11 >> 10) & 1 == 1 {
                        0b1111111111111
                    } else {
                        0
                    };
                    (0b11101010 << 24) | (sign_ext << 11) | immed_11
                }
                0b01 => {
                    // Undefined
                    (0b1110001 << 25) | (1 << 4)
                }
                0b10 => {
                    // BL prefix
                    special_type_opt = Some(InstructionType::ThumbBranchPrefix);
                    thumb_encoding as u32
                }
                0b11 | _ => {
                    // BL suffix
                    special_type_opt = Some(InstructionType::ThumbBranchSuffix);
                    thumb_encoding as u32
                }
            }
        } else {
            // Undefined instruction
            (0b1110001 << 25) | (1 << 4)
        };

        let instr_type = if let Some(special_type) = special_type_opt {
            special_type
        } else {
            InstructionType::from_encoding(translated)
        };

        (instr_type, translated)
    }
}
