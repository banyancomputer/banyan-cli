#[macro_export]
macro_rules! cast {
    ($target: expr, $pattern: path) => {{
        if let $pattern(a) = $target {
            Some(a)
        } else {
            None
        }
    }};
}
