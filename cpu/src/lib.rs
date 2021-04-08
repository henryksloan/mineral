mod condition;
mod instruction_type;
mod operating_mode;
mod status_register;

use crate::{
    condition::Condition, instruction_type::InstructionType, operating_mode::OperatingMode,
    status_register::StatusRegister,
};
use memory::{Memory, RAM, ROM};

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
            "{:8x}: {:8X} {:?} {:?} {:b}",
            pc, encoding, instr_type, condition, self.cpsr.raw
        );

        if self.eval_condition(condition) {
            match instr_type {
                InstructionType::MultiplyAccumulate => {}
                InstructionType::MultiplyAccumulateLong => {}
                InstructionType::BranchExchange => self.branch_exchange(encoding),
                InstructionType::SingleSwap => {}
                InstructionType::HalfwordTransferReg => {}
                InstructionType::HalfwordTransferImm => {}
                InstructionType::SignedTransfer => {}
                InstructionType::DataProcessing => self.data_proc_instr(encoding),
                InstructionType::LoadStore => {}
                InstructionType::Undefined => println!("Undefined instruction {:8X}", encoding),
                InstructionType::BlockTransfer => {}
                InstructionType::Branch => self.branch_instr(encoding),
                InstructionType::CoprocDataTransfer => {}
                InstructionType::CoprocOperation => {}
                InstructionType::CoprocRegOperation => {}
                InstructionType::SoftwareInterrupt => {}
            }
        }

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
                OperatingMode::FastInterrupt => self.fiq_register_bank[n],
                OperatingMode::Interrupt => self.irq_register_bank[n],
                OperatingMode::Supervisor => self.svc_register_bank[n],
                OperatingMode::Abort => self.abt_register_bank[n],
                OperatingMode::Undefined => self.und_register_bank[n],
            }
        } else if mode == OperatingMode::FastInterrupt && n >= 8 && n <= 14 {
            self.fiq_register_bank[n]
        } else {
            self.registers[n]
        }
    }

    fn set_register(&mut self, n: usize, val: u32) {
        let mode = self.cpsr.get_mode();
        if n == 13 || n == 14 {
            match mode {
                OperatingMode::User | OperatingMode::System => self.registers[n] = val,
                OperatingMode::FastInterrupt => self.fiq_register_bank[n] = val,
                OperatingMode::Interrupt => self.irq_register_bank[n] = val,
                OperatingMode::Supervisor => self.svc_register_bank[n] = val,
                OperatingMode::Abort => self.abt_register_bank[n] = val,
                OperatingMode::Undefined => self.und_register_bank[n] = val,
            }
        } else if mode == OperatingMode::FastInterrupt && n >= 8 && n <= 14 {
            self.fiq_register_bank[n] = val
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

    fn branch_exchange(&mut self, encoding: u32) {
        let val = self.get_register((encoding & 0b1111) as usize);
        self.set_register(15, val);
        self.cpsr.set_t(val & 1 == 1); // Set Thumb bit based on LSB
    }

    fn data_proc_instr(&mut self, encoding: u32) {
        let imm_flag = (encoding >> 25) & 1;
        let set_cond_flag = (encoding >> 20) & 1;
        let opcode = (encoding >> 21) & 0b1111;

        let op1_reg_n = ((encoding >> 16) & 0b1111) as usize;
        let op1_reg = self.get_register(op1_reg_n)
            + if op1_reg_n == 15 {
                // If the PC is used as an operand, prefetching causes it to be higher
                // by an amount depending on whether the shift is specified directly or by a register
                if imm_flag == 1 {
                    8
                } else {
                    12
                }
            } else {
                0
            };
        let dest_reg_n = ((encoding >> 12) & 0b1111) as usize;

        // http://vision.gel.ulaval.ca/~jflalonde/cours/1001/h17/docs/arm-instructionset.pdf pages 4-12 through 4-15
        // TODO: PC is supposed to produce lots of special cases
        let (op2, shifter_carry) = if imm_flag == 1 {
            let rotate = ((encoding >> 8) & 0b1111) * 2;
            let imm = encoding & 0xFF;
            let shifter_operand = imm.rotate_right(rotate);
            let shifter_carry = if rotate == 0 {
                self.cpsr.get_c()
            } else {
                (shifter_operand >> 31) & 1 == 1
            };
            (shifter_operand, shifter_carry)
        } else {
            let op2_reg_n = (encoding & 0b1111) as usize;
            let op2_reg = self.get_register(op2_reg_n)
                + if op2_reg_n == 15 {
                    if imm_flag == 1 {
                        8
                    } else {
                        12
                    }
                } else {
                    0
                };

            let shift_by_reg = (encoding >> 4) & 1 == 1;
            let shift_amount = if shift_by_reg {
                self.get_register(((encoding >> 8) & 0b1111) as usize)
            } else {
                (encoding >> 7) & 0b11111
            };

            if shift_by_reg && shift_amount == 0 {
                (op2_reg, self.cpsr.get_c())
            } else {
                match (encoding >> 5) & 0b11 {
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
        };

        let carry = self.cpsr.get_c() as u32;
        let (result, update_overflow, write_result) = match opcode {
            0b0000 => (op1_reg & op2, false, true),            // AND
            0b0001 => (op1_reg ^ op2, false, true),            // EOR
            0b0010 => (op1_reg.wrapping_sub(op2), true, true), // SUB
            0b0011 => (op2.wrapping_sub(op1_reg), true, true), // RSB
            0b0100 => (op1_reg.wrapping_add(op2), true, true), // ADD
            0b0101 => (op1_reg.wrapping_add(op2).wrapping_add(carry), true, true), // ADC
            0b0110 => (
                op1_reg
                    .wrapping_sub(op2)
                    .wrapping_add(carry)
                    .wrapping_sub(1),
                true,
                true,
            ), // SBC
            0b0111 => (
                op2.wrapping_sub(op1_reg)
                    .wrapping_add(carry)
                    .wrapping_sub(1),
                true,
                true,
            ), // RSC
            0b1000 => (op1_reg & op2, false, false),           // TST
            0b1001 => (op1_reg ^ op2, false, false),           // TEQ
            0b1010 => (op1_reg.wrapping_sub(op2), true, false), // CMP
            0b1011 => (op1_reg.wrapping_add(op2), true, false), // CMN
            0b1100 => (op1_reg | op2, false, true),            // OOR
            0b1101 => (op2, false, true),                      // MOV
            0b1110 => (op1_reg & !op2, false, true),           // BIC
            0b1111 | _ => (!op2, false, true),                 // MVN
        };

        if write_result {
            self.set_register(dest_reg_n, result);
        }

        if set_cond_flag == 1 {
            if dest_reg_n == 15 {
                if let Some(spsr) = self.get_mode_spsr() {
                    self.cpsr.raw = spsr.raw;
                } else {
                    panic!("attempted to copy from SPSR in User or System mode");
                }
            } else {
                if update_overflow {
                    // TODO
                    // self.cpsr.set_v()
                }
                self.cpsr.set_c(shifter_carry);
                self.cpsr.set_z(result == 0);
                self.cpsr.set_n((result >> 31) & 1 == 1);
            }
        }
    }

    fn branch_instr(&mut self, encoding: u32) {
        let link_flag = (encoding >> 24) & 1;
        if link_flag == 1 {
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
}

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
