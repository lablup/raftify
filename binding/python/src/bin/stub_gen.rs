use pyo3_stub_gen::Result;
// Specify the exact path to the desired `raftify` crate

fn main() -> Result<()> {
    // `stub_info` is a function defined by `define_stub_info_gatherer!` macro.
    let stub = raftify_py::stub_info()?;

    stub.generate()?;
    Ok(())
}
