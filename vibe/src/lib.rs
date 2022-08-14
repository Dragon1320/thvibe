use std::{error::Error, sync::Arc, time::Duration};

use buttplug::{
  client::{ButtplugClient, ButtplugClientDevice, ButtplugClientEvent, VibrateCommand},
  server::ButtplugServer,
};
use futures::StreamExt;
use tokio::sync::oneshot;

type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync + 'static>>;

pub struct XBone {
  rt: tokio::runtime::Runtime,
  client: ButtplugClient,
  xbone: Arc<ButtplugClientDevice>,
}

impl XBone {
  pub fn new(rt: tokio::runtime::Runtime, client: ButtplugClient, xbone: Arc<ButtplugClientDevice>) -> BoxResult<Self> {
    Ok(Self { rt, client, xbone })
  }

  pub fn vibe(&self, intensity: f64, duration: Duration) -> BoxResult<()> {
    let xbone = self.xbone.clone();

    self.rt.spawn(async move {
      xbone.vibrate(VibrateCommand::Speed(intensity)).await.unwrap();

      tokio::time::sleep(duration).await;

      xbone.stop().await.unwrap();
    });

    Ok(())
  }

  pub fn stop(&self) -> BoxResult<()> {
    let xbone = self.xbone.clone();

    self.rt.spawn(async move {
      xbone.stop().await.unwrap();
    });

    Ok(())
  }
}

pub fn init_xbone() -> BoxResult<XBone> {
  let rt = tokio::runtime::Runtime::new()?;

  let client = ButtplugClient::new("2hu vibe");
  let mut event_stream = client.event_stream();

  let (tx, rx) = oneshot::channel();

  rt.spawn(async move {
    while let Some(event) = event_stream.next().await {
      match event {
        ButtplugClientEvent::ServerConnect => {
          println!("connected owo");
        }
        ButtplugClientEvent::ServerDisconnect => {
          println!("disconnected uwu");
        }
        ButtplugClientEvent::DeviceAdded(device) => {
          tx.send(()).unwrap();

          println!("device added: {}", device.name);

          // tmp
          break;
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
  });

  let xbone = rt.block_on(async {
    client
      .connect_in_process(Some(ButtplugServer::default()))
      .await
      .unwrap();
    client.start_scanning().await.unwrap();

    rx.await.unwrap();

    let xbone = client.devices()[0].clone();

    xbone
  });

  let xbone = XBone::new(rt, client, xbone)?;

  Ok(xbone)
}
