
#[repr(C)]
pub struct Pot {
    pub pointer: usize,
}

#[no_mangle]
pub extern "C" fn test_create() -> Pot {
    println!("[Rust]: Creating Pot");
    Pot {
        pointer: 987654321987654321,
    }
}

#[no_mangle]
pub extern "C" fn test_mut_star(pot: *mut Pot)  {
    println!("[Rust]: test_mut_star");
    let pot = unsafe { &mut *pot };
    println!("[Rust]: *mut {}", pot.pointer);
}

#[no_mangle]
pub extern "C" fn test_value(pot: Pot)  {
    println!("[Rust]: value {}", pot.pointer);
}
