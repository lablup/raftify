pub mod macro_utils {
    macro_rules! function_name {
        () => {{
            fn f() {}
            fn type_name_of<T>(_: T) -> &'static str {
                std::any::type_name::<T>()
            }
            let name = type_name_of(f);
            &name[..name.len() - 3] // subtract length of "f"
        }};
    }

    pub(crate) use function_name;
}
