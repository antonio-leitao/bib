use anyhow::Result;

pub fn yank(remote:String,from: String)->Result<()>{
    println!("Yanking from remote {}/{} into current stack",remote,from);
    Ok(())
}
pub fn yeet(remote:String,into: String)->Result<()>{
    println!("Yeeting current stack into remote {}/{}",remote,into);
    Ok(())
}
