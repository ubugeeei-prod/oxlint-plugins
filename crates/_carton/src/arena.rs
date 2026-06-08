#![doc = "Arena allocation aliases for rule implementations."]

pub use bumpalo::Bump as Allocator;
pub use bumpalo::Bump as BumpAllocator;
pub use bumpalo::boxed::Box as ArenaBox;
pub use bumpalo::collections::String as ArenaString;
pub use bumpalo::collections::Vec as ArenaVec;

#[cfg(test)]
mod tests {
    use super::{ArenaString, ArenaVec, BumpAllocator};

    #[test]
    fn allocates_short_lived_values_in_bump_arena() {
        let allocator = BumpAllocator::new();
        let text = ArenaString::from_str_in("identifier", &allocator);
        let mut names = ArenaVec::new_in(&allocator);

        names.push(text.as_str());

        assert_eq!(names.as_slice(), ["identifier"]);
    }
}
