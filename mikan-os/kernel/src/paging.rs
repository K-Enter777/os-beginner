use core::ops::{Index, IndexMut};

use crate::asmfunc::set_cr3;

pub(crate) const PAGE_DIRECTORY_COUNT: usize = 64;

const PAGE_SIZE_4K: u64 = 4096;
const PAGE_SIZE_2M: u64 = 512 * PAGE_SIZE_4K;
const PAGE_SIZE_1G: u64 = 512 * PAGE_SIZE_2M;

static mut PML4_TABLE: PageTable<u64, 512> = PageTable::<_, 512>::new(0);
static mut PDP_TABLE: PageTable<u64, 512> = PageTable::<_, 512>::new(0);
static mut PAGE_DIRECTORY: PageTable<[u64; 512], PAGE_DIRECTORY_COUNT> =
    PageTable::<_, PAGE_DIRECTORY_COUNT>::new([0; 512]);

#[repr(align(4096))]
struct PageTable<T, const N: usize> {
    table: [T; N],
}

impl<T, const N: usize> PageTable<T, N> {
    fn len(&self) -> usize {
        N
    }

    fn as_ptr(&self) -> *const T {
        self.table.as_ptr()
    }
}

impl<T: Copy, const N: usize> PageTable<T, N> {
    const fn new(value: T) -> Self {
        Self { table: [value; N] }
    }
}

impl<T, const N: usize> Index<usize> for PageTable<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.table[index]
    }
}

impl<T, const N: usize> IndexMut<usize> for PageTable<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.table[index]
    }
}

pub(crate) fn setup_indentity_page_table() {
    unsafe {
        PML4_TABLE[0] = PDP_TABLE.as_ptr() as u64 | 0x003;

        for i_pdpt in 0..PAGE_DIRECTORY.len() {
            PDP_TABLE[i_pdpt] = PAGE_DIRECTORY[i_pdpt].as_ptr() as u64 | 0x003;

            for i_pd in 0..PAGE_DIRECTORY[0].len() {
                PAGE_DIRECTORY[i_pdpt][i_pd] =
                    i_pdpt as u64 * PAGE_SIZE_1G + i_pd as u64 * PAGE_SIZE_2M | 0x083;
            }
        }

        set_cr3(PML4_TABLE.as_ptr() as u64);
    }
}
