use std::error::Error;

use buttplug::{
  client::{ButtplugClient, ButtplugClientDeviceMessageType, ButtplugClientEvent, VibrateCommand},
  core::messages::StopDeviceCmd,
  server::ButtplugServer,
};
use futures::StreamExt;

type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync + 'static>>;

#[tokio::main]
async fn main() -> BoxResult<()> {
  let client = ButtplugClient::new("2hu vibe");
  let mut event_stream = client.event_stream();

  let event_loop = async {
    while let Some(event) = event_stream.next().await {
      match event {
        ButtplugClientEvent::ServerConnect => {
          println!("connected owo");
        }
        ButtplugClientEvent::ServerDisconnect => {
          println!("disconnected uwu");
        }
        ButtplugClientEvent::DeviceAdded(device) => {
          println!("device added: {}", device.name);
        }
        ButtplugClientEvent::DeviceRemoved(info) => {
          println!("device removed: {}", info.name);
        }
        ButtplugClientEvent::ScanningFinished => {
          println!("scanning finished");
        }
        ButtplugClientEvent::PingTimeout => {
          eprintln!("ping timeout");
        }
        ButtplugClientEvent::Error(error) => {
          eprintln!("error: {}", error);
        }
      }
    }
  };

  client.connect_in_process(Some(ButtplugServer::default())).await?;
  client.start_scanning().await?;

  tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

  // tmp
  let xbone = &client.devices()[0];

  xbone.vibrate(VibrateCommand::Speed(1.0)).await?;

  tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

  xbone.stop().await?;
  //

  event_loop.await;
  client.disconnect().await?;

  Ok(())
}
