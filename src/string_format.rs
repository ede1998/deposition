use core::fmt::Write;

pub fn format<const N: usize>(args: core::fmt::Arguments) -> heapless::String<N> {
    fn format_inner<const N: usize>(args: core::fmt::Arguments) -> heapless::String<N> {
        let mut output = heapless::String::new();
        output
            .write_fmt(args)
            .expect("a formatting trait implementation returned an error");
        output
    }

    args.as_str()
        .map_or_else(|| format_inner(args), |s| s.try_into().unwrap())
}

#[macro_export]
macro_rules! format {
    ($max:literal, $($arg:tt)*) => {{
        let res = $crate::string_format::format::<$max>(core::format_args!($($arg)*));
        res
    }}
}
