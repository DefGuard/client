use vergen_git2::{Emitter, Git2Builder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let git2 = Git2Builder::default().sha(true).build()?;
    Emitter::default().add_instructions(&git2)?.emit()?;
    Ok(())
}
