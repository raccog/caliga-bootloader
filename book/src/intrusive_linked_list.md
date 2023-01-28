# Intrusive Linked List

This is an implementation of intrusive, singly-linked lists.

It is singly-linked because each node contains a single pointer to the next node in the list.

It is intrusive because the pointers, or links, are directly embedded in the data structure being used; these data structures are referred to as the nodes in the list.

## Data Structures

There are two data structures defined for intrusive linked lists:

```rust
# use std::ptr::NonNull;
pub struct IntrusiveLink<T> {
    next: Option<NonNull<T>>
}

struct IntrusiveList<T> {
    storage: NonNull<T>,
    size: usize,
    start: IntrusiveLink<T>
}
```

Here's an example of a data structure that can be used in this intrusive linked list:

```rust
# use std::ptr::NonNull;
# struct IntrusiveLink<T> {
#     next: Option<NonNull<T>>
# }
struct ExampleNode {
    /// Link to the next node
    next: IntrusiveLink<ExampleNode>
    // ...
}
```

### List Initialization

First, the list needs to be initialized. How the list is allocated depends on if a memory allocator exists yet; so that part will be skipped in this example.

All that needs to be known is that the list is initialized with no nodes; they need to be explicitly added.

### Insert

To insert a new node, first iterate over the list until you find the place where the new node should be inserted. This is not explicitly defined here because the implementation could be different depending on if the list is ordered or unordered.

