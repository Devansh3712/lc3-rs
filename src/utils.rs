pub fn opc(i: u16) -> u16 {
    i >> 12
}

pub fn dr(i: u16) -> u16 {
    (i >> 9) & 0x7
}

pub fn sr1(i: u16) -> u16 {
    (i >> 6) & 0x7
}

pub fn sr2(i: u16) -> u16 {
    i & 0x7
}

pub fn imm(i: u16) -> u16 {
    i & 0x1F
}

pub fn fimm(i: u16) -> u16 {
    (i >> 5) & 1
}

pub fn fl(i: u16) -> u16 {
    (i >> 11) & 1
}

pub fn fcnd(i: u16) -> u16 {
    (i >> 9) & 0x07
}

pub fn trp(i: u16) -> u16 {
    i & 0xFF
}

fn sext(n: u16, b: i16) -> u16 {
    if ((n >> (b - 1)) & 1) == 1 {
        return n | (0xFFFF << b);
    }
    n
}

pub fn sextimm(i: u16) -> u16 {
    sext(imm(i), 5)
}

pub fn poff(i: u16) -> u16 {
    sext(i & 0x3F, 6)
}

pub fn poff9(i: u16) -> u16 {
    sext(i & 0x1F, 9)
}

pub fn poff11(i: u16) -> u16 {
    sext(i & 0x7FF, 11)
}
