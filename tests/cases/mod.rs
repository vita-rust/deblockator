use std::vec::Vec;

pub fn small_alloc() {
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        println!("{:?}", i);
        v.push(i);
    }
}
