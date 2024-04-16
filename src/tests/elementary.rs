// a test if assert works
#[cfg(test)]
mod elementary {
    #[test_case]
    fn test_assert() {
        assert!(true);
    }
}