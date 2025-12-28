use crate::envs;

mod com;
mod graphics;
mod infect;
mod libload;
mod sound;
mod times;

pub(super) fn init() -> anyhow::Result<()> {
    if *envs::PROCESS_IS_CLI || *envs::AGGRESSIVE {
        infect::init()?;
    }
    if *envs::PROCESS_IS_CLI {
        return Ok(());
    }
    com::init()?;
    libload::init()?;
    times::init()?;
    Ok(())
}
