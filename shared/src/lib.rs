pub mod loader;
pub mod object;
pub mod process;

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct IndexedTriangle([usize; 3]);

impl std::ops::Index<usize> for IndexedTriangle {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for IndexedTriangle {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

pub struct IndexedLine([usize; 2]);

impl std::ops::Index<usize> for IndexedLine {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for IndexedLine {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

pub struct SliceInput<M> {
    pub objects: Vec<object::ObjectMesh>,
    pub masks: Vec<M>,
}
