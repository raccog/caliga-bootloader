/// This implementation is a shortened version of the RedoxOS implementation found here:
///
/// https://gitlab.redox-os.org/redox-os/syscall/-/blob/master/src/io/io.rs

pub trait Io {
    type Value: Copy + PartialEq;

    fn read(&self) -> Self::Value;
    fn write(&mut self, value: Self::Value);
}

pub struct ReadOnly<I> {
    inner: I,
}

impl<I> ReadOnly<I> {
    pub const fn new(inner: I) -> ReadOnly<I> {
        ReadOnly { inner }
    }
}

impl<I: Io> ReadOnly<I> {
    #[inline(always)]
    pub fn read(&self) -> I::Value {
        self.inner.read()
    }
}

pub struct WriteOnly<I> {
    inner: I,
}

impl<I> WriteOnly<I> {
    pub const fn new(inner: I) -> ReadOnly<I> {
        ReadOnly { inner }
    }
}

impl<I: Io> WriteOnly<I> {
    #[inline(always)]
    pub fn write(&mut self, value: I::Value) {
        self.inner.write(value);
    }
}
