mod executor;
mod port;

use port::Port;
use regex::Regex;
use std::io::{stdin, BufRead};
use std::str::FromStr;

fn main() {
    let mut executor =
        executor::Executor::new(&std::env::args().nth(1).expect("missing path to firmware"));
    let mut bubble_count: u64 = 0;
    for line in stdin().lock().lines() {
        let line = line.unwrap();
        let mut parts = line.split(" ");
        let port1 = parse_commit_port(parts.next().expect("expecting port 1"));
        let port2 = parse_commit_port(parts.next().expect("expecting port 2"));
        println!("{:x?} {:x?}", port1, port2);
        bubble_count += 1;
        if let Some(port1) = port1 {
            executor.next(port1);
            bubble_count = 0;
        }
        if let Some(port2) = port2 {
            executor.next(port2);
            bubble_count = 0;
        }
        if bubble_count == 100 {
            panic!("Too many continuous bubbles. Core stuck?");
        }
    }
}

fn parse_commit_port(s: &str) -> Option<Port> {
    if s == "(bubble)" {
        return None;
    }

    let re = Regex::new(r"^[\[](.+)[\]]<(.+)>$").unwrap();
    let cap = re.captures(s).unwrap();
    assert_eq!(cap.len(), 3);
    Some(Port {
        pc: decode_hex(&cap[1]),
        reg_write: decode_reg_write(&cap[2]),
    })
}

fn decode_hex(x: &str) -> u32 {
    let without_prefix = x.trim_start_matches("0x");
    u32::from_str_radix(without_prefix, 16).unwrap()
}

fn decode_reg_write(x: &str) -> Option<(u8, u32)> {
    if x == "no_write" {
        None
    } else {
        let x = x.trim_start_matches("write:");
        let mut parts = x.split("=");
        let i = u8::from_str(parts.next().unwrap()).unwrap();
        let v = decode_hex(parts.next().unwrap());
        Some((i, v))
    }
}
