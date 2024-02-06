use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    mem::size_of,
    ops::{Index, IndexMut},
    ptr,
};

use uefi::table::boot::MemoryMap;

use crate::{bitfield::BitField, memory_map};

/// グローバルアロケータ。
#[global_allocator]
pub(crate) static GLOBAL: BitmapMemoryManager = BitmapMemoryManager::new();

const KIB: usize = 1024;
const MIB: usize = 1024 * KIB;
const GIB: usize = 1024 * MIB;

/// 1フレームで取り扱うメモリのサイズ。
pub(crate) const BYTES_PER_FRAME: usize = 4 * KIB;

/// フレームを表す構造体。
struct FrameId {
    id: usize,
}

impl FrameId {
    /// ID から [FrameId] を作る。
    pub(crate) const fn new(id: usize) -> Self {
        Self { id }
    }

    pub(crate) fn from_addr(addr: usize) -> Self {
        Self {
            id: addr / BYTES_PER_FRAME,
        }
    }

    /// ID を取得する。
    pub(crate) fn id(&self) -> usize {
        self.id
    }

    /// フレームの先頭へのポインタ。
    pub(crate) fn frame(&self) -> *mut u8 {
        (self.id * BYTES_PER_FRAME) as *mut u8
    }
}

/// ビットマップ配列の要素型。
type MapLineType = usize;

/// [BitmapMemoryManager] で管理できる最大の物理メモリのサイズ。
const BITS_PER_MAP_LINE: usize = 8 * size_of::<MapLineType>();

/// [MAX_PHYSICAL_MEMORY] のメモリを管理するのに必要なフレーム数。
const MAX_PHYSICAL_MEMORY: usize = 128 * GIB;

/// ビットマップ配列の1要素のビット数 == 1要素で扱えるフレーム数
const FRAME_COUNT: usize = MAX_PHYSICAL_MEMORY / BYTES_PER_FRAME;

const UEFI_PAGE_SIZE: usize = 4 * KIB;

pub(crate) struct BitmapMemoryManager {
    alloc_map: UnsafeCell<[MapLineType; FRAME_COUNT / BITS_PER_MAP_LINE]>,
    range_begin: UnsafeCell<FrameId>,
    range_end: UnsafeCell<FrameId>,
    is_initialized: UnsafeCell<bool>,
}

impl BitmapMemoryManager {
    /// [BitmapMemoryManager] を作る。
    const fn new() -> Self {
        Self {
            alloc_map: UnsafeCell::new([0; FRAME_COUNT / BITS_PER_MAP_LINE]),
            range_begin: UnsafeCell::new(FrameId::new(0)),
            range_end: UnsafeCell::new(FrameId::new(0)),
            is_initialized: UnsafeCell::new(false),
        }
    }

    pub(crate) fn initialize(&self, memory_map: &MemoryMap) {
        if unsafe { *self.is_initialized.get() } {
            return;
        }

        let mut available_end = 0;
        for desc in memory_map.entries() {
            if available_end < desc.phys_start as usize {
                self.mark_allocated(
                    FrameId::from_addr(available_end),
                    get_num_frames(desc.phys_start as usize - available_end),
                );
            }

            let phys_end = desc.phys_start as usize + desc.page_count as usize * UEFI_PAGE_SIZE;
            if memory_map::is_available(desc.ty) {
                available_end = phys_end;
            } else {
                // REVIEW: 以下のコメントアウトは要らないと思うが……
                // self.mark_allocated(
                //     FrameId::from_addr(desc.phys_start as usize),
                //     get_num_frames(desc.page_count as usize * UEFI_PAGE_SIZE),
                // );
            }
        }

        unsafe {
            *self.range_begin.get() = FrameId::new(1);
            *self.range_end.get() = FrameId::from_addr(available_end);
            *self.is_initialized.get() = true;
        }
    }

    fn mark_allocated(&self, start_frame: FrameId, num_frames: usize) {
        // OPTIMIZE: まとめてセットできるようにした方が良い
        for i in 0..num_frames {
            self.set_bit(FrameId::new(start_frame.id() + i), true);
        }
    }

    fn is_allocated(&self, frame: FrameId) -> bool {
        let line_index = frame.id() / BITS_PER_MAP_LINE;
        let bit_index = frame.id() % BITS_PER_MAP_LINE;

        unsafe { &*self.alloc_map.get() }
            .index(line_index)
            .get_bit(bit_index as u32)
    }

    fn set_bit(&self, frame: FrameId, allocated: bool) {
        let line_index = frame.id() / BITS_PER_MAP_LINE;
        let bit_index = frame.id() % BITS_PER_MAP_LINE;

        let map = unsafe { &mut *self.alloc_map.get() }.index_mut(line_index);
        map.set_bit(bit_index as u32, allocated);
    }
}

// FIXME: これは今は大丈夫だが、マルチタスクが始まったら問題になる。
unsafe impl Send for BitmapMemoryManager {}
unsafe impl Sync for BitmapMemoryManager {}

unsafe impl GlobalAlloc for BitmapMemoryManager {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !*self.is_initialized.get() {
            return ptr::null_mut();
        }

        let num_frames = get_num_frames(layout.size());

        // フレームで見たときのアラインメント
        let align_frames = layout.align() / BYTES_PER_FRAME;
        let align_frames = if align_frames == 0 { 1 } else { align_frames };

        let mut start_frame_id = (*self.range_begin.get()).id();
        loop {
            // アラインメント調整
            let res = start_frame_id % align_frames;
            if res != 0 {
                start_frame_id += align_frames - res;
            }

            let mut i = 0;
            while i < num_frames {
                if start_frame_id + i >= (*self.range_end.get()).id() {
                    return ptr::null_mut();
                }
                if self.is_allocated(FrameId::new(start_frame_id + i)) {
                    break;
                }
                i += 1;
            }

            if i == num_frames {
                self.mark_allocated(FrameId::new(start_frame_id), num_frames);
                return (start_frame_id * BYTES_PER_FRAME) as *mut u8;
            }

            start_frame_id += i + 1;
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let start_frame = FrameId::from_addr(ptr as usize);
        let num_frames = get_num_frames(layout.size());

        for i in 0..num_frames {
            self.set_bit(FrameId::new(start_frame.id() + i), false);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_num_frames = get_num_frames(layout.size());
        let new_num_frames = get_num_frames(new_size);
        if new_num_frames == old_num_frames {
            ptr
        } else if new_num_frames < old_num_frames {
            let de_size = (old_num_frames - new_num_frames) * BYTES_PER_FRAME;
            let de_ptr = ptr.add(de_size);
            let de_layout = Layout::from_size_align_unchecked(de_size, layout.align());
            self.dealloc(de_ptr, de_layout);
            ptr
        } else {
            let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
            let new_ptr = self.alloc(new_layout);
            ptr::copy_nonoverlapping(ptr, new_ptr, layout.size());
            self.dealloc(ptr, layout);
            new_ptr
        }
    }
}

fn get_num_frames(size: usize) -> usize {
    (size + BYTES_PER_FRAME - 1) / BYTES_PER_FRAME
}
