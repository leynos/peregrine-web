//! Simultaneous mutable loans of one value remain rejected.

fn main() {
    let mut value = String::from("start");
    let first = &mut value;
    let second = &mut value;
    first.push('!');
    second.push('?');
}