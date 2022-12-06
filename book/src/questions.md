# Questions

Some questions to keep track of.

* How do file systems report errors when any part of the file system could be corrupted?
* How do other boot loaders interface with multiple file system types as well as multiple physical storage device types?
  * How are the errors handled at the physical storage layer?
  * Are errors that are unique to a certain file system or storage device returned with specific error codes detailing what error occurred? Are these errors eventually combined into generalized error types?
* Are assertion statements in boot loaders a reliable method for preventing crashes and vulnerabilities?
  * Are assertsion statements only used in conjunction with other error reporting methods?
