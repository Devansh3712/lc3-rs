use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::ops::{Index, IndexMut};

mod utils;
use utils::*;

#[allow(dead_code)]
enum Register {
    R0 = 0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    RPC,
    RCND,
    RCNT,
}

impl Index<Register> for [u16] {
    type Output = u16;

    fn index(&self, register: Register) -> &Self::Output {
        &self[register as usize]
    }
}

impl IndexMut<Register> for [u16] {
    fn index_mut(&mut self, register: Register) -> &mut Self::Output {
        &mut self[register as usize]
    }
}

enum Flag {
    FP = 1 << 0,
    FZ = 1 << 1,
    FN = 1 << 2,
}

const NOPS: usize = 16;
const TRAP_OFFSET: u16 = 0x20;
const MEMORY_SIZE: usize = u16::MAX as usize + 1;
const REGISTER_COUNT: usize = Register::RCNT as usize;

// TODO: Implement Default trait
struct VirtualMachine {
    pc: u16,
    running: bool,
    memory: [u16; MEMORY_SIZE],
    registers: [u16; REGISTER_COUNT],
    trp_ex: [fn(&mut VirtualMachine); 8],
    op_ex: [fn(&mut VirtualMachine, u16); NOPS],
}

impl VirtualMachine {
    fn new() -> Self {
        VirtualMachine {
            pc: 0x3000,
            running: true,
            memory: [0; MEMORY_SIZE],
            registers: [0; REGISTER_COUNT],
            trp_ex: [
                Self::tgetc,
                Self::tout,
                Self::tputs,
                Self::tin,
                Self::tputsp,
                Self::thalt,
                Self::tinu16,
                Self::toutu16,
            ],
            op_ex: [
                Self::br,
                Self::add,
                Self::ld,
                Self::st,
                Self::jsr,
                Self::and,
                Self::ldr,
                Self::str,
                Self::rti,
                Self::not,
                Self::ldi,
                Self::sti,
                Self::jmp,
                Self::res,
                Self::lea,
                Self::trap,
            ],
        }
    }

    fn memread(&self, address: u16) -> u16 {
        self.memory[address as usize]
    }

    fn memwrite(&mut self, address: u16, value: u16) {
        self.memory[address as usize] = value;
    }

    fn uf(&mut self, register: u16) {
        let val = self.registers[register as usize];
        let flag = match val {
            0 => Flag::FZ,
            v if (v >> 15) == 1 => Flag::FN,
            _ => Flag::FP,
        };
        self.registers[Register::RCND] = flag as u16;
    }

    fn add(&mut self, i: u16) {
        let val = match fimm(i) {
            1 => sextimm(i),
            _ => self.registers[sr2(i) as usize],
        };
        self.registers[dr(i) as usize] = self.registers[sr1(i) as usize] + val;
        self.uf(dr(i));
    }

    fn and(&mut self, i: u16) {
        let val = match fimm(i) {
            1 => sextimm(i),
            _ => self.registers[sr2(i) as usize],
        };
        self.registers[dr(i) as usize] = self.registers[sr1(i) as usize] & val;
        self.uf(dr(i));
    }

    fn not(&mut self, i: u16) {
        self.registers[dr(i) as usize] = !self.registers[sr1(i) as usize];
        self.uf(dr(i));
    }

    fn ld(&mut self, i: u16) {
        self.registers[dr(i) as usize] = self.memread(self.registers[Register::RPC] + poff9(i));
        self.uf(dr(i));
    }

    fn ldi(&mut self, i: u16) {
        self.registers[dr(i) as usize] =
            self.memread(self.memread(self.registers[Register::RPC] + poff9(i)));
        self.uf(dr(i));
    }

    fn ldr(&mut self, i: u16) {
        self.registers[dr(i) as usize] = self.memread(self.registers[sr1(i) as usize] + poff(i));
        self.uf(dr(i));
    }

    fn lea(&mut self, i: u16) {
        self.registers[dr(i) as usize] = self.registers[Register::RPC] + poff9(i);
        self.uf(dr(i));
    }

    fn st(&mut self, i: u16) {
        self.memwrite(
            self.registers[Register::RPC] + poff9(i),
            self.registers[dr(i) as usize],
        );
    }

    fn sti(&mut self, i: u16) {
        self.memwrite(
            self.memread(self.registers[Register::RPC] + poff9(i)),
            self.registers[dr(i) as usize],
        );
    }

    fn str(&mut self, i: u16) {
        self.memwrite(
            self.registers[sr1(i) as usize] + poff(i),
            self.registers[dr(i) as usize],
        );
    }

    fn jmp(&mut self, i: u16) {
        self.registers[Register::RPC] = self.registers[sr1(i) as usize];
    }

    fn jsr(&mut self, i: u16) {
        self.registers[Register::R7] = self.registers[Register::RPC];
        let val = match fl(i) {
            1 => self.registers[Register::RPC] + poff11(i),
            _ => self.registers[sr1(i) as usize],
        };
        self.registers[Register::RPC] = val;
    }

    fn rti(&mut self, _i: u16) {}
    fn res(&mut self, _i: u16) {}

    fn br(&mut self, i: u16) {
        if self.registers[Register::RCND] & fcnd(i) == 1 {
            self.registers[Register::RPC] += poff9(i);
        }
    }

    fn tgetc(&mut self) {
        let mut buffer = [0; 1];
        if io::stdin().read_exact(&mut buffer).is_ok() {
            self.registers[Register::R0] = buffer[0] as u16;
        }
    }

    fn tout(&mut self) {
        let val = self.registers[Register::R0] as u8 as char;
        print!("{}", val);
        io::stdout().flush().unwrap();
    }

    fn tputs(&mut self) {
        let mut address = self.registers[Register::R0];
        while self.memory[address as usize] != 0 {
            let val = self.memory[address as usize] as u8 as char;
            print!("{}", val);
            address = address.wrapping_add(1);
        }
        io::stdout().flush().unwrap();
    }

    fn tin(&mut self) {
        let mut buffer = [0; 1];
        if io::stdin().read_exact(&mut buffer).is_ok() {
            self.registers[Register::R0] = buffer[0] as u16;
            print!("{}", buffer[0] as u8 as char);
            io::stdout().flush().unwrap();
        }
    }

    fn thalt(&mut self) {
        self.running = false;
    }

    fn tinu16(&mut self) {
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            if let Ok(val) = input.trim().parse::<u16>() {
                self.registers[Register::R0] = val;
            }
        }
    }

    fn toutu16(&mut self) {
        println!("{}", self.registers[Register::R0]);
        io::stdout().flush().unwrap();
    }

    fn tputsp(&mut self) {}

    fn trap(&mut self, i: u16) {
        self.trp_ex[(trp(i) - TRAP_OFFSET) as usize](self);
    }

    fn load(&mut self, fname: &str, offset: u16) -> io::Result<()> {
        let mut file = File::open(fname)?;
        let start = (self.pc + offset) as usize;
        let max_words = MEMORY_SIZE - start;
        let mem_slice = &mut self.memory[start..start + max_words];

        // read_exact requires &mut [u8] buffer as input and the VM memory
        // is [u16], so we create a new slice from a raw pointer to the first byte
        // of mem_slice casted as *mut u8
        // since each u16 is 2 bytes, &mut [u16] is treated as &mut [u8] that is
        // twice as long
        let byte_slice = unsafe {
            std::slice::from_raw_parts_mut(mem_slice.as_mut_ptr() as *mut u8, mem_slice.len() * 2)
        };
        file.read(byte_slice)?;

        Ok(())
    }

    fn start(&mut self, offset: u16) {
        self.registers[Register::RPC as usize] = self.pc.wrapping_add(offset);
        while self.running {
            let i: u16 = self.memread(self.registers[Register::RPC]);
            self.registers[Register::RPC] = self.registers[Register::RPC].wrapping_add(1);
            self.op_ex[opc(i) as usize](self, i);
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut vm = VirtualMachine::new();
    vm.load(&args[1], 0x0)?;
    vm.start(0x0);

    Ok(())
}
