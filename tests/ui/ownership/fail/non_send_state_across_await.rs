//! Non-`Send` state retained across an await cannot be spawned on a
//! multi-threaded runtime.

use std::rc::Rc;

fn main() {
    let _handle = tokio::spawn(async {
        let local = Rc::new(1_u8);
        tokio::task::yield_now().await;
        let _ = Rc::clone(&local);
    });
}