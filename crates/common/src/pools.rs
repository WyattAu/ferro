//! Memory pools for high-frequency allocations.
//!
//! Provides arena allocators for request handling, buffer pools for network I/O,
//! and string interning for repeated strings.

use bumpalo::Bump;
use dashmap::DashMap;
use std::sync::Arc;

/// Request-scoped arena allocator.
///
/// Allocations are freed in bulk when the arena is dropped, making it
/// ideal for per-request temporary data.
pub struct RequestArena {
    bump: Bump,
}

impl RequestArena {
    /// Create a new arena with default capacity.
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }

    /// Create a new arena with specified capacity hint.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bump: Bump::with_capacity(capacity),
        }
    }

    /// Allocate a value in the arena.
    pub fn alloc<T>(&self, val: T) -> &mut T {
        self.bump.alloc(val)
    }

    /// Allocate a string slice in the arena.
    pub fn alloc_str(&self, s: &str) -> &str {
        self.bump.alloc_str(s)
    }

    /// Allocate a copy of a slice in the arena.
    pub fn alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> &[T] {
        self.bump.alloc_slice_copy(slice)
    }

    /// Allocate a clone of a slice in the arena.
    pub fn alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> &[T] {
        self.bump.alloc_slice_clone(slice)
    }

    /// Get the number of bytes allocated.
    pub fn allocated_bytes(&self) -> usize {
        self.bump.allocated_bytes()
    }

    /// Get the capacity of the arena.
    pub fn capacity(&self) -> usize {
        self.bump.chunk_capacity()
    }
}

impl Default for RequestArena {
    fn default() -> Self {
        Self::new()
    }
}

/// Buffer pool for network I/O.
///
/// Reuses pre-allocated buffers to reduce allocation pressure during
/// high-throughput network operations.
pub struct BufferPool {
    buffers: Vec<Vec<u8>>,
    max_size: usize,
    buffer_size: usize,
}

impl BufferPool {
    /// Create a new buffer pool.
    ///
    /// # Arguments
    /// * `max_size` - Maximum number of buffers to keep in the pool
    /// * `buffer_size` - Initial capacity for each buffer
    pub fn new(max_size: usize, buffer_size: usize) -> Self {
        let mut buffers = Vec::with_capacity(max_size);
        for _ in 0..max_size {
            buffers.push(Vec::with_capacity(buffer_size));
        }
        Self {
            buffers,
            max_size,
            buffer_size,
        }
    }

    /// Get a buffer from the pool, or create a new one if empty.
    pub fn get(&mut self) -> Vec<u8> {
        self.buffers
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(self.buffer_size))
    }

    /// Return a buffer to the pool.
    pub fn put(&mut self, mut buffer: Vec<u8>) {
        if self.buffers.len() < self.max_size {
            buffer.clear();
            self.buffers.push(buffer);
        }
    }

    /// Get the number of available buffers.
    pub fn available(&self) -> usize {
        self.buffers.len()
    }

    /// Get the maximum pool size.
    pub fn max_size(&self) -> usize {
        self.max_size
    }
}

/// Thread-safe buffer pool wrapper.
pub struct SharedBufferPool {
    inner: std::sync::Mutex<BufferPool>,
}

impl SharedBufferPool {
    /// Create a new shared buffer pool.
    pub fn new(max_size: usize, buffer_size: usize) -> Self {
        Self {
            inner: std::sync::Mutex::new(BufferPool::new(max_size, buffer_size)),
        }
    }

    /// Get a buffer from the pool.
    pub fn get(&self) -> Vec<u8> {
        self.inner.lock().unwrap().get()
    }

    /// Return a buffer to the pool.
    pub fn put(&self, buffer: Vec<u8>) {
        self.inner.lock().unwrap().put(buffer)
    }

    /// Get the number of available buffers.
    pub fn available(&self) -> usize {
        self.inner.lock().unwrap().available()
    }
}

/// String interner for deduplicating repeated strings.
///
/// Uses a concurrent hash map for thread-safe access. Interned strings
/// are reference-counted and shared across threads.
pub struct StringInterner {
    strings: DashMap<String, Arc<str>>,
}

impl StringInterner {
    /// Create a new string interner.
    pub fn new() -> Self {
        Self {
            strings: DashMap::new(),
        }
    }

    /// Intern a string, returning a shared reference.
    ///
    /// If the string is already interned, returns the existing reference.
    /// Otherwise, interns it and returns a new reference.
    pub fn intern(&self, s: &str) -> Arc<str> {
        if let Some(interned) = self.strings.get(s) {
            return Arc::clone(interned.value());
        }
        let interned: Arc<str> = Arc::from(s);
        self.strings.insert(s.to_string(), Arc::clone(&interned));
        interned
    }

    /// Get the number of interned strings.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if the interner is empty.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Clear all interned strings.
    pub fn clear(&self) {
        self.strings.clear();
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Global pools singleton.
pub struct GlobalPools {
    /// Shared buffer pool for network I/O.
    pub buffer_pool: SharedBufferPool,
    /// Global string interner.
    pub string_interner: StringInterner,
}

impl GlobalPools {
    /// Create a new global pools instance.
    pub fn new() -> Self {
        Self {
            buffer_pool: SharedBufferPool::new(64, 8192),
            string_interner: StringInterner::new(),
        }
    }

    /// Get the global pools instance.
    pub fn instance() -> &'static Self {
        use std::sync::OnceLock;
        static POOLS: OnceLock<GlobalPools> = OnceLock::new();
        POOLS.get_or_init(GlobalPools::new)
    }
}

impl Default for GlobalPools {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_arena_alloc() {
        let arena = RequestArena::new();
        let val = arena.alloc(42);
        assert_eq!(*val, 42);
    }

    #[test]
    fn test_request_arena_alloc_str() {
        let arena = RequestArena::new();
        let s = arena.alloc_str("hello world");
        assert_eq!(s, "hello world");
    }

    #[test]
    fn test_request_arena_alloc_slice_copy() {
        let arena = RequestArena::new();
        let slice = &[1, 2, 3, 4, 5];
        let allocated = arena.alloc_slice_copy(slice);
        assert_eq!(allocated, slice);
    }

    #[test]
    fn test_buffer_pool_get_put() {
        let mut pool = BufferPool::new(4, 1024);
        assert_eq!(pool.available(), 4);

        let buf = pool.get();
        assert_eq!(buf.capacity(), 1024);
        assert_eq!(pool.available(), 3);

        pool.put(buf);
        assert_eq!(pool.available(), 4);
    }

    #[test]
    fn test_buffer_pool_creates_new_when_empty() {
        let mut pool = BufferPool::new(2, 512);
        let _buf1 = pool.get();
        let _buf2 = pool.get();
        let buf3 = pool.get(); // Pool empty, creates new
        assert_eq!(buf3.capacity(), 512);
    }

    #[test]
    fn test_buffer_pool_max_size() {
        let mut pool = BufferPool::new(2, 1024);
        let buf1 = pool.get();
        let buf2 = pool.get();
        let buf3 = pool.get();

        pool.put(buf1);
        pool.put(buf2);
        pool.put(buf3); // Pool full, dropped
        assert_eq!(pool.available(), 2);
    }

    #[test]
    fn test_shared_buffer_pool() {
        let pool = SharedBufferPool::new(4, 1024);
        assert_eq!(pool.available(), 4);

        let buf = pool.get();
        assert_eq!(pool.available(), 3);

        pool.put(buf);
        assert_eq!(pool.available(), 4);
    }

    #[test]
    fn test_string_interner() {
        let interner = StringInterner::new();
        let s1 = interner.intern("hello");
        let s2 = interner.intern("hello");
        let s3 = interner.intern("world");

        assert!(Arc::ptr_eq(&s1, &s2));
        assert!(!Arc::ptr_eq(&s1, &s3));
        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn test_string_interner_is_empty() {
        let interner = StringInterner::new();
        assert!(interner.is_empty());
        interner.intern("test");
        assert!(!interner.is_empty());
    }

    #[test]
    fn test_string_interner_clear() {
        let interner = StringInterner::new();
        interner.intern("hello");
        interner.intern("world");
        assert_eq!(interner.len(), 2);

        interner.clear();
        assert_eq!(interner.len(), 0);
    }

    #[test]
    fn test_global_pools() {
        let pools = GlobalPools::instance();
        let buf = pools.buffer_pool.get();
        assert_eq!(buf.capacity(), 8192);
        pools.buffer_pool.put(buf);

        let s = pools.string_interner.intern("test");
        assert_eq!(&*s, "test");
    }

    #[test]
    fn test_request_arena_with_capacity() {
        let arena = RequestArena::with_capacity(1024);
        let val = arena.alloc(42);
        assert_eq!(*val, 42);
    }

    #[test]
    fn test_request_arena_default() {
        let arena = RequestArena::default();
        let val = arena.alloc(42);
        assert_eq!(*val, 42);
    }

    #[test]
    fn test_request_arena_alloc_slice_clone() {
        let arena = RequestArena::new();
        let slice = &["hello", "world"];
        let allocated = arena.alloc_slice_clone(slice);
        assert_eq!(allocated, slice);
    }

    #[test]
    fn test_request_arena_allocated_bytes() {
        let arena = RequestArena::new();
        let _ = arena.alloc(42u64);
        assert!(arena.allocated_bytes() > 0);
    }

    #[test]
    fn test_request_arena_capacity() {
        let arena = RequestArena::with_capacity(1024);
        assert!(arena.capacity() >= 1024);
    }

    #[test]
    fn test_buffer_pool_max_size_getter() {
        let pool = BufferPool::new(8, 2048);
        assert_eq!(pool.max_size(), 8);
    }

    #[test]
    fn test_buffer_pool_available_after_exhaust() {
        let mut pool = BufferPool::new(2, 512);
        let _buf1 = pool.get();
        let _buf2 = pool.get();
        assert_eq!(pool.available(), 0);

        let _buf3 = pool.get();
        assert_eq!(pool.available(), 0);
    }

    #[test]
    fn test_shared_buffer_pool_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let pool = Arc::new(SharedBufferPool::new(4, 1024));
        let mut handles = vec![];

        for _ in 0..4 {
            let pool = pool.clone();
            handles.push(thread::spawn(move || {
                let buf = pool.get();
                assert_eq!(buf.capacity(), 1024);
                pool.put(buf);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_string_interner_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let interner = Arc::new(StringInterner::new());
        let mut handles = vec![];

        for i in 0..4 {
            let interner = interner.clone();
            handles.push(thread::spawn(move || {
                let s = interner.intern(&format!("string-{}", i));
                assert!(!s.is_empty());
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_global_pools_singleton() {
        let pools1 = GlobalPools::instance();
        let pools2 = GlobalPools::instance();
        assert!(std::ptr::eq(pools1, pools2));
    }

    #[test]
    fn test_global_pools_default() {
        let pools = GlobalPools::default();
        let buf = pools.buffer_pool.get();
        assert_eq!(buf.capacity(), 8192);
        pools.buffer_pool.put(buf);
    }
}
