use std::io::Read;
use std::fs::File;
use rvsim::{MemoryAccess, Op};
use crate::port::Port;

struct SimMemory {
    ram: Vec<u8>,
    mmio_mark: bool,
}

impl SimMemory {
    fn from_file(path: &str) -> Self {
        let mut f = File::open(path).unwrap();
        let mut ram = vec![];
        f.read_to_end(&mut ram).unwrap();
        ram.resize(65536, 0);
        Self { ram, mmio_mark: false }
    }
}

impl rvsim::Memory for SimMemory {
    fn access<T: Copy>(&mut self, addr: u32, access: MemoryAccess<T>) -> bool {
        if addr >> 28 == 0xf {
            self.mmio_mark = true;
            true
        } else {
            rvsim::Memory::access(&mut self.ram[..], addr, access)
        }
    }
}

fn decode_value<T: Copy>(x: T) -> u32 {
    unsafe {
        use std::mem::transmute_copy;
        match std::mem::size_of::<T>() {
            1 => transmute_copy::<T, u8>(&x) as u32,
            2 => transmute_copy::<T, u16>(&x) as u32,
            4 => transmute_copy::<T, u32>(&x),
            _ => panic!("decode_value: bad size")
        }
    }
}

pub struct Executor {
    m: SimMemory,
    clock: rvsim::SimpleClock,
    cpu: rvsim::CpuState,

    remote_rf: [u32; 32],

    inst_count: u64,
}

impl Executor {
    pub fn new(memory_path: &str) -> Self {
        Executor {
            m: SimMemory::from_file(memory_path),
            clock: rvsim::SimpleClock::new(),
            cpu: rvsim::CpuState::new(0),
            remote_rf: [0; 32],
            inst_count: 0,
        }
    }

    pub fn next(&mut self, commit: Port) {
        if commit.pc != self.cpu.pc {
            panic!("pc mismatch: remote: 0x{:016x}, local: 0x{:016x}", commit.pc, self.cpu.pc);
        }
        let mut interp = rvsim::Interp::new(&mut self.cpu, &mut self.m, &mut self.clock);
        let op = match interp.step() {
            Ok(x) => x,
            Err((e, op)) => {
                let mut fixed = false;
                if let Some(ref op) = op {
                    if Self::is_csr_op(&op) {
                        fixed = true;
                    }
                }
                if !fixed {
                    panic!("Simulation error: {:?} {:?}", e, op);
                }
                op.unwrap()
            }
        };
        println!("interp pc: 0x{:016x}", self.cpu.pc);
        self.apply_commit(commit);
        self.patch_mmio_mark(&op);
        if self.remote_rf != self.cpu.x {
            println!("regfile mismatch: remote: {:?}, local: {:?}", self.remote_rf, self.cpu.x);
            if !self.m.mmio_mark {
                panic!("Regfile mismatch not accepted.")
            } else {
                eprintln!("Accepted regfile mismatch after MMIO/CSR operation.");
                self.cpu.x = self.remote_rf;
            }
        }
        self.m.mmio_mark = false;

        self.inst_count += 1;
        if self.inst_count % 1000000 == 0 {
            eprintln!("Co-simulated {} instructions.", self.inst_count);
        }
    }

    fn is_csr_op(op: &Op) -> bool {
        match op {
            Op::Csrrw { .. } | Op::Csrrs { .. } | Op::Csrrc { .. } | Op::Csrrwi { .. } | Op::Csrrsi { .. } | Op::Csrrci { .. } => true,
            _ => false
        }
    }

    fn patch_mmio_mark(&mut self, op: &Op) {
        if Self::is_csr_op(op) {
            self.m.mmio_mark = true;
        }
    }

    fn apply_commit(&mut self, commit: Port) {
        if let Some(w) = commit.reg_write {
            if w.0 != 0 {
                self.remote_rf[w.0 as usize] = w.1;
            }
        }
    }
}