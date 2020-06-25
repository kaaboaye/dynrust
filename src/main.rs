#![feature(rustc_private)]
#![feature(link_args)]

#[macro_use]
extern crate lazy_static;
extern crate ctrlc;
extern crate libloading as lib;
extern crate rustc_driver;

use std::fs::{self, File};
use std::io::prelude::*;
use std::io::{self, BufRead};
use std::process::{self, Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        cleanup_workspace().unwrap();
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    prepare_workspace()?;

    let res = run();

    cleanup_workspace()?;

    res
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let sample: Vec<f64> = vec![1.0, 2.0, 3.0, 50.0, 69.0];
    dbg!(&sample);

    let stdin = io::stdin();
    for (id, line) in stdin.lock().lines().enumerate() {
        let line = line?;

        // .map(|n| n + 0.23)

        let foo = code_gen(id, line)?;

        let stream = Box::new(sample.clone().into_iter());
        let result = foo(stream).collect::<Vec<_>>();
        drop(foo);

        dbg!(result);
    }

    Ok(())
}

lazy_static! {
    static ref WORKSPACE: String = format!("rust_script_ws_{}", process::id());
}

fn prepare_workspace() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir(WORKSPACE.as_str())?;
    Ok(())
}

fn cleanup_workspace() -> Result<(), Box<dyn std::error::Error>> {
    fs::remove_dir_all(WORKSPACE.as_str())?;
    Ok(())
}

fn code_gen(
    id: usize,
    injection: String,
) -> Result<
    Box<dyn Fn(Box<dyn Iterator<Item = f64>>) -> Box<dyn Iterator<Item = f64>>>,
    Box<dyn std::error::Error>,
> {
    let src_path = format!("{}/{}.rs", WORKSPACE.as_str(), id);
    let lib_path = format!("{}/lib{}.dylib", WORKSPACE.as_str(), id);

    let code = format!(
        r###"
            #![no_std]

            type Stream = Box<dyn Iterator<Item = f64>>;

            #[no_mangle]
            pub fn foo(iter: Stream) -> Stream {{
                Box::new(iter{})
            }}
        "###,
        injection
    );

    let mut file = File::create(src_path.as_str())?;
    file.write_all(code.as_bytes())?;
    drop(file);

    let args = vec![
        String::from("--crate-type=dylib"),
        src_path,
        String::from("-o"),
        lib_path.clone(),
    ];

    let mut callbacks = rustc_driver::TimePassesCallbacks::default();
    let compilation_result =
        rustc_driver::run_compiler(args.as_slice(), &mut callbacks, None, None);

    dbg!(compilation_result);

    // if compilation_result.status.code().unwrap_or_default() != 0 {
    //     println!(
    //         "Compilation result {}\n{}\n{}",
    //         compilation_result.status,
    //         std::str::from_utf8(compilation_result.stdout.as_slice()).unwrap(),
    //         std::str::from_utf8(compilation_result.stderr.as_slice()).unwrap()
    //     );
    // }

    // drop(compilation_result);

    let lib = Box::new(lib::Library::new(lib_path)?);

    let foo = unsafe {
        move |iter| {
            lib.get::<unsafe fn(Box<dyn Iterator<Item = f64>>) -> Box<dyn Iterator<Item = f64>>>(
                b"foo",
            )
            .unwrap()(iter)
        }
    };

    Ok(Box::new(foo))
}
