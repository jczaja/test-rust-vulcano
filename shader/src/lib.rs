#![cfg_attr(target_arch = "spirv", no_std)]

use glam::UVec3;
use spirv_std::{glam, spirv};

// Adapted from the wgpu hello-compute example

pub fn collatz(n: u32) -> Option<u32> {
    if n == 0 || n == 1 {
        return None;
    }
    let mut divisor : u32 = n/2;

    while n % divisor != 0 {
       divisor-=1; 
    }

    if divisor == 1 {
        Some(n)
    } else {
        None
    }
}

// LocalSize/numthreads of (x = 64, y = 1, z = 1)
#[spirv(compute(threads(768)))]
pub fn main_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] prime_indices: &mut [u32],
) {
    let index = id.x as usize;
    prime_indices[index] = collatz(prime_indices[index]).unwrap_or(1u32);
}
