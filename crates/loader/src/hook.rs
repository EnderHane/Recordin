use crate::env;

mod com;
mod graphics;
mod infect;
mod lib_load;
mod sound;
mod timing;

pub(super) fn init() -> anyhow::Result<()> {
    if *env::PROCESS_IS_CLI || *env::AGGRESSIVE {
        infect::init()?;
    }
    if *env::PROCESS_IS_CLI {
        return Ok(());
    }
    com::init()?;
    lib_load::init()?;
    timing::init()?;
    Ok(())
}
