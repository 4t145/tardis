use std::env;

use tardis::basic::result::TardisResult;
use tardis::TardisFuns;
use tracing::{error, info};

use crate::app::req::test_req;

#[tokio::test]
async fn test_basic_tracing() -> TardisResult<()> {
    // env::set_var("RUST_LOG", "OFF");
    env::set_var("RUST_LOG", "info");
    TardisFuns::init(Some("tests/config")).await?;
    info!("main info...");
    error!("main error");
    test_req();
    Ok(())
}

mod app {
    pub mod req {
        use tardis::log::{error, info};

        pub fn test_req() {
            info!("app::req info");
            error!("app::req error");
        }
    }
}
