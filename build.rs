use std::env;

pub fn main()
{
    let key = "DEBUG";
    let val = env::var(key).unwrap();
    if val == "true".to_string()
    {
        println!("cargo:rustc-cfg=debug");
    }
    else
    {
        println!("cargo:rustc-cfg=ndebug");
    }
}
