

extern crate libc;

#[cfg(windows)] extern crate kernel32;
#[cfg(windows)] extern crate winapi;

use std::io::{Write, Error, ErrorKind, Cursor};
use std::ops::{Index, IndexMut};
use std::marker::PhantomData;


struct CodeBuff {
    buff : *mut u8,
    size: u32,
    
    pos: isize,
}

impl CodeBuff {
    fn new(num_pages: u32) -> Result<CodeBuff, Error> {
        let _buff: *mut u8;
        
        match CodeBuff::alloc(num_pages) {
            Ok(page) => _buff = page,
            Err(err) => return Err(err)
        }
        
        let _size = CodeBuff::get_page_size() * num_pages;
        Ok(CodeBuff{buff: _buff, size: _size, pos: 0})
    }

    #[cfg(windows)]
    pub fn get_page_size() -> u32 {
        use winapi::sysinfoapi::SYSTEM_INFO;
        use kernel32::GetSystemInfo;
        
        let mut sys_info: SYSTEM_INFO;
        let ret = unsafe { 
            sys_info = std::mem::uninitialized();
            GetSystemInfo(&mut sys_info as *mut SYSTEM_INFO)
        };

        sys_info.dwPageSize
    }

    #[cfg(windows)]
    fn alloc(num_pages: u32) -> Result<(*mut u8), Error> {
        use kernel32::VirtualAlloc;
        use std::ptr::null_mut;
        use std::os::raw::c_void;
        use winapi::winnt::{MEM_COMMIT, PAGE_READWRITE};
        
        
        let page_size = CodeBuff::get_page_size() * num_pages;
        let page: *mut c_void;
        let lp_address: * mut c_void = null_mut();
        unsafe {
            page = VirtualAlloc(lp_address, page_size as u64, MEM_COMMIT, PAGE_READWRITE);   
        }
        
        if page.is_null(){
            Err(Error::last_os_error())
        }else{
            Ok(page as *mut u8)
        }   
    }
    
    #[cfg(windows)]
    fn protect(&mut self, exec_en:bool, write_en:bool) -> Result<(), Error> {
        use kernel32::VirtualProtect;
        use winapi::winnt::{PAGE_READWRITE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_READONLY};
        use std::os::raw::c_void;
        let prot: u32;
        
        match (exec_en, write_en){
            ( true,  true) => prot = PAGE_EXECUTE_READWRITE,
            ( true, false) => prot = PAGE_EXECUTE_READ,
            (false,  true) => prot = PAGE_READWRITE,
            (false, false) => prot = PAGE_READONLY,
        };
        let mut old:u32 = 0;
        let ret = unsafe {
            VirtualProtect(self.buff as *mut c_void, self.size as u64,  prot, &mut old)
        };
        
        if ret == 0 {
            Err(Error::last_os_error())
        }else{
            Ok(())
        }   
    }
    
    fn get_function(&self, offset: isize) -> (fn() -> i64) {
        unsafe {
            std::mem::transmute(self.buff.offset(offset))
        }
    }
    
    
    
    fn get_function1<RT, T1>(&self, offset: isize) -> fn(T1) -> RT {
        unsafe {
            std::mem::transmute(self.buff.offset(offset))
        }
    }
    
    
    fn get_address(&self, offset:isize) -> usize {
        unsafe {
            std::mem::transmute(self.buff.offset(offset))
        }
    }
    
    fn get_size(&self) -> u32 {
        self.size
    }
    
    pub fn position(&self) -> isize {
        self.pos
    }
    
    fn set_position(&mut self, pos:isize) {
        self.pos = pos
    }
    
    fn write_u8(&mut self, x:u8) {
        unsafe { 
            *self.buff.offset(self.pos) = x;
        }
        self.pos = self.pos + 1;
    }
    
    fn write_u16(&mut self, x:u16) {
        unsafe { 
            *(self.buff.offset(self.pos) as *mut u16) = x;
        }
        self.pos = self.pos + 2;
    }
    
    fn write_u32(&mut self, x:u32) {
        unsafe { 
            *(self.buff.offset(self.pos) as *mut u32) = x;
        }
        self.pos = self.pos + 4;
    }
    
    fn write_u64(&mut self, x:u64) {
        unsafe { 
            *(self.buff.offset(self.pos) as *mut u64) = x;
        }
        self.pos = self.pos + 8;
    }
    
    fn write<T>(&mut self, x:T) {
        unsafe { 
            *(self.buff.offset(self.pos) as *mut T) = x;
        }
        self.pos = self.pos + std::mem::size_of::<T>() as isize;
    }
    
    fn write_bytes(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        
        for b in buf {
            if self.pos >= self.size as isize {
                return Err(Error::new(ErrorKind::AddrNotAvailable, "Ran out of buffer room."));
            }
            unsafe { *self.buff.offset(self.pos) = *b; }
            self.pos += 1;
            
        }
        Ok(buf.len())
    }
    
}




impl Index<usize> for CodeBuff {
    type Output = u8;

    fn index(&self, _index: usize) -> &u8 {
        unsafe {&*self.buff.offset(_index as isize) }
    }
}

impl IndexMut<usize> for CodeBuff {
    fn index_mut(&mut self, _index: usize) -> &mut u8 {
        unsafe {&mut *self.buff.offset(_index as isize) }
    }
}

impl Drop for CodeBuff {
    #[cfg(windows)]
    fn drop(&mut self){
        use kernel32::VirtualFree;
        use std::os::raw::c_void;
        use winapi::winnt::{MEM_RELEASE};
        if !self.buff.is_null() {
            let ret = unsafe {
                VirtualFree(self.buff as *mut c_void, 0, MEM_RELEASE)
            };
            if ret == 0{
                println!("VirtualFree failed: {}", Error::last_os_error())
            }
        }
    }
}


mod Emitter;

use Emitter::x64;


mod bf {
    use std::fs::File;
    use std::io::prelude::*;
    use std::io;
    use std::collections::HashMap;
    use std::fmt;
    type CellType = u32;
    
    #[derive(Eq, PartialEq, Debug, Copy, Clone)]
    pub enum Opcode {
        Ptr(i32),
        Byte(i32),
        LoopEnter(usize),
        LoopExit(usize),
        Out,
        In,
    }
    
    impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Opcode::Ptr(x)       => write!(f, "P({})", x),
            Opcode::Byte(x)      => write!(f, "B({})", x),
            Opcode::LoopEnter(x) => write!(f, "[({})", x),
            Opcode::LoopExit(x)  => write!(f, "]({})", x),
            Opcode::Out          => write!(f, "."),
            Opcode::In           => write!(f, ","),
        
        
        }
    }
}
    
    
    pub struct OptimizedInterpreter {
    
        prog: Vec<Opcode>,
        jmp_table: HashMap<usize, usize>,
        mem:Vec<u32>,
        mem_ptr: usize,
        ip: usize,
    }
    
    impl OptimizedInterpreter {
        pub fn new() -> OptimizedInterpreter {
            
        
            OptimizedInterpreter {mem: vec![0u32;30000], mem_ptr: 0, ip:0, prog: Vec::<Opcode>::new(), jmp_table: HashMap::<usize, usize>::new(),}
        }
    
        pub fn print(&self) {
            for op in &self.prog {
                print!("{} ", op); 
            }
        
        }
        pub fn load(&mut self, f: &mut File) {
        
        
            let mut s = String::new();
            f.read_to_string(&mut s);
            
            let mut stack = Vec::<usize>::new();
            
            self.prog = Vec::<Opcode>::new();
            
            for cs in s.chars() {
                match cs {
                    '+' => {
                        match self.prog.last() {
                            Some(&Opcode::Byte(x)) => {
                                let nx = x+1;
                                self.prog.pop();
                                self.prog.push(Opcode::Byte(nx));
                            },
                            
                            _ => {
                                self.prog.push(Opcode::Byte(1))
                            },
                        }
                    }, 
                    '-' => {
                        match self.prog.last() {
                            Some(&Opcode::Byte(x)) => {
                                let nx = x-1;
                                self.prog.pop();
                                self.prog.push(Opcode::Byte(nx));
                            },
                            
                            
                            _ => {
                                self.prog.push(Opcode::Byte(-1))
                            },
                        }
                    },
                    '<' => {
                        match self.prog.last() {
                            Some(&Opcode::Ptr(x)) => {
                                let nx = x-1;
                                self.prog.pop();
                                self.prog.push(Opcode::Ptr(nx));
                            },
                            
                            _ => {
                                self.prog.push(Opcode::Ptr(-1))
                            },
                        }
                    },
                    '>' => {
                        match self.prog.last() {
                            Some(&Opcode::Ptr(x)) => {
                                let nx = x+1;
                                self.prog.pop();
                                self.prog.push(Opcode::Ptr(nx));
                            },
                            
                            _ => {
                                self.prog.push(Opcode::Ptr(1))
                            },
                        }
                    },
                    '[' => {
                        stack.push(self.prog.len());
                        self.prog.push(Opcode::LoopEnter(0));
                        
                    
                    },
                    ']' => {
                        match stack.pop() {
                            Some(x) => {
                                let i = self.prog.len();
                                self.prog.push(Opcode::LoopExit(x));
                                self.jmp_table.insert(i, x);
                                self.jmp_table.insert(x, i);
                                self.prog[x] = Opcode::LoopEnter(i);
                            },
                            None    => {
                                    println!("Unbalanced brackets!");
                                    break
                                
                                },
                        }
        
                    }, 
                    '.' => {
                        self.prog.push(Opcode::Out);
                    },
                    ',' => {
                        self.prog.push(Opcode::In);
                    },
                     _  => {},
                }
            }
            
            if stack.len() > 0 {
                println!("Unbalanced brackets!");
            }
            
        }
    
        pub fn run(&mut self) {
            
            while self.ip < self.prog.len() {
                //println!("{} {} {}",self.ip, self.mem_ptr, self.prog[self.ip]);
                match self.prog[self.ip]{
                    //Some(x) => {
                        
                        //match x {
                            Opcode::Ptr(x)       => {self.mem_ptr = self.mem_ptr.wrapping_add(x as usize);},
                            Opcode::Byte(x)      => {self.mem[self.mem_ptr] = self.mem[self.mem_ptr].wrapping_add(x as u32);},
                            
                            
                            Opcode::LoopEnter(x) => {
                                    if self.mem[self.mem_ptr] == 0 {
                                        self.ip = x;
                                    }
                                },
                            Opcode::LoopExit(x)  => {
                                    if self.mem[self.mem_ptr] != 0 {
                                        self.ip = x;
                                    }
                                },
                            Opcode::Out => {
                                    io::stdout().write(&vec![self.mem[self.mem_ptr] as u8]);
                                    io::stdout().flush();
                                },
                            Opcode::In => {
                                   
                                    let mut c:Vec<u8> = vec![0;1];
                                    io::stdin().read_exact(&mut c);
                                    self.mem[self.mem_ptr] = c[0] as u32;
                                },
                            
                            
                        //}
                       
                    
                    //},

                    //None => {break},
                }
                self.ip += 1;
            
            }
        }
    
    
    
    }
    
    
    
    
    
    
    
    pub struct Interpreter {
        mem:Vec<u32>,
        mem_ptr: usize,
        ip: usize,
        prog: Vec<char>,
        jmp_table: HashMap<usize, usize>,
    }
    
    impl Interpreter {
    
        pub fn new() -> Interpreter {
            Interpreter{mem: vec![0u32;30000], mem_ptr: 0, ip:0, prog:Vec::<char>::new(), jmp_table: HashMap::<usize,usize>::new()}
        }
        
        pub fn load(&mut self, f: &mut File) {
        
        
            let mut s = String::new();
            f.read_to_string(&mut s);
            
            self.prog = Vec::<char>::new();
            
            for c in s.chars() {
                match c {
                    
                        '+' | '-' | '<' | '>' | '[' | ']' | '.' | ',' => {
                            self.prog.push(c);
                        },
                        _ => {},
                }
            
            }
            
            
            self.build_jmp_table();
        }
        
        fn build_jmp_table(&mut self) -> Result<(), &'static str> {
            let mut ci = self.prog.iter().enumerate();
            
            let mut stack = Vec::<usize>::new();
            
            loop {
                match ci.next() {
                    Some((i,&c)) => {
                        match c {
                            '[' => {
                                    stack.push(i);
                                },
                            ']' => {
                                    match stack.pop() {
                                        Some(x) => {
                                            self.jmp_table.insert(i, x);
                                            self.jmp_table.insert(x, i);
                                        },
                                        None    => {
                                                println!("Unbalanced brackets!");
                                                break
                                            
                                            },
                                    }
                                },
                             _  => {},
                        
                        }
                    },
                    None => { 
                        if stack.len() > 0 {
                            println!("Unbalanced brackets!");
                        }
                        break
                    }
                }
            }
            
            
            Ok(())
        }
        
        
        pub fn run(&mut self) {
            
            while self.ip < self.prog.len() {
                
                match self.prog[self.ip]{
                    //Some(x) => {
                        
                        //match x {
                            '+' => self.mem[self.mem_ptr] += 1,
                            '-' => self.mem[self.mem_ptr] -= 1,
                            '<' => self.mem_ptr -= 1,
                            '>' => self.mem_ptr += 1,
                            '[' => {
                                    if self.mem[self.mem_ptr] == 0 {
                                        self.ip = *self.jmp_table.get(&self.ip).unwrap();
                                    }
                                },
                            ']' => {
                                    if self.mem[self.mem_ptr] != 0 {
                                        self.ip = *self.jmp_table.get(&self.ip).unwrap();
                                    }
                                },
                            '.' => {
                                    io::stdout().write(&vec![self.mem[self.mem_ptr] as u8]);
                                    io::stdout().flush();
                                },
                            ',' => {
                                   
                                    let mut c:Vec<u8> = vec![0;1];
                                    io::stdin().read_exact(&mut c);
                                    self.mem[self.mem_ptr] = c[0] as u32;
                                },
                            _ => {},
                            
                        //}
                       
                    
                    //},

                    //None => {break},
                }
                self.ip += 1;
            
            }
        }
        
    
    
    }




}


fn test_emitter() {
    let b = 24u8;
    
    println!("Page size: 0x{:X}", CodeBuff::get_page_size());
    
    let mut code_buff= match CodeBuff::new(1){
        Ok(mut cb)    => 
        {
            println!("Code buffer created.");
            cb
        },
        Err(err) => panic!("Code buffer creation failed: {}", err),
    };
    
    
    
    let e = Emitter::Emitter::new();
    
    //I haven't implemented any move yet. :c
    /*
    //mov rax, 0x3
    code_buff.write::<u8>(0x48);
    code_buff.write::<u8>(0xc7);
    code_buff.write::<u8>(0xc0);
    code_buff.write::<u32>(0x00000003);
    */
    e.emit(x64::Opcode::Mov, x64::Operand::Reg64Reg64{d: x64::Reg64::Rax, s: x64::Reg64::Rax}, &mut code_buff);
    e.emit(x64::Opcode::Mov, x64::Operand::Reg64Reg64{d: x64::Reg64::Rcx, s: x64::Reg64::Rax}, &mut code_buff);
    e.emit(x64::Opcode::Mov, x64::Operand::Reg64Reg64{d: x64::Reg64::Rax, s: x64::Reg64::Rcx}, &mut code_buff);
    e.emit(x64::Opcode::Mov, x64::Operand::Reg64Imm32{r: x64::Reg64::Rax, i: 3}, &mut code_buff);
    e.emit(x64::Opcode::Inc, x64::Operand::Register(x64::Register::Reg64(x64::Reg64::Rax)), &mut code_buff);
    e.emit(x64::Opcode::Ret, x64::Operand::None, &mut code_buff);

    
    
    
    let  mut pos = code_buff.position();
    let addr = code_buff.get_address(pos);
    
    println!("{:08x} {:08x}", pos, addr);
    
    //ehco function
    
    e.emit(x64::Opcode::Mov, x64::Operand::Reg64Reg64{d: x64::Reg64::Rax, s:Emitter::Emitter::ArgReg(0)}, &mut code_buff);
    e.emit(x64::Opcode::Ret, x64::Operand::None, &mut code_buff);
    
    let echo_fn = code_buff.get_function1::<u32,u32>(pos);
    
    
    let  mut pos = code_buff.position();
    
    e.emit(x64::Opcode::Inc, x64::Operand::BytePtr(Emitter::Emitter::ArgReg(0)), &mut code_buff);
    e.emit(x64::Opcode::Ret, x64::Operand::None, &mut code_buff);
    
    let inc_byte_by_ptr = code_buff.get_function1::<(),&u8>(pos);
    
    //turn on execution flag of memory (and leave write enabled (some OSes will not allow this))
    code_buff.protect(true, true);
    
    
    for y in 0..4 {
        for x in 0..16 {
            print!("{:02x} ", code_buff[x+16*y]);
        }
        println!("");
    }
    let func = code_buff.get_function(0);
    println!("Return value is: {}", func());
    
    
    println!("echo_fn(1): {}", echo_fn(1));
    println!("echo_fn(42): {}", echo_fn(42));
    
    
    
    println!("byte before: {} ", b);
    
    inc_byte_by_ptr(&b);
    
    println!("byte after: {} ", b);

}

use std::fs::File;

fn test_interpreter(){
    let mut f = File::open("mandelbrot.bf.txt").unwrap();
    let mut b = bf::Interpreter::new();
    b.load(&mut f);
    b.run();
}


fn test_optimized_interpreter(){
    let mut f = File::open("mandelbrot.bf.txt").unwrap();
    let mut b = bf::OptimizedInterpreter::new();
    b.load(&mut f);
    //b.print();
    b.run();
}
extern crate time;
use time::{Duration, PreciseTime};
//Duration: PT1400.233456298S
fn main() {


    //test_optimized_interpreter();
    let start = PreciseTime::now();
    test_optimized_interpreter();
    
    let end = PreciseTime::now();
    println!("Duration: {}", start.to(end));
    
}

