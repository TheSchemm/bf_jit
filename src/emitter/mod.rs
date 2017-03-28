#![feature(question_mark)]
#![feature(associated_consts)]
extern crate byteorder;

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

    pub enum Jmp{
      JO   = 0x0,
      JNO  = 0x1,
      JB   = 0x2,
      JNB  = 0x3,
      JZ   = 0x4,
      JNZ  = 0x5,
      JBE  = 0x6,
      JNBE = 0x7,
      JS   = 0x8,
      JNS  = 0x9,
      JP   = 0xA,
      JNP  = 0xB,
      JL   = 0xC,
      JNL  = 0xD,
      JLE  = 0xE,
      JNLE = 0xF,
    }

    pub const JNAE:Jmp = Jmp::JB;
    pub const   JC:Jmp = Jmp::JB;

    pub const  JAE:Jmp = Jmp::JNB;
    pub const  JNC:Jmp = Jmp::JNB;

    pub const   JE:Jmp = Jmp::JZ;

    pub const  JNE:Jmp = Jmp::JNZ;

    pub const  JNA:Jmp = Jmp::JBE;

    pub const   JA:Jmp = Jmp::JNBE;

    pub const  JPE:Jmp = Jmp::JP;

    pub const  JPO:Jmp = Jmp::JNP;

    pub const JNGE:Jmp = Jmp::JL;

    pub const  JGE:Jmp = Jmp::JNL;

    pub const  JNG:Jmp = Jmp::JLE;

    pub const   JG:Jmp = Jmp::JNLE;




    #[derive(Eq, PartialEq, Debug, Copy, Clone)]
    pub enum Register {
        Reg64(Reg64),
    }

    #[derive(Eq, PartialEq, Debug, Copy, Clone)]
    pub enum Opcode{
        Cmp,
        Dec,
        Inc,
        Mov,
        Ret,


    }

    pub enum Operand {
        None,
        Register(Register),
        Imm8(u8),
        Imm32(u32),
        Reg64Imm32{r:Reg64, i:u32},
        Reg64Reg64{d:Reg64, s:Reg64},
        BytePtr(Reg64),
        BytePtrImm8{d:Reg64, s:u8}
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

    pub fn emit_cmp(oprnd: x64::Operand) -> Result<Vec<u8>,&'static str>{
        use self::x64::Operand;
        match oprnd {
            Operand::BytePtrImm8{ d:r, s: imm8} => {
                let b = (r as u8 >> 3) & 0x1;
                let reg = r as u8 & 07;



                let mut temp = vec![0x80u8, Emitter::ModRM(0b11, reg, 0), imm8];
                if  b == 1 {
                    temp.insert(0, Emitter::REX(false, false, false, true));
                }


                Ok(temp)
            },
            _ => {
                Err("Unimplemented")
            }

        }


    }

    pub fn emit_inc_dec(oprnd: x64::Operand, inc: bool) -> Result<Vec<u8>,&'static str>{
        use self::x64::Operand;
        let reg:u8 = (!inc) as u8;
        match oprnd {
            Operand::Register(r) => {
                match r{
                    x64::Register::Reg64(r64) => {
                        let temp = r64;
                        let rm:u8 = (temp as u8) & 0x7;
                        let b:u8  = ((temp as u8) >> 3) & 0x1;
                        println!("{:02x} {:02x} {:02x}", Emitter::REX(true, false, false, b == 1), 0xff, Emitter::ModRM(0b11, reg, rm));
                        return Ok(vec![Emitter::REX(true, false, false, b == 1), 0xff, Emitter::ModRM(0b11, reg, rm)]);
                    },

                    //_ => {},
                }
            },

            Operand::BytePtr(r) => {
                let  b:u8 = (r as u8 >> 3) & 0x1;
                let rm:u8 = (r as u8 & 0x7);

                if b == 1 {
                    return Ok(vec![Emitter::REX(false, false, false, false), 0xfe, Emitter::ModRM(0,reg,rm)]);

                }else{
                    return Ok(vec![0xfe, Emitter::ModRM(0,reg,rm)]);
                }


            },

            _ => {
                Err("Unimplemented")
            }

        }
    }


    pub fn emit_mov(oprnd: x64::Operand) -> Result<Vec<u8>,&'static str>{
        use self::x64::Operand;
        use self::byteorder::{LittleEndian, WriteBytesExt};
        match oprnd {
            Operand::Reg64Imm32{r,i} => {
                let temp = r;
                let rm:u8 = (temp as u8) & 0x7;
                let b:u8  = ((temp as u8) >> 3) & 0x1;
                println!("{:02x} {:02x} {:02x}", Emitter::REX(true, false, false, b == 1), 0xff, Emitter::ModRM(0b11, 0, rm));
                let mut temp_vec = vec![Emitter::REX(true, false, false, b == 1), 0xc7, Emitter::ModRM(0b11, 0, rm)];
                let mut le = vec![];
                le.write_u32::<LittleEndian>(i).unwrap();
                temp_vec.append(&mut le);
                return Ok(temp_vec);
            },

            Operand::Reg64Reg64{d,s} => {

                let rm:u8 = (d as u8) & 0x7;
                let b:u8  = ((d as u8) >> 3) & 0x1;
                let reg:u8 = (s as u8) & 0x7;
                let r:u8 = ((s as u8) >> 3) & 0x1;
                println!("{:02x} {:02x} {:02x}", Emitter::REX(true, r == 1, false, b == 1), 0x89, Emitter::ModRM(0b11, reg, rm));
                let mut temp_vec = vec![Emitter::REX(true, false, r == 1, b == 1), 0x89, Emitter::ModRM(0b11, reg, rm)];
                return Ok(temp_vec);
            },
            _ => {
                Err("Unimplemented")
            }

        }
    }

    #[cfg(windows)]
    pub fn ArgReg(i: u8) -> x64::Reg64 {
        match i {
            0 => x64::Reg64::Rcx,
            1 => x64::Reg64::Rdx,
            2 => x64::Reg64::R8,
            3 => x64::Reg64::R9,

            _ => unreachable!(),
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
            //(Ret, _)  => Err("Invalid"),
            (Inc, o) => Emitter::emit_inc_dec(o,true),
            (Dec, o) => Emitter::emit_inc_dec(o,false),
            (Mov, o) => Emitter::emit_mov(o),
            _ => Err("Invalid"),

        };

        match ret_bytes {
            Ok(ok) => {
                    size = ok.len() as i32;
                    match cb.write_bytes(&ok) {
                        Ok(_) => {},
                        Err(s) => println!("Error: {}", s),
                    }
                },
            Err(wat)  => {println!("Error: {}", wat); size = -1},
        }


        size
    }


}
