
pub mod x64 {
    pub enum Opcode{
        Mov,
        Ret,
    
    
    }
    
    pub enum Operand {
        None,
    
    
    }
    
}



pub struct Emitter {
    unused: u32,


}

impl Emitter{
    pub fn new() -> Emitter {
        
        Emitter{ unused: 0}
    }


    pub fn emit(&self, op:x64::Opcode, oprnd:x64::Operand) -> i32 {
        
        use self::x64::Opcode::*;
        use self::x64::Operand::*;
        let size:i32;
        match (op, oprnd) {
            (Ret, None) => {println!("c3"); size = 1;},
            //(Ret,    ) => {println!("Invalid instruction.");}
        
        
            (_,_) => {println!("Unimplemented"); size = -1;},
        };
        
        size
    }
}