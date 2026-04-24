mod app;
mod models;
mod services;
mod tabs;
mod ui;

use app::App;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    mac_host_core::common::load_env();
    let mut app = App::new()?;
    app.run().await?;
    Ok(())
}
