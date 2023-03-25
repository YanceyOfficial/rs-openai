pub mod apis;
pub mod client;
pub mod shared;

pub use client::*;
pub use shared::response_wrapper::*;

#[cfg(test)]
mod tests {

    // use super::*;

    #[test]
    fn it_works() {
        assert_eq!(1, 1);
    }
}
