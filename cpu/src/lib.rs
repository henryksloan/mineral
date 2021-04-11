mod condition;
mod instruction_type;
mod operating_mode;
mod status_register;

use crate::{
    condition::Condition, instruction_type::InstructionType, operating_mode::OperatingMode,
    status_register::StatusRegister,
};
use memory::{Memory, RAM, ROM};

// https://developer.arm.com/documentation/ddi0210/c/Programmer-s-Model/Exceptions/Exception-vectors
const RESET_VEC: u32 = 0x00000000;
const UND_VEC: u32 = 0x00000004;
const SWI_VEC: u32 = 0x00000008;
const PREFETCH_ABT_VEC: u32 = 0x0000000C;
const DATA_ABT_VEC: u32 = 0x00000010;
const RESERVED_VEC: u32 = 0x00000014;
const IRQ_VEC: u32 = 0x00000018;
const FIQ_VEC: u32 = 0x0000001C;

pub struct CPU {
    // The CPSR (current program status register),
    // and a SPSR (saved program status register) for each interrupt mode
    cpsr: StatusRegister,
    fiq_spsr: StatusRegister,
    svc_spsr: StatusRegister,
    abt_spsr: StatusRegister,
    irq_spsr: StatusRegister,
    und_spsr: StatusRegister,

    // Each mode has r13 and r14 banked, allowing for private SP and LR
    // FIQ mode additionally has r8-r12 banked
    pub registers: [u32; 16],
    fiq_register_bank: [u32; 7],
    svc_register_bank: [u32; 2],
    abt_register_bank: [u32; 2],
    irq_register_bank: [u32; 2],
    und_register_bank: [u32; 2],

    pub bios_rom: ROM<0x4000>,
    ewram: RAM<0x40000>, // Internal work ram
    iwram: RAM<0x8000>,  // External work ram
}

impl CPU {
    pub fn new() -> Self {
        Self {
            cpsr: StatusRegister::new(),
            fiq_spsr: StatusRegister::new(),
            svc_spsr: StatusRegister::new(),
            abt_spsr: StatusRegister::new(),
            irq_spsr: StatusRegister::new(),
            und_spsr: StatusRegister::new(),

            // Each mode has r13 and r14 banked, allowing for private SP and LR
            // FIQ mode additionally has r8-r12 banked
            registers: [0; 16],
            fiq_register_bank: [0; 7],
            svc_register_bank: [0; 2],
            abt_register_bank: [0; 2],
            irq_register_bank: [0; 2],
            und_register_bank: [0; 2],

            bios_rom: ROM::new(),
            ewram: RAM::new(), // Internal work ram
            iwram: RAM::new(), // External work ram
        }
    }

    pub fn tick(&mut self) {
        // TODO: Implement 3-stage pipeline
        // In ARM mode, the bottom two bytes of the PC aren't used, so PC selects a word
        // TODO: In Thumb mode, only the bottom bit is unused
        let pc = self.get_register(15) & !0b11;
        let encoding = self.read_u32(pc as usize);
        let instr_type = InstructionType::from_encoding(encoding);
        let condition = Condition::from_u8(((encoding >> 28) & 0xF) as u8);
        println!(
            "{:8X}: {:8X} {:?} {:?} {:b}",
            pc, encoding, instr_type, condition, self.cpsr.raw
        );

        if self.eval_condition(condition) {
            match instr_type {
                InstructionType::Multiply | InstructionType::MultiplyLong => {
                    self.multiply_instr(encoding)
                }
                InstructionType::BranchExchange => self.branch_exchange(encoding),
                InstructionType::SingleSwap => self.single_swap_instr(encoding),
                InstructionType::HalfwordTransferReg | InstructionType::HalfwordTransferImm => {
                    self.halfword_transfer_instr(encoding)
                }
                InstructionType::SingleTransfer => self.single_transfer_instr(encoding),
                InstructionType::DataProcessing => self.data_proc_instr(encoding),
                InstructionType::Undefined => self.undefined_interrupt(),
                InstructionType::BlockTransfer => self.block_transfer(encoding),
                InstructionType::Branch => self.branch_instr(encoding),
                InstructionType::CoprocDataTransfer => {}
                InstructionType::CoprocOperation => {}
                InstructionType::CoprocRegOperation => {}
                InstructionType::SoftwareInterrupt => self.software_interrupt(),
            }
        }

        // TODO: I think it might be better to increment this before execution
        // I think that's how it would work with a pipeline, anyway
        // If changed, remember to modify the destination of branch instr's
        self.set_register(15, self.get_register(15).wrapping_add(4));
    }

    fn eval_condition(&self, condition: Condition) -> bool {
        match condition {
            Condition::EQ => self.cpsr.get_z(),
            Condition::NE => !self.cpsr.get_z(),
            Condition::CS => self.cpsr.get_c(),
            Condition::CC => !self.cpsr.get_c(),
            Condition::MI => self.cpsr.get_n(),
            Condition::PL => !self.cpsr.get_n(),
            Condition::VS => self.cpsr.get_v(),
            Condition::VC => !self.cpsr.get_v(),
            Condition::HI => self.cpsr.get_c() && !self.cpsr.get_z(),
            Condition::LS => !self.cpsr.get_c() || self.cpsr.get_z(),
            Condition::GE => self.cpsr.get_n() == self.cpsr.get_v(),
            Condition::LT => self.cpsr.get_n() != self.cpsr.get_v(),
            Condition::GT => !self.cpsr.get_z() && (self.cpsr.get_n() == self.cpsr.get_v()),
            Condition::LE => self.cpsr.get_z() && (self.cpsr.get_n() != self.cpsr.get_v()),
            Condition::AL => true,
            Condition::NV => false,
        }
    }

    fn get_register(&self, n: usize) -> u32 {
        let mode = self.cpsr.get_mode();
        if n == 13 || n == 14 {
            match mode {
                OperatingMode::User | OperatingMode::System => self.registers[n],
                OperatingMode::FastInterrupt => self.fiq_register_bank[n - 8],
                OperatingMode::Interrupt => self.irq_register_bank[n - 13],
                OperatingMode::Supervisor => self.svc_register_bank[n - 13],
                OperatingMode::Abort => self.abt_register_bank[n - 13],
                OperatingMode::Undefined => self.und_register_bank[n - 13],
            }
        } else if mode == OperatingMode::FastInterrupt && n >= 8 && n <= 14 {
            self.fiq_register_bank[n - 8]
        } else {
            self.registers[n]
        }
    }

    fn set_register(&mut self, n: usize, val: u32) {
        let mode = self.cpsr.get_mode();
        if n == 13 || n == 14 {
            match mode {
                OperatingMode::User | OperatingMode::System => self.registers[n] = val,
                OperatingMode::FastInterrupt => self.fiq_register_bank[n - 8] = val,
                OperatingMode::Interrupt => self.irq_register_bank[n - 13] = val,
                OperatingMode::Supervisor => self.svc_register_bank[n - 13] = val,
                OperatingMode::Abort => self.abt_register_bank[n - 13] = val,
                OperatingMode::Undefined => self.und_register_bank[n - 13] = val,
            }
        } else if mode == OperatingMode::FastInterrupt && n >= 8 && n <= 14 {
            self.fiq_register_bank[n - 8] = val
        } else {
            self.registers[n] = val
        }
    }

    fn get_mode_spsr(&mut self) -> Option<&mut StatusRegister> {
        match self.cpsr.get_mode() {
            OperatingMode::FastInterrupt => Some(&mut self.fiq_spsr),
            OperatingMode::Supervisor => Some(&mut self.svc_spsr),
            OperatingMode::Abort => Some(&mut self.abt_spsr),
            OperatingMode::Interrupt => Some(&mut self.irq_spsr),
            OperatingMode::Undefined => Some(&mut self.und_spsr),
            _ => None,
        }
    }

    fn multiply_instr(&mut self, encoding: u32) {
        let long_flag = (encoding >> 23) & 1 == 1; // Output to two registers, allowing 64 bits
        let unsigned_flag = (encoding >> 22) & 1 == 1; // Only used for long multiplies
        let accumulate_flag = (encoding >> 21) & 1 == 1; // Allows a value to be added to the product
        let set_cond_flag = (encoding >> 20) & 1 == 1; // Updates zero and negative CPSR flags

        let op1 = self.get_register(((encoding >> 8) & 0b1111) as usize);
        let op2 = self.get_register((encoding & 0b1111) as usize);
        let product = if long_flag && !unsigned_flag {
            (op1 as i64 * op2 as i64) as u64
        } else {
            op1 as u64 * op2 as u64
        };

        let other_reg_hi_n = ((encoding >> 16) & 0b1111) as usize;
        let other_reg_lo_n = ((encoding >> 12) & 0b1111) as usize;

        let addend = if long_flag {
            ((self.get_register(other_reg_hi_n) as u64) << 32)
                | self.get_register(other_reg_lo_n) as u64
        } else {
            self.get_register(other_reg_lo_n as usize) as u64
        };

        // Write results and optionally set condition flags
        let result = product + if accumulate_flag { addend } else { 0 };
        if long_flag {
            self.set_register(other_reg_lo_n, (result & 0xFFFFFFFF) as u32);
            self.set_register(other_reg_hi_n, ((result >> 32) & 0xFFFFFFFF) as u32);
        } else {
            self.set_register(other_reg_hi_n, (result & 0xFFFFFFFF) as u32);
        }

        if set_cond_flag {
            self.cpsr.set_z(result == 0);
            let sign_bit_offset = if long_flag { 63 } else { 31 };
            self.cpsr.set_n((result >> sign_bit_offset) & 1 == 1);
        }
    }

    fn branch_exchange(&mut self, encoding: u32) {
        let val = self.get_register((encoding & 0b1111) as usize);
        self.set_register(15, val);
        self.cpsr.set_t(val & 1 == 1); // Set Thumb bit based on LSB
    }

    fn single_swap_instr(&mut self, encoding: u32) {
        let byte_flag = (encoding >> 22) & 1 == 1;

        let base_reg_n = ((encoding >> 16) & 0b1111) as usize;
        let swap_addr = self.get_register(base_reg_n) as usize;

        let dest_reg_n = ((encoding >> 12) & 0b1111) as usize;
        let source_reg_n = (encoding & 0b1111) as usize;
        let source_reg = self.get_register(source_reg_n);

        if byte_flag {
            let old_data = self.read(swap_addr) as u32;
            self.write(swap_addr, (source_reg & 0xFF) as u8);
            self.set_register(dest_reg_n, old_data);
        } else {
            let old_data = self.read_u32(swap_addr);
            self.write_u32(swap_addr, source_reg);
            self.set_register(dest_reg_n, old_data);
        };
    }

    fn halfword_transfer_instr(&mut self, encoding: u32) {
        let pre_index_flag = (encoding >> 24) & 1 == 1;
        let up_flag = (encoding >> 23) & 1 == 1;
        let write_back_flag = (encoding >> 21) & 1 == 1;
        let load_flag = (encoding >> 20) & 1 == 1;

        let base_reg_n = ((encoding >> 16) & 0b1111) as usize;
        let base_reg = self
            .get_register(base_reg_n)
            .wrapping_add(if base_reg_n == 15 { 8 } else { 0 });
        let source_dest_reg_n = ((encoding >> 12) & 0b1111) as usize;

        let offset = if (encoding >> 22) & 1 == 1 {
            ((encoding & 0xF00) >> 4) | (encoding & 0xF)
        } else {
            self.get_register((encoding & 0b1111) as usize)
        };

        let offset_addr = if up_flag {
            base_reg.wrapping_add(offset)
        } else {
            base_reg.wrapping_sub(offset)
        };

        let transfer_addr = if pre_index_flag {
            offset_addr
        } else {
            base_reg
        } as usize;

        // TODO: Handle endianness
        // TODO: Handle special LDR behavior on non-word-aligned addresses
        if load_flag {
            let data = match (encoding >> 5) & 0b11 {
                0b01 => self.read_u16(transfer_addr) as u32, // Unsigned halfword
                0b10 => ((self.read(transfer_addr) as i8) as i32) as u32, // Signed byte
                0b11 => ((self.read_u16(transfer_addr) as i16) as i32) as u32, // Signed halfword
                0b00 | _ => panic!("SWP format encountered in halfword transfer instruction"),
            };
            self.set_register(source_dest_reg_n, data);
        } else {
            let data = self.get_register(source_dest_reg_n);
            match (encoding >> 5) & 0b11 {
                0b01 => self.write_u16(transfer_addr, (data & 0xFFFF) as u16), // Unsigned halfword
                0b10 | 0b11 => panic!("signed transfers used with store instructions"),
                0b00 | _ => panic!("SWP format encountered in halfword transfer instruction"),
            }
        }

        // Post-indexing always writes back
        // TODO: https://iitd-plos.github.io/col718/ref/arm-instructionset.pdf Page 4-27
        // says "the W bit forces non-privileged mode for the transfer"
        if write_back_flag || !pre_index_flag {
            self.set_register(base_reg_n, offset_addr);
        }
    }

    fn single_transfer_instr(&mut self, encoding: u32) {
        let reg_offset_flag = (encoding >> 25) & 1 == 1;
        let pre_index_flag = (encoding >> 24) & 1 == 1;
        let up_flag = (encoding >> 23) & 1 == 1;
        let byte_flag = (encoding >> 22) & 1 == 1;
        let write_back_flag = (encoding >> 21) & 1 == 1;
        let load_flag = (encoding >> 20) & 1 == 1;

        let base_reg_n = ((encoding >> 16) & 0b1111) as usize;
        let base_reg = self
            .get_register(base_reg_n)
            .wrapping_add(if base_reg_n == 15 { 8 } else { 0 });
        let source_dest_reg_n = ((encoding >> 12) & 0b1111) as usize;

        let offset = if reg_offset_flag {
            self.shifted_reg_operand(encoding & 0xFFF, false).0
        } else {
            encoding & 0xFFF
        };

        let offset_addr = if up_flag {
            base_reg.wrapping_add(offset)
        } else {
            base_reg.wrapping_sub(offset)
        };

        let transfer_addr = if pre_index_flag {
            offset_addr
        } else {
            base_reg
        } as usize;

        // TODO: Handle endianness
        // TODO: Handle special LDR behavior on non-word-aligned addresses
        if load_flag {
            let data = if byte_flag {
                self.read(transfer_addr) as u32
            } else {
                self.read_u32(transfer_addr)
            };
            self.set_register(source_dest_reg_n, data);
        } else {
            if byte_flag {
                let data = (self.get_register(source_dest_reg_n) & 0xFF) as u8;
                self.write(transfer_addr, data);
            } else {
                self.write_u32(transfer_addr, self.get_register(source_dest_reg_n));
            }
        }

        // Post-indexing always writes back
        // TODO: https://iitd-plos.github.io/col718/ref/arm-instructionset.pdf Page 4-27
        // says "the W bit forces non-privileged mode for the transfer"
        if write_back_flag || !pre_index_flag {
            self.set_register(base_reg_n, offset_addr);
        }
    }

    fn data_proc_instr(&mut self, encoding: u32) {
        let imm_flag = (encoding >> 25) & 1 == 1;
        let set_cond_flag = (encoding >> 20) & 1 == 1;
        let opcode = (encoding >> 21) & 0b1111;

        let dest_reg_n = ((encoding >> 12) & 0b1111) as usize;

        // PSR instructions are special cases of this encoding
        if !set_cond_flag {
            let use_spsr_flag = (encoding >> 22) & 1 == 1;
            if (opcode | 0b0010) == 0b1010 && !imm_flag {
                self.move_psr_into_reg(use_spsr_flag, dest_reg_n);
                return;
            } else if (opcode | 0b0010) == 0b1011 {
                self.move_into_psr(use_spsr_flag, imm_flag, encoding);
                return;
            }
        };

        let op1_reg_n = ((encoding >> 16) & 0b1111) as usize;
        let op1_reg = self
            .get_register(op1_reg_n)
            .wrapping_add(if op1_reg_n == 15 {
                // If the PC is used as an operand, prefetching causes it to be higher
                // by an amount depending on whether the shift is specified directly or by a register
                if imm_flag {
                    8
                } else {
                    12
                }
            } else {
                0
            });

        // http://vision.gel.ulaval.ca/~jflalonde/cours/1001/h17/docs/arm-instructionset.pdf pages 4-12 through 4-15
        // TODO: PC is supposed to produce lots of special cases
        let (op2, shifter_carry) = if imm_flag {
            self.rotated_imm_operand(encoding & 0xFFF)
        } else {
            self.shifted_reg_operand(encoding & 0xFFF, true)
        };

        let check_overflow = |result: u32, write_result: bool| {
            (
                result,
                Some(Self::did_overflow(op1_reg, op2, result)),
                write_result,
            )
        };

        let carry = self.cpsr.get_c() as u32;
        let (result, overflow, write_result) = match opcode {
            0b0000 => (op1_reg & op2, None, true), // AND
            0b0001 => (op1_reg ^ op2, None, true), // EOR
            0b0010 => check_overflow(op1_reg.wrapping_sub(op2), true), // SUB
            0b0011 => check_overflow(op2.wrapping_sub(op1_reg), true), // RSB
            0b0100 => check_overflow(op1_reg.wrapping_add(op2), true), // ADD
            0b0101 => check_overflow(op1_reg.wrapping_add(op2).wrapping_add(carry), true), // ADC
            0b0110 => check_overflow(
                op1_reg
                    .wrapping_sub(op2)
                    .wrapping_add(carry)
                    .wrapping_sub(1),
                true,
            ), // SBC
            0b0111 => check_overflow(
                op2.wrapping_sub(op1_reg)
                    .wrapping_add(carry)
                    .wrapping_sub(1),
                true,
            ), // RSC
            0b1000 => (op1_reg & op2, None, false), // TST
            0b1001 => (op1_reg ^ op2, None, false), // TEQ
            0b1010 => check_overflow(op1_reg.wrapping_sub(op2), false), // CMP
            0b1011 => check_overflow(op1_reg.wrapping_add(op2), false), // CMN
            0b1100 => (op1_reg | op2, None, true), // OOR
            0b1101 => (op2, None, true),           // MOV
            0b1110 => (op1_reg & !op2, None, true), // BIC
            0b1111 | _ => (!op2, None, true),      // MVN
        };

        if write_result {
            self.set_register(dest_reg_n, result);
        }

        if set_cond_flag {
            if dest_reg_n == 15 {
                // TODO: should this update thumb state?
                // https://www.cs.rit.edu/~tjh8300/CowBite/CowBiteSpec.htm:
                // "Executing any arithmetic instruction with the PC as the target
                // and the 'S' bit of the instruction set, with bit 0 of the new PC being 1."
                if let Some(spsr) = self.get_mode_spsr() {
                    self.cpsr.raw = spsr.raw;
                } else {
                    panic!("attempted to copy from SPSR in User or System mode");
                }
            } else {
                if let Some(new_overflow) = overflow {
                    self.cpsr.set_v(new_overflow)
                }
                self.cpsr.set_c(shifter_carry);
                self.cpsr.set_z(result == 0);
                self.cpsr.set_n((result >> 31) & 1 == 1);
            }
        }
    }

    fn move_psr_into_reg(&mut self, use_spsr_flag: bool, dest_reg_n: usize) {
        let val = if use_spsr_flag {
            self.get_mode_spsr()
                .expect("attempted to get SPSR in non-privileged mode")
                .raw
        } else {
            self.cpsr.raw
        };
        self.set_register(dest_reg_n, val);
    }

    fn move_into_psr(&mut self, use_spsr_flag: bool, imm_flag: bool, encoding: u32) {
        let val = if imm_flag {
            self.rotated_imm_operand(encoding & 0xFFF).0
        } else {
            self.get_register((encoding & 0b1111) as usize)
        };

        // Sets whether certain parts of the PSR will be modified
        let control_mask = (encoding >> 16) & 1 == 1; // PSR[7:0]
        let extension_mask = (encoding >> 17) & 1 == 1; // PSR[15:8]
        let status_mask = (encoding >> 18) & 1 == 1; // PSR[23:16]
        let flags_mask = (encoding >> 19) & 1 == 1; // PSR[31:24]
        let mask = if control_mask { 0xFF } else { 0 }
            | if extension_mask { 0xFF << 8 } else { 0 }
            | if status_mask { 0xFF << 16 } else { 0 }
            | if flags_mask { 0xFF << 24 } else { 0 };

        if use_spsr_flag {
            let spsr = self
                .get_mode_spsr()
                .expect("attempted to get SPSR in non-privileged mode");
            (*spsr).raw &= mask;
            (*spsr).raw |= val & mask;
        }
    }

    fn block_transfer(&mut self, encoding: u32) {
        let pre_index_flag = (encoding >> 24) & 1 == 1;
        let up_flag = (encoding >> 23) & 1 == 1;
        let psr_force_user_flag = (encoding >> 22) & 1 == 1;
        let write_back_flag = (encoding >> 21) & 1 == 1;
        let load_flag = (encoding >> 20) & 1 == 1;

        let reg_n_list = (0..16)
            .filter(|i| (encoding >> i) & 1 == 1)
            .collect::<Vec<usize>>();
        let pc_in_list = (encoding >> 15) & 1 == 1;

        let base_reg_n = ((encoding >> 16) & 0b1111) as usize;
        let base_reg = self.get_register(base_reg_n);
        let mut transfer_addr = if up_flag {
            base_reg.wrapping_add(if pre_index_flag { 4 } else { 0 })
        } else {
            base_reg
                .wrapping_sub(4 * reg_n_list.len() as u32)
                .wrapping_add(if !pre_index_flag { 4 } else { 0 })
        };

        for reg_n in &reg_n_list {
            if load_flag {
                let data = self.read_u32(transfer_addr as usize);
                // If S flag is set and r15 is not in the list, the user bank is used
                if psr_force_user_flag && !pc_in_list {
                    self.registers[*reg_n] = data;
                } else {
                    self.set_register(*reg_n, data);
                }

                // If S flag is set and r15 is in the list, the SPSR is restored
                // at the same time r15 is loaded
                if psr_force_user_flag && *reg_n == 15 {
                    self.cpsr.raw = self
                        .get_mode_spsr()
                        .expect("attempted to get SPSR in non-privileged mode")
                        .raw;
                }
            } else {
                // If S flag is set and r15 is not in the list, the user bank is used
                if psr_force_user_flag {
                    self.write_u32(transfer_addr as usize, self.registers[*reg_n])
                } else {
                    self.write_u32(transfer_addr as usize, self.get_register(*reg_n))
                }
            }

            transfer_addr += 4;
        }

        if write_back_flag {
            let offset_addr = if up_flag {
                base_reg.wrapping_add(4 * reg_n_list.len() as u32)
            } else {
                base_reg.wrapping_sub(4 * reg_n_list.len() as u32)
            };
            self.set_register(base_reg_n, offset_addr)
        }
    }

    fn branch_instr(&mut self, encoding: u32) {
        let link_flag = (encoding >> 24) & 1 == 1;
        if link_flag {
            self.set_register(14, self.get_register(15));
        }

        let mut offset = (encoding & 0xFFFFFF) << 2; // 24 bits, shifted left
        if (offset >> 23) & 1 == 1 {
            offset |= 0xFF_000000; // Sign extend
        }
        self.set_register(
            15,
            self.get_register(15).wrapping_add(offset).wrapping_add(4),
        );
    }

    fn software_interrupt(&mut self) {
        self.svc_register_bank[1] = self.get_register(15).wrapping_add(4);
        self.svc_spsr.raw = self.cpsr.raw;
        self.cpsr.set_mode(OperatingMode::System);
        self.cpsr.set_t(false);
        self.cpsr.set_i(true);
        self.set_register(15, SWI_VEC);
    }

    fn undefined_interrupt(&mut self) {
        self.und_register_bank[1] = self.get_register(15).wrapping_add(4);
        self.und_spsr.raw = self.cpsr.raw;
        self.cpsr.set_mode(OperatingMode::Undefined);
        self.cpsr.set_t(false);
        self.cpsr.set_i(true);
        self.set_register(15, UND_VEC);
    }

    // Decodes a 12-bit operand to a register shifted by an immediate- or register-defined value
    // Returns (shifted result, barrel shifter carry out)
    fn shifted_reg_operand(&self, operand: u32, allow_shift_by_reg: bool) -> (u32, bool) {
        let op2_reg_n = (operand & 0b1111) as usize;
        let op2_reg = self.get_register(op2_reg_n) + if op2_reg_n == 15 { 8 } else { 0 };

        let shift_by_reg = (operand >> 4) & 1 == 1;
        let shift_amount = if allow_shift_by_reg && shift_by_reg {
            self.get_register(((operand >> 8) & 0b1111) as usize)
        } else {
            (operand >> 7) & 0b11111
        };

        if shift_by_reg && shift_amount == 0 {
            (op2_reg, self.cpsr.get_c())
        } else {
            match (operand >> 5) & 0b11 {
                0b00 => {
                    // LSL
                    if shift_amount == 32 {
                        (0, op2_reg & 1 == 1)
                    } else if shift_amount > 32 {
                        (0, false)
                    } else {
                        let shifter_carry = if shift_amount == 0 {
                            self.cpsr.get_c()
                        } else {
                            (op2_reg >> (32 - shift_amount)) & 1 == 1
                        };
                        (op2_reg << shift_amount, shifter_carry)
                    }
                }
                0b01 => {
                    // LSR
                    if shift_amount == 32 || shift_amount == 0 {
                        (0, (op2_reg >> 31) & 1 == 1)
                    } else if shift_amount > 32 {
                        (0, false)
                    } else {
                        let shifter_carry = (op2_reg >> (shift_amount - 1)) & 1 == 1;
                        (op2_reg >> shift_amount, shifter_carry)
                    }
                }
                0b10 => {
                    // ASR
                    if shift_amount >= 32 || shift_amount == 0 {
                        if (op2_reg >> 31) & 1 == 1 {
                            (0xFFFFFFFF, true)
                        } else {
                            (0, false)
                        }
                    } else {
                        let shifter_carry = (op2_reg >> (shift_amount - 1)) & 1 == 1;
                        (((op2_reg as i32) >> shift_amount) as u32, shifter_carry)
                    }
                }
                0b11 | _ => {
                    // ROR
                    if shift_amount == 32 {
                        (op2_reg, (op2_reg >> 31) & 1 == 1)
                    } else {
                        let new_shift_amount = shift_amount % 32;
                        if new_shift_amount == 0 {
                            (
                                (op2_reg >> 1) | ((self.cpsr.get_c() as u32) << 31),
                                (op2_reg & 1) == 1,
                            )
                        } else {
                            let shifter_carry = (op2_reg >> (shift_amount - 1)) & 1 == 1;
                            (op2_reg.rotate_right(shift_amount), shifter_carry)
                        }
                    }
                }
            }
        }
    }

    // Decodes a 12-bit operand to an immediate rotated by a 4-bit unsigned immediate
    // Returns (shifted result, barrel shifter carry out)
    fn rotated_imm_operand(&self, operand: u32) -> (u32, bool) {
        let rotate = ((operand >> 8) & 0b1111) * 2;
        let imm = operand & 0xFF;
        let shifter_operand = imm.rotate_right(rotate);
        let shifter_carry = if rotate == 0 {
            self.cpsr.get_c()
        } else {
            (shifter_operand >> 31) & 1 == 1
        };
        (shifter_operand, shifter_carry)
    }

    // Checks whether an add or subtract has resulted in overflow
    fn did_overflow(op1: u32, op2: u32, result: u32) -> bool {
        let op1_sign = (op1 >> 31) & 1 == 1;
        let op2_sign = (op2 >> 31) & 1 == 1;
        let result_sign = (result >> 31) & 1 == 1;
        (op1_sign == op2_sign) && (op1_sign != result_sign)
    }
}

// TODO: Most memory-mapped registers seem to be 16- or 32-bit
// Should they only be readable through reads of exactly that width?
// It would make sense if reading a 32-bit address that contained two registers read both
// But reading a byte within a 16-bit register wouldn't happen physically (?)
impl Memory for CPU {
    // TODO: When do reads have side-effects?
    // kevtris says open bus, link port reg's, RX errors, joybus RX
    fn peek(&self, addr: usize) -> u8 {
        match addr {
            0x00000000..=0x00003FFF => self.bios_rom.peek(addr),
            0x02000000..=0x0203FFFF => self.ewram.peek(addr - 0x02000000),
            0x03000000..=0x0307FFFF => self.iwram.peek(addr - 0x03000000),
            0x04000000..=0x040003FE => {
                0 // TODO
            }
            _ => 0, // TODO: What to do here?
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
        match addr {
            0x02000000..=0x0203FFFF => self.ewram.write(addr - 0x02000000, data),
            0x03000000..=0x0307FFFF => self.iwram.write(addr - 0x03000000, data),
            0x04000000..=0x040003FE => {
                // TODO
            }
            _ => {} // TODO: What to do here?
        }
    }
}
