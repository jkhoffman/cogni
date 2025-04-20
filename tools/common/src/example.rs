// Example showing correct usage of NonZero types

use nonzero_ext::nonzero;
use std::num::{NonZeroU32, NonZeroU64}; // This crate provides NonZeroU* constructors

fn main() {
    // NonZeroU64 from the standard library
    let value_u64 = 42u64;
    let non_zero_u64 = NonZeroU64::new(value_u64).expect("Value should not be zero");

    // NonZeroU64 with the nonzero_ext crate
    let non_zero_u64_ext = nonzero!(42u64);

    // There is no NonZeroF32 in std, but you can use f64 values instead
    let value_f64 = 3.14_f64;
    let non_zero_f64 = std::num::NonZeroF64::new(value_f64).expect("Value should not be zero");

    // Or create a NonZeroF32 wrapper struct
    #[derive(Debug, Clone, Copy)]
    struct NonZeroF32(f32);

    impl NonZeroF32 {
        pub fn new(value: f32) -> Option<Self> {
            if value != 0.0 {
                Some(NonZeroF32(value))
            } else {
                None
            }
        }

        pub fn get(&self) -> f32 {
            self.0
        }
    }

    let value_f32 = 2.71_f32;
    let non_zero_f32 = NonZeroF32::new(value_f32).expect("Value should not be zero");

    println!("NonZeroU64: {:?}", non_zero_u64);
    println!("NonZeroU64 (ext): {:?}", non_zero_u64_ext);
    println!("NonZeroF64: {:?}", non_zero_f64);
    println!("Custom NonZeroF32: {:?}", non_zero_f32);
}
