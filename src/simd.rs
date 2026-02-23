//! SIMD vectorization for array operations (int[], float[], etc.).
//!
//! Provides accelerated array copy and fill using portable SIMD.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::arch::x86_64::*;

/// SIMD-enabled array copy for int[] - copies src to dst using vectorized loads/stores
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub unsafe fn array_copy_int_simd(dst: &mut [i32], src: &[i32], len: usize) {
    let chunks = len / 8; // 8 x i32 = 256 bits
    for i in 0..chunks {
        let idx = i * 8;
        let v = _mm256_loadu_si256(src.as_ptr().add(idx) as *const __m256i);
        _mm256_storeu_si256(dst.as_mut_ptr().add(idx) as *mut __m256i, v);
    }
    // Scalar remainder
    for i in (chunks * 8)..len {
        dst[i] = src[i];
    }
}

/// Fallback scalar array copy when SIMD not available
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
pub fn array_copy_int_simd(dst: &mut [i32], src: &[i32], len: usize) {
    let end = len.min(dst.len()).min(src.len());
    dst[..end].copy_from_slice(&src[..end]);
}

/// SIMD-enabled array copy for float[] - copies src to dst
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx")]
pub unsafe fn array_copy_float_simd(dst: &mut [f32], src: &[f32], len: usize) {
    let chunks = len / 8;
    for i in 0..chunks {
        let idx = i * 8;
        let v = _mm256_loadu_ps(src.as_ptr().add(idx));
        _mm256_storeu_ps(dst.as_mut_ptr().add(idx), v);
    }
    for i in (chunks * 8)..len {
        dst[i] = src[i];
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
pub fn array_copy_float_simd(dst: &mut [f32], src: &[f32], len: usize) {
    let end = len.min(dst.len()).min(src.len());
    dst[..end].copy_from_slice(&src[..end]);
}

/// Copy within HeapArray int[] using SIMD when available
pub fn heap_array_copy_int(dst: &mut [i32], src: &[i32], src_pos: usize, dst_pos: usize, len: usize) {
    let copy_len = len.min(src.len().saturating_sub(src_pos))
        .min(dst.len().saturating_sub(dst_pos));
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if copy_len >= 8 && is_x86_feature_detected!("avx2") {
            unsafe {
                array_copy_int_simd(
                    &mut dst[dst_pos..],
                    &src[src_pos..],
                    copy_len,
                );
            }
        } else {
            dst[dst_pos..dst_pos + copy_len]
                .copy_from_slice(&src[src_pos..src_pos + copy_len]);
        }
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        dst[dst_pos..dst_pos + copy_len].copy_from_slice(&src[src_pos..src_pos + copy_len]);
    }
}

/// Copy within HeapArray float[] using SIMD when available
pub fn heap_array_copy_float(dst: &mut [f32], src: &[f32], src_pos: usize, dst_pos: usize, len: usize) {
    let copy_len = len.min(src.len().saturating_sub(src_pos))
        .min(dst.len().saturating_sub(dst_pos));
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if copy_len >= 8 && is_x86_feature_detected!("avx") {
            unsafe {
                array_copy_float_simd(&mut dst[dst_pos..], &src[src_pos..], copy_len);
            }
        } else {
            dst[dst_pos..dst_pos + copy_len]
                .copy_from_slice(&src[src_pos..src_pos + copy_len]);
        }
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        dst[dst_pos..dst_pos + copy_len].copy_from_slice(&src[src_pos..src_pos + copy_len]);
    }
}
