
extern crate libc;

#[cfg(windows)] extern crate kernel32;
#[cfg(windows)] extern crate winapi;

use std::io::{Write, Error, ErrorKind, Cursor};
use std::ops::{Index, IndexMut};



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
    
    fn get_size(&self) -> u32 {
        self.size
    }
    
    fn position(&self) -> isize {
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
fn main() {
    
    
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
    
    //mov rax, 0x3
    code_buff.write::<u8>(0x48);
    code_buff.write::<u8>(0xc7);
    code_buff.write::<u8>(0xc0);
    code_buff.write::<u32>(0x00000003);
    
    e.emit(x64::Opcode::Inc, x64::Operand::Register(x64::Register::Reg64(x64::Reg64::Rax)), &mut code_buff);
    e.emit(x64::Opcode::Ret, x64::Operand::None, &mut code_buff);

    
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
}

