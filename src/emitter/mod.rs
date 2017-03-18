use CodeBuff;

pub mod x64 {
    
    #[derive(Eq, PartialEq, Debug, Copy, Clone)]
    pub enum Reg64 {
        Rax = 0,
        Rcx = 1,
        Rdx = 2,
        Rbx = 3,
        Rsp = 4,
        Rbp = 5,
        Rsi = 6,
        Rdi = 7,
        R8  = 8,
        R9  = 9,
        R10 = 10,
        R11 = 11,
        R12 = 12,
        R13 = 13,
        R14 = 14,
        R15 = 15,
    }
    
    #[derive(Eq, PartialEq, Debug, Copy, Clone)]
    pub enum Register {
        Reg64(Reg64),
    
    }
    
    #[derive(Eq, PartialEq, Debug, Copy, Clone)]
    pub enum Opcode{
        Inc,
        Ret,
    
    
    }
    
    pub enum Operand {
        None,
        Register(Register),
    
    }
    
}

use std::io::{Write, Error, Cursor};


pub struct Emitter {
    unused: usize,
    

}


impl Emitter{
    pub fn new() -> Emitter {
        
        Emitter{ unused: 0}
    }
    
    
    pub fn ModRM(m:u8, reg:u8, rm:u8) -> u8 {
    
        (m & 3) << 6 | (reg & 7) << 3 | (rm & 7)
    }

    pub fn emit_inc(oprnd: x64::Operand) -> Result<Vec<u8>,&'static str>{
        use self::x64::Operand;
        
        match oprnd {
            Operand::Register(r) => {
                match r{
                    x64::Register::Reg64(r64) => {
                        let temp = r64;
                        let rm:u8 = (temp as u8) & 0x7;
                        let b:u8  = ((temp as u8) >> 3) & 0x1;
                        println!("{:02x} {:02x} {:02x}", Emitter::REX(true, false, false, b == 1), 0xff, Emitter::ModRM(0b11, 0, rm));
                        return Ok(vec![Emitter::REX(true, false, false, b == 1), 0xff, Emitter::ModRM(0b11, 0, rm)]);
                    },
                    
                    //_ => {},
                }
            },
            _ => {
                Err("Unimplemented")
            }
        
        }
        
        
    }
    
    
    pub fn REX(w:bool, r:bool, x:bool, b:bool) -> u8{
        let mut rex: u8;
        
        rex = 0x40 | ((w as u8) << 3) | ((r as u8) << 2) | ((x as u8) << 1) | ((b as u8) << 0);
        rex
    }
    

    pub fn emit(&self, op:x64::Opcode, oprnd:x64::Operand, cb: &mut CodeBuff) -> i32 {
        
        use self::x64::Opcode::*;
        use self::x64::Operand::*;
        let size:i32;
        let ret_bytes = match (op, oprnd) {
            (Ret, self::x64::Operand::None) => Ok(vec![0xc3u8]),
            //(Ret,    ) => {println!("Invalid instruction.");}
            (Ret, _)  => Err("Invalid"),
            (Inc, o) => Emitter::emit_inc(o),
        
             
        };
        
        match ret_bytes {
            Ok(ok) => {
                    size = ok.len() as i32; 
                    match cb.write_bytes(&ok) {
                        Ok(_) => {},
                        Err(s) => println!("Error: {}", s),
                    }
                },
            Err(_)  => {println!("Error"); size = -1},
        }
        
        
        size
    }
}