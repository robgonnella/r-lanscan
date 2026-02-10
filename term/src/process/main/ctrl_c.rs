use std::{
    process,
    sync::{Arc, RwLock},
};

use color_eyre::eyre::{Result, eyre};

#[derive(Default)]
pub struct CtrlCHandler {
    block: Arc<RwLock<bool>>,
}

impl CtrlCHandler {
    pub fn intercept(&self) -> Result<()> {
        // captures ctrl-c only in main thread so when we drop down to shell
        // commands like ssh, we will pause the key handler for ctrl-c in app
        // and capture ctrl-c here to prevent exiting app and just let ctrl-c
        // be handled by the command being executed, which should return us
        // to our app where we can restart our ui and key-handlers
        let block = Arc::clone(&self.block);

        ctrlc::set_handler(move || {
            if let Ok(blocked) = block.read() {
                if *blocked {
                    println!("captured ctrl-c!");
                } else {
                    process::exit(1);
                }
            } else {
                process::exit(1);
            }
        })
        .map_err(|err| eyre!("failed to set ctrl-c handler: {}", err))
    }

    pub fn block(&self) -> Result<()> {
        let mut blocked = self.block.write().map_err(|err| {
            eyre!("failed to get write lock on ctrl-c block setting: {}", err)
        })?;
        *blocked = true;
        Ok(())
    }

    pub fn unblock(&self) -> Result<()> {
        let mut blocked = self.block.write().map_err(|err| {
            eyre!("failed to get write lock on ctrl-c block setting: {}", err)
        })?;
        *blocked = false;
        Ok(())
    }
}
