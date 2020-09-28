use std::io::Read;
use std::fs::File;
use rvsim::MemoryAccess;
use crate::port::Port;

struct SimMemory {
    ram: Vec<u8>,
}

impl SimMemory {
    fn from_file(path: &str) -> Self {
        let mut f = File::open(path).unwrap();
        let mut ram = vec![];
        f.read_to_end(&mut ram).unwrap();
        ram.resize(65536, 0);
        Self { ram }
    }
}

impl rvsim::Memory for SimMemory {
    fn access<T: Copy>(&mut self, addr: u32, access: MemoryAccess<T>) -> bool {
        if addr >> 28 == 0xf {
            // IO mem
            match addr {
                0xfe000000 => {
                    match access {
                        MemoryAccess::Store(value) => {
                            let value = decode_value(value);
                            println!("Cosim putchar: {}", value);
                            true
                        }
                        _ => panic!("bad access to 0xfe000000")
                    }
                }
                _ => panic!("invalid io address: 0x{:016x}", addr)
            }
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
}

impl Executor {
    pub fn new(memory_path: &str) -> Self {
        Executor {
            m: SimMemory::from_file(memory_path),
            clock: rvsim::SimpleClock::new(),
            cpu: rvsim::CpuState::new(0),
            remote_rf: [0; 32],
        }
    }

    pub fn next(&mut self, commit: Port) {
        if commit.pc != self.cpu.pc {
            println!("pc mismatch: remote: 0x{:016x}, local: 0x{:016x} - rectifying local and continueing", commit.pc, self.cpu.pc);
            self.cpu.pc = commit.pc;
        }
        let mut interp = rvsim::Interp::new(&mut self.cpu, &mut self.m, &mut self.clock);
        interp.step().unwrap();
        println!("interp pc: 0x{:016x}", self.cpu.pc);
        self.apply_commit(commit);
        if self.remote_rf != self.cpu.x {
            panic!("regfile mismatch: remote: {:?}, local: {:?}", self.remote_rf, self.cpu.x);
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