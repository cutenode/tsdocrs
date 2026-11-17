//! How much one parse of bench/corpus/large.json allocates, held under ceilings, so a change that
//! allocates more fails here. The ceilings sit at what a parse takes now; lower them when a change
//! takes less. `cargo nextest run --test allocations --no-capture` prints the counts.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use tsdocrs::TSDocParser;

/// The system allocator, counting what this thread asks of it.
struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    static REALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    static BYTES: Cell<u64> = const { Cell::new(0) };
}

fn add(counter: &'static std::thread::LocalKey<Cell<u64>>, by: usize) {
    counter.with(|count| count.set(count.get() + by as u64));
}

// SAFETY: every call goes straight to `System`, which upholds `GlobalAlloc`'s contract.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        add(&ALLOCATIONS, 1);
        add(&BYTES, layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        add(&ALLOCATIONS, 1);
        add(&BYTES, layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        add(&REALLOCATIONS, 1);
        add(&BYTES, new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// What this thread has asked for so far: allocations, reallocations, and bytes requested by both.
fn counts() -> [u64; 3] {
    [ALLOCATIONS.with(Cell::get), REALLOCATIONS.with(Cell::get), BYTES.with(Cell::get)]
}

#[test]
fn a_corpus_pass_allocates_no_more_than_it_did() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/bench/corpus/large.json");
    let comments: Vec<String> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let parser = TSDocParser::default();
    // The default configuration is built on first use, outside the count.
    drop(parser.parse_string("/** */"));

    let before = counts();
    for comment in &comments {
        drop(parser.parse_string(comment));
    }
    let [allocations, reallocations, bytes] = std::array::from_fn(|at| counts()[at] - before[at]);
    println!("{allocations} allocations, {reallocations} reallocations, {bytes} bytes requested");

    assert!(allocations <= 93_718, "{allocations} allocations");
    assert!(reallocations <= 85_349, "{reallocations} reallocations");
    assert!(bytes <= 262_714_582, "{bytes} bytes requested");
}
