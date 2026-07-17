use vergen_git2::{Emitter, Git2};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let git2 = Git2::builder().sha(true).build();
    Emitter::default().add_instructions(&git2)?.emit()?;
    Ok(())
}
