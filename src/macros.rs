// src/macros.rs
#[macro_export]
macro_rules! s {
    // String shorthand!
    
    // Zero-arg → String::new()
    () => {
        ::std::string::String::new()
    };
    // Any single expression — works for literals, consts, or vars
    ($expr:expr) => {
        ::std::string::String::from($expr)
    };
}

#[macro_export]
macro_rules! join {
    // String-type concatenation shorthand!
    ($first:expr $(, $rest:expr)+ $(,)?) => {{
        let mut s = ::std::string::String::from($first);
        $(
            s.push_str($rest);
        )+
        s
    }};
}