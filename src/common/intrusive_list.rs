pub trait IntrusiveLink<NodeType>
where NodeType: Copy + IntrusiveLink<NodeType> {
    unsafe fn next(&self) -> *const NodeType;

    unsafe fn next_mut(&mut self) -> *mut NodeType {
        self.next() as *mut NodeType
    }

    unsafe fn insert_node(&mut self, mut node: NodeType) {
        *node.next_mut() = *self.next();
        *self.next_mut() = node;
    }
}

#[derive(Clone, Copy)]
pub struct IntrusiveList<NodeType>
where NodeType: Copy + IntrusiveLink<NodeType> {
    pub storage: *const Option<NodeType>,
    pub capacity: usize,
    pub start: *const NodeType,
    pub current: *const NodeType
}

impl<NodeType: Copy + IntrusiveLink<NodeType>> IntrusiveList<NodeType> {
    pub unsafe fn get_first(&self) -> *const NodeType {
        self.start
    }

    pub unsafe fn get_first_mut(&mut self) -> *mut NodeType {
        self.start as *mut NodeType
    }
}

impl<NodeType: Copy + IntrusiveLink<NodeType>> Iterator for IntrusiveList<NodeType> {
    type Item = *const NodeType;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.current;

        unsafe {
            if let Some(current) = node.as_ref() {
                self.current = current.next();

                Some(node)
            } else {
                self.current = self.start;

                None
            }
        }
    }
}
