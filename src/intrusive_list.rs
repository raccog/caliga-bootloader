pub struct IntrusiveList<DataType: PartialOrd> {
    storage: *const DataType,
    capacity: usize,
    node_start: *const DataType,
}

impl<DataType: PartialOrd> IntrusiveList<DataType> {
    pub const unsafe fn new(storage: *const DataType, capacity: usize) -> Self {
        Self {
            storage,
            capacity,
            node_start: storage,
        }
    }

    pub unsafe fn get_first(&mut self) -> &DataType {
        &*self.node_start
    }

    pub unsafe fn replace_first(&mut self, replacement: DataType) {
        *(self.node_start as *mut DataType) = replacement;
    }
}

pub struct IntrusiveNode<DataType> {
    next: *const DataType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert() {
        use std::vec::Vec;
        use std::cmp::Ordering;

        struct MockDataType {
            pub data: i32,
            node: Option<IntrusiveNode<MockDataType>>,
        }

        impl PartialOrd for MockDataType {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.data.cmp(&other.data))
            }
        }

        impl PartialEq for MockDataType {
            fn eq(&self, other: &Self) -> bool {
                self.data == other.data
            }
        }

        const CAPACITY: usize = 0xff;
        let node_storage = Vec::with_capacity(CAPACITY);


        unsafe {
            let mut mock_list = IntrusiveList::new(node_storage.as_ptr(), CAPACITY);
            let data = -123;
            let first_node = MockDataType {
                data,
                node: None,
            };
            mock_list.replace_first(first_node);
            assert_eq!(mock_list.get_first().data, data);
        };
    }
}
