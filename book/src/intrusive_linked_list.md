# Intrusive Linked List

An Intrusive Linked List is a singly-linked list where each node in the list is embedded in another data structure.

## Data Structures

There are two data structures defined for intrusive linked lists.

```c
// Pointer to the next node in the linked list
typedef void* intrusive_link;

// Intrusive links can be used in other data structures such as the following example:
struct example_node {
    // Any number of fields can be put here
    int example_field1;
    char example_field2;
    // Link to the next node
    intrusive_link next;
};

// An intrusive linked list
//
// Contains heap-allocated storage (how you allocate this depends on the environment which the data structure
// is used in) and a pointer to the first node in the list. These pointers may or may not be the same
struct intrusive_list {
    // Heap-allocated pointer for nodes
    void* storage;
    // Size of heap allocation in bytes
    size_t size;
    // Pointer to the first node in the list
    intrusive_link start;
};
```

### List Initialization

First, the list needs to be initialized. As previously stated, how the list is allocated depends on if a memory allocator exists yet; so that part will be skipped in this example.

All that needs to be known is that the list is initialized with no nodes; they need to be explicitly added.

### Insert

To insert a new node, first iterate over the list until you find the place where the new node should be inserted. This is not explicitly defined here because it would be different depending on if the list is ordered or unordered.

The following insertion function requires the pointer which the node should be inserted at and the size of the new node:

```c
void insert_node(intrusive_list src, intrusive_link* dst, size_t size) {

}
```
