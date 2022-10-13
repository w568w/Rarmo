type BitmapCell = u64;

const BITMAP_BITS_PER_CELL: usize = core::mem::size_of::<BitmapCell>() * 8;

pub struct Bitmap<const cell_num: usize> {
    cells: [BitmapCell; cell_num],
}

pub const fn cell(num: usize) -> usize {
    (num + BITMAP_BITS_PER_CELL - 1) / BITMAP_BITS_PER_CELL
}

const fn get_cell_index_and_offset(index: usize) -> (usize, usize) {
    (index / BITMAP_BITS_PER_CELL, index % BITMAP_BITS_PER_CELL)
}

impl<const size: usize> Bitmap<size> {
    pub const fn new() -> Self {
        Self {
            cells: [0; size],
        }
    }

    pub fn set(&mut self, index: usize) {
        let (cell_index, offset) = get_cell_index_and_offset(index);
        self.cells[cell_index] |= 1 << offset;
    }

    pub fn clear(&mut self, index: usize) {
        let (cell_index, offset) = get_cell_index_and_offset(index);
        self.cells[cell_index] &= !(1 << offset);
    }

    pub fn get(&self, index: usize) -> bool {
        let (cell_index, offset) = get_cell_index_and_offset(index);
        self.cells[cell_index] & (1 << offset) != 0
    }
}