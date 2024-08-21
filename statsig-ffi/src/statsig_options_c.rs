use statsig::statsig_options::StatsigOptionsBuilder;
use statsig::StatsigOptions;

#[repr(C)]
pub struct StatsigOptionsRef {
    pub pointer: usize,
}

impl StatsigOptionsRef {
    pub fn to_internal(&self) -> Option<&StatsigOptions> {
        if self.pointer == 0 {
            println!("Failed to fetch StatsigOptions. Reference has been released");
            return None;
        }

        Some(unsafe { &*(self.pointer as *mut StatsigOptions) })
    }
}

#[no_mangle]
pub extern "C" fn statsig_options_create() -> StatsigOptionsRef {
    let instance = Box::new(StatsigOptions::new() );
    let pointer = Box::into_raw(instance) as usize;
    println!("Created StatsigOptions {}", pointer);

    StatsigOptionsRef {
        pointer,
    }
}

#[no_mangle]
pub extern "C" fn statsig_options_release(options_ref: *mut StatsigOptionsRef) {
    let ref_obj = unsafe { &mut *options_ref };
    println!("Releasing StatsigOptions {}", ref_obj.pointer);

    if ref_obj.pointer != 0 {
        unsafe { drop(Box::from_raw(ref_obj.pointer as *mut StatsigOptions)) };
        ref_obj.pointer = 0;
        println!("StatsigOptions released.");
    } else {
        println!("Warn: StatsigOptions already released.");
    }
}
