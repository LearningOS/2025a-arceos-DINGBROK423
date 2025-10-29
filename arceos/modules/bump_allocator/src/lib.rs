#![no_std]

use allocator::{AllocError, AllocResult, BaseAllocator, ByteAllocator, PageAllocator};
use core::alloc::Layout;
use core::ptr::NonNull;

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const SIZE: usize> {
    start: usize,
    end: usize,
    b_pos: usize,   // Current position for byte allocation (growing forward)
    p_pos: usize,   // Current position for page allocation (growing backward)
    b_count: usize, // Number of byte allocations
    p_used: usize,  // Number of pages used
}

impl<const SIZE: usize> EarlyAllocator<SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            p_pos: 0,
            b_count: 0,
            p_used: 0,
        }
    }
    
    /// Align up to the given alignment
    #[inline]
    const fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }
    
    /// Align down to the given alignment
    #[inline]
    const fn align_down(addr: usize, align: usize) -> usize {
        addr & !(align - 1)
    }
}

impl<const SIZE: usize> BaseAllocator for EarlyAllocator<SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.b_pos = start;
        self.p_pos = self.end;
        self.b_count = 0;
        self.p_used = 0;
    }

    fn add_memory(&mut self, _start: usize, _size: usize) -> AllocResult {
        // Early allocator doesn't support adding memory
        Err(AllocError::NoMemory)
    }
}

impl<const SIZE: usize> ByteAllocator for EarlyAllocator<SIZE> {
    fn alloc(
        &mut self,
        layout: Layout,
    ) -> AllocResult<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();
        
        // Align the current position
        let alloc_start = Self::align_up(self.b_pos, align);
        let alloc_end = alloc_start.checked_add(size).ok_or(AllocError::NoMemory)?;
        
        // Check if there's enough space (must not overlap with page area)
        if alloc_end > self.p_pos {
            return Err(AllocError::NoMemory);
        }
        
        // Update the byte position and count
        self.b_pos = alloc_end;
        self.b_count += 1;
        
        // Return the allocated pointer
        NonNull::new(alloc_start as *mut u8).ok_or(AllocError::NoMemory)
    }

    fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {
        // Decrease the allocation count
        if self.b_count > 0 {
            self.b_count -= 1;
        }
        
        // When count goes to zero, free the entire bytes-used area
        if self.b_count == 0 {
            self.b_pos = self.start;
        }
    }

    fn total_bytes(&self) -> usize {
        self.end - self.start
    }

    fn used_bytes(&self) -> usize {
        (self.b_pos - self.start) + (self.end - self.p_pos)
    }

    fn available_bytes(&self) -> usize {
        if self.p_pos > self.b_pos {
            self.p_pos - self.b_pos
        } else {
            0
        }
    }
}

impl<const SIZE: usize> PageAllocator for EarlyAllocator<SIZE> {
    const PAGE_SIZE: usize = SIZE;

    fn alloc_pages(
        &mut self,
        num_pages: usize,
        align_pow2: usize,
    ) -> AllocResult<usize> {
        let alloc_size = num_pages * SIZE;
        
        // Allocate from the back, so we need to move p_pos backward
        let alloc_end = self.p_pos;
        let alloc_start = alloc_end.checked_sub(alloc_size).ok_or(AllocError::NoMemory)?;
        
        // Align down the start address to meet alignment requirement
        let aligned_start = Self::align_down(alloc_start, align_pow2);
        
        // Check if there's enough space (must not overlap with byte area)
        if aligned_start < self.b_pos {
            return Err(AllocError::NoMemory);
        }
        
        // Update the page position and page count
        self.p_pos = aligned_start;
        self.p_used += num_pages;
        
        Ok(aligned_start)
    }

    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // According to the specification, pages will never be freed
        // Do nothing
    }

    fn total_pages(&self) -> usize {
        (self.end - self.start) / SIZE
    }

    fn used_pages(&self) -> usize {
        self.p_used
    }

    fn available_pages(&self) -> usize {
        if self.p_pos > self.b_pos {
            (self.p_pos - self.b_pos) / SIZE
        } else {
            0
        }
    }
}