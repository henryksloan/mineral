// https://www.ecs.csun.edu/~smirzaei/docs/ece425/arm7tdmi_instruction_set_reference.pdf page 2
#[derive(Debug)]
pub enum Condition {
    EQ, // Equal: Z==1
    NE, // Not equal: Z==0
    CS, // aka HS. Carry set / unsigned higher or same: C==1
    CC, // aka LO. Carry clear / unsigned lower: C==0
    MI, // Minus / negative: N==1
    PL, // Plus / positive or zero: N==0
    VS, // Overflow: V==1
    VC, // No overflow: V==0
    HI, // Unsigned higher: (C==1) AND (Z==0)
    LS, // Unsigned lower or same: (C==0) OR (Z==1)
    GE, // Signed greater than or equal: N == V
    LT, // Signed less than: N != V
    GT, // Signed greater than: (Z==0) AND (N==V)
    LE, // Signed less than or equal: (Z==1) OR (N!=V)
    AL, // Always (unconditional)
    NV, // Never: Obsolete, unpredictable in ARM7TDMI
}

impl Condition {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0b0000 => Self::EQ,
            0b0001 => Self::NE,
            0b0010 => Self::CS,
            0b0011 => Self::CC,
            0b0100 => Self::MI,
            0b0101 => Self::PL,
            0b0110 => Self::VS,
            0b0111 => Self::VC,
            0b1000 => Self::HI,
            0b1001 => Self::LS,
            0b1010 => Self::GE,
            0b1011 => Self::LT,
            0b1100 => Self::GT,
            0b1101 => Self::LE,
            0b1110 => Self::AL,
            _ => Self::NV,
        }
    }
}
