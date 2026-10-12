//! Bun allocates with mimalloc, which is written in C. These are the functions
//! of it that Bun's crates call, on top of Rust's allocator.
//!
//! What matters to Bun's arenas is that a heap frees all of its blocks at once,
//! so the blocks of a heap are in a list.

use std::alloc::Layout;
use std::ffi::c_void;
use std::ptr;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

/// `mi_malloc`
#[unsafe(no_mangle)]
pub extern "C" fn mi_malloc(size: usize) -> *mut c_void {
  allocate(ptr::null_mut(), size, MIN_ALIGN, false)
}

/// `mi_zalloc`
#[unsafe(no_mangle)]
pub extern "C" fn mi_zalloc(size: usize) -> *mut c_void {
  allocate(ptr::null_mut(), size, MIN_ALIGN, true)
}

/// `mi_calloc`
#[unsafe(no_mangle)]
pub extern "C" fn mi_calloc(count: usize, size: usize) -> *mut c_void {
  match count.checked_mul(size) {
    Some(size) => allocate(ptr::null_mut(), size, MIN_ALIGN, true),
    None => ptr::null_mut(),
  }
}

/// `mi_malloc_aligned`
#[unsafe(no_mangle)]
pub extern "C" fn mi_malloc_aligned(size: usize, alignment: usize) -> *mut c_void {
  allocate(ptr::null_mut(), size, alignment, false)
}

/// `mi_zalloc_aligned`
#[unsafe(no_mangle)]
pub extern "C" fn mi_zalloc_aligned(size: usize, alignment: usize) -> *mut c_void {
  allocate(ptr::null_mut(), size, alignment, true)
}

/// `mi_realloc`
///
/// # Safety
/// `block` has to be null or a live block of this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_realloc(block: *mut c_void, new_size: usize) -> *mut c_void {
  // SAFETY: the caller guarantees the block
  unsafe { reallocate(None, block, new_size, MIN_ALIGN) }
}

/// `mi_realloc_aligned`
///
/// # Safety
/// `block` has to be null or a live block of this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_realloc_aligned(block: *mut c_void, new_size: usize, alignment: usize) -> *mut c_void {
  // SAFETY: the caller guarantees the block
  unsafe { reallocate(None, block, new_size, alignment) }
}

/// `mi_expand`, which grows a block where it is or fails. It always fails here.
///
/// # Safety
/// None.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_expand(_block: *mut c_void, _new_size: usize) -> *mut c_void {
  ptr::null_mut()
}

/// `mi_free`
///
/// # Safety
/// `block` has to be null or a live block of this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_free(block: *mut c_void) {
  // SAFETY: the caller guarantees the block
  unsafe { release(block) }
}

/// `mi_free_size`
///
/// # Safety
/// `block` has to be null or a live block of this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_free_size(block: *mut c_void, _size: usize) {
  // SAFETY: the caller guarantees the block
  unsafe { release(block) }
}

/// `mi_free_size_aligned`
///
/// # Safety
/// `block` has to be null or a live block of this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_free_size_aligned(block: *mut c_void, _size: usize, _alignment: usize) {
  // SAFETY: the caller guarantees the block
  unsafe { release(block) }
}

/// `mi_usable_size`
///
/// # Safety
/// `block` has to be null or a live block of this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_usable_size(block: *const c_void) -> usize {
  if block.is_null() {
    return 0;
  }
  // SAFETY: the header comes before a live block
  unsafe { (*header_of(block.cast_mut())).size }
}

/// `mi_malloc_usable_size`
///
/// # Safety
/// `block` has to be null or a live block of this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_malloc_usable_size(block: *const c_void) -> usize {
  // SAFETY: the caller guarantees the block
  unsafe { mi_usable_size(block) }
}

/// `mi_heap_new`
#[unsafe(no_mangle)]
pub extern "C" fn mi_heap_new() -> *mut Heap {
  let heap = Box::into_raw(Box::new(Heap {
    blocks: Header {
      size: 0,
      prev: ptr::null_mut(),
      next: ptr::null_mut(),
      owner: 0,
    },
  }));
  // SAFETY: the heap was allocated above, and an empty list is its sentinel alone
  unsafe {
    let sentinel = &raw mut (*heap).blocks;
    (*sentinel).prev = sentinel;
    (*sentinel).next = sentinel;
  }
  heap
}

/// `mi_heap_main`: the heap of what is allocated without one. Its blocks are
/// not freed together, so they are not in a list.
#[unsafe(no_mangle)]
pub extern "C" fn mi_heap_main() -> *mut Heap {
  main_heap()
}

/// `mi_heap_destroy`: frees the heap and every block that is still in it.
///
/// # Safety
/// `heap` has to be a live heap of `mi_heap_new`, and none of its blocks may
/// be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_heap_destroy(heap: *mut Heap) {
  if heap.is_null() || heap == main_heap() {
    return;
  }
  let _guard = lock();
  // SAFETY: the caller guarantees the heap, whose list has only live blocks
  unsafe {
    let sentinel = &raw mut (*heap).blocks;
    let mut header = (*sentinel).next;
    while header != sentinel {
      let next = (*header).next;
      deallocate(header);
      header = next;
    }
    drop(Box::from_raw(heap));
  }
}

/// `mi_heap_malloc`
///
/// # Safety
/// `heap` has to be a live heap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_heap_malloc(heap: *mut Heap, size: usize) -> *mut c_void {
  allocate(heap, size, MIN_ALIGN, false)
}

/// `mi_heap_zalloc`
///
/// # Safety
/// `heap` has to be a live heap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_heap_zalloc(heap: *mut Heap, size: usize) -> *mut c_void {
  allocate(heap, size, MIN_ALIGN, true)
}

/// `mi_heap_malloc_aligned`
///
/// # Safety
/// `heap` has to be a live heap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_heap_malloc_aligned(heap: *mut Heap, size: usize, alignment: usize) -> *mut c_void {
  allocate(heap, size, alignment, false)
}

/// `mi_heap_zalloc_aligned`
///
/// # Safety
/// `heap` has to be a live heap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_heap_zalloc_aligned(heap: *mut Heap, size: usize, alignment: usize) -> *mut c_void {
  allocate(heap, size, alignment, true)
}

/// `mi_heap_realloc`
///
/// # Safety
/// `heap` has to be a live heap, and `block` null or a live block of this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_heap_realloc(heap: *mut Heap, block: *mut c_void, new_size: usize) -> *mut c_void {
  // SAFETY: the caller guarantees the heap and the block
  unsafe { reallocate(Some(heap), block, new_size, MIN_ALIGN) }
}

/// `mi_heap_realloc_aligned`
///
/// # Safety
/// `heap` has to be a live heap, and `block` null or a live block of this allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_heap_realloc_aligned(
  heap: *mut Heap,
  block: *mut c_void,
  new_size: usize,
  alignment: usize,
) -> *mut c_void {
  // SAFETY: the caller guarantees the heap and the block
  unsafe { reallocate(Some(heap), block, new_size, alignment) }
}

/// `mi_is_in_heap_region`: every pointer that Bun asks about is a block.
///
/// # Safety
/// None.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mi_is_in_heap_region(block: *const c_void) -> bool {
  !block.is_null()
}

/// `mi_collect`: Rust's allocator has nothing to give back.
#[unsafe(no_mangle)]
pub extern "C" fn mi_collect(_force: bool) {}

/// `mi_option_set`: there are no options.
#[unsafe(no_mangle)]
pub extern "C" fn mi_option_set(_option: u32, _value: core::ffi::c_long) {}

/// `mi_heap_t`. Its alignment leaves the low bits of a pointer to it free,
/// which is where a block keeps its own alignment.
#[repr(C, align(64))]
pub struct Heap {
  /// The sentinel of the list of the blocks.
  blocks: Header,
}

/// What comes right before every block.
#[repr(C)]
struct Header {
  /// The size that was asked for.
  size: usize,
  /// The neighbors in the list of the heap, or null when the block is in none.
  prev: *mut Header,
  next: *mut Header,
  /// The heap that the block is in, or zero, with the binary logarithm of the
  /// alignment of the allocation in the low bits.
  owner: usize,
}

const HEADER_SIZE: usize = size_of::<Header>();
/// What mimalloc aligns every block to (`MI_MAX_ALIGN_SIZE`).
const MIN_ALIGN: usize = 16;
const ALIGN_BITS: usize = 0b11_1111;

impl Header {
  fn new(size: usize, align: usize, heap: *mut Heap) -> Header {
    Header {
      size,
      prev: ptr::null_mut(),
      next: ptr::null_mut(),
      owner: heap as usize | align.trailing_zeros() as usize,
    }
  }

  fn align(&self) -> usize {
    1 << (self.owner & ALIGN_BITS)
  }

  fn heap(&self) -> *mut Heap {
    (self.owner & !ALIGN_BITS) as *mut Heap
  }

  /// How far the block is from the start of its allocation.
  fn offset(&self) -> usize {
    offset_for(self.align())
  }

  fn layout(&self) -> Layout {
    // SAFETY: it was a valid layout when the block was allocated
    unsafe { Layout::from_size_align_unchecked(self.size + self.offset(), self.align()) }
  }
}

fn offset_for(align: usize) -> usize {
  HEADER_SIZE.next_multiple_of(align)
}

fn allocate(heap: *mut Heap, size: usize, align: usize, zeroed: bool) -> *mut c_void {
  let align = align.max(MIN_ALIGN);
  let offset = offset_for(align);
  let Some(layout) = size
    .checked_add(offset)
    .and_then(|total| Layout::from_size_align(total, align).ok())
  else {
    return ptr::null_mut();
  };
  // SAFETY: the layout has the size of the header at least
  let base = unsafe {
    match zeroed {
      true => std::alloc::alloc_zeroed(layout),
      false => std::alloc::alloc(layout),
    }
  };
  if base.is_null() {
    return ptr::null_mut();
  }
  let heap = if heap == main_heap() { ptr::null_mut() } else { heap };
  // SAFETY: the allocation has room for the header before the block
  unsafe {
    let block = base.add(offset);
    let header = header_of(block.cast());
    header.write(Header::new(size, align, heap));
    if !heap.is_null() {
      let _guard = lock();
      link(heap, header);
    }
    block.cast()
  }
}

/// `heap`: where the block has to be afterwards, or `None` for where it is.
unsafe fn reallocate(heap: Option<*mut Heap>, block: *mut c_void, new_size: usize, align: usize) -> *mut c_void {
  let wanted_heap = heap.map(|heap| if heap == main_heap() { ptr::null_mut() } else { heap });
  if block.is_null() {
    return allocate(wanted_heap.unwrap_or(ptr::null_mut()), new_size, align, false);
  }
  // SAFETY: the caller guarantees a live block, which has a header
  unsafe {
    let header = header_of(block);
    let old = header.read();
    let stays = wanted_heap.is_none_or(|heap| heap == old.heap()) && align <= old.align();
    if !stays {
      let new_block = allocate(wanted_heap.unwrap_or(old.heap()), new_size, align, false);
      if !new_block.is_null() {
        ptr::copy_nonoverlapping(block.cast::<u8>(), new_block.cast::<u8>(), old.size.min(new_size));
        release(block);
      }
      return new_block;
    }
    let offset = old.offset();
    let Some(new_total) = new_size.checked_add(offset) else {
      return ptr::null_mut();
    };
    if Layout::from_size_align(new_total, old.align()).is_err() {
      return ptr::null_mut();
    }
    // the neighbors point at the header, which moves
    let _guard = lock();
    let new_base = std::alloc::realloc(block.cast::<u8>().sub(offset), old.layout(), new_total);
    if new_base.is_null() {
      return ptr::null_mut();
    }
    let new_block = new_base.add(offset);
    let new_header = header_of(new_block.cast());
    (*new_header).size = new_size;
    if !old.prev.is_null() {
      (*old.prev).next = new_header;
      (*old.next).prev = new_header;
    }
    new_block.cast()
  }
}

unsafe fn release(block: *mut c_void) {
  if block.is_null() {
    return;
  }
  // SAFETY: the caller guarantees a live block, which has a header
  unsafe {
    let header = header_of(block);
    if !(*header).prev.is_null() {
      let _guard = lock();
      (*(*header).prev).next = (*header).next;
      (*(*header).next).prev = (*header).prev;
    }
    deallocate(header);
  }
}

/// Frees the allocation of a block that is in no list anymore.
unsafe fn deallocate(header: *mut Header) {
  // SAFETY: the caller guarantees the header of a live block
  unsafe {
    let layout = (*header).layout();
    let base = header.add(1).cast::<u8>().sub((*header).offset());
    std::alloc::dealloc(base, layout);
  }
}

/// Puts a block at the end of the list of a heap.
unsafe fn link(heap: *mut Heap, header: *mut Header) {
  // SAFETY: the caller guarantees the heap and the header, and holds the lock
  unsafe {
    let sentinel = &raw mut (*heap).blocks;
    let last = (*sentinel).prev;
    (*header).prev = last;
    (*header).next = sentinel;
    (*last).next = header;
    (*sentinel).prev = header;
  }
}

unsafe fn header_of(block: *mut c_void) -> *mut Header {
  // SAFETY: the caller guarantees a block, which comes after its header
  unsafe { block.cast::<Header>().sub(1) }
}

fn main_heap() -> *mut Heap {
  static MAIN: OnceLock<usize> = OnceLock::new();
  *MAIN.get_or_init(|| mi_heap_new() as usize) as *mut Heap
}

/// A block can be freed on another thread than the one of its heap.
fn lock() -> MutexGuard<'static, ()> {
  static LOCK: Mutex<()> = Mutex::new(());
  LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
