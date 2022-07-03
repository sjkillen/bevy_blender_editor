// Source: https://github.com/peterholko/bevy_tokio_tungstenite/blob/main/src/main.rs

use async_compat::Compat;
use bevy::{
    app::{ScheduleRunnerPlugin, ScheduleRunnerSettings},
    core::CorePlugin,
    prelude::*,
    tasks::IoTaskPool,
    utils::Duration,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::{error::Error, net::SocketAddr};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver as Rx, UnboundedSender as Tx};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_tungstenite::tungstenite::protocol::Message::{
    Binary as WSBinary, Pong as WSPong, Text as WSText,
};

mod camera;
mod cube_test;
mod util;

const TIMESTEP: f64 = 2.0;
const LISTEN_ADDR: &str = "127.0.0.1:9005";

pub struct BlenderEditorPlugin;
impl Plugin for BlenderEditorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            TIMESTEP,
        )))
        .add_plugin(camera::ImageCameraPlugin)
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_startup_system(setup)
        .add_system(message_system);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(BlenderEditorPlugin)
        .add_plugin(CorePlugin::default())
        .add_startup_system(cube_test::setup_cube)
        .run();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Protocol {
    WelcomeClient,
    Data(Vec<u8>),
    DataLocation(String),
}

fn setup(mut commands: Commands, task_pool: Res<IoTaskPool>) {
    let (tx, rx) = unbounded_channel::<Protocol>();
    let (b_tx, b_rx) = unbounded_channel::<Protocol>();
    task_pool
        .spawn(Compat::new(start_server(rx, b_tx)))
        .detach();
    commands.insert_resource(tx);
    commands.insert_resource(b_rx);
}

async fn start_server(mut outbound: Rx<Protocol>, incoming_tx: Tx<Protocol>) {
    println!("Starting websocket server...");
    let listener = TcpListener::bind(LISTEN_ADDR).await.expect("Can't listen");
    println!("Listening on {LISTEN_ADDR}");
    let mut clients: Vec<Tx<Protocol>> = Default::default();
    loop {
        tokio::select! {
            msg = outbound.recv() => {
                if let Some(msg) = msg {
                    clients = clients.into_iter().filter(|c_tx| {
                        c_tx.send(msg.clone()).is_ok()
                    }).collect();
                    continue;
                }
            }
            accept = listener.accept() => {
                if let Ok((stream, _)) = accept {
                    let (c_tx, c_rx) = unbounded_channel::<Protocol>();
                    clients.push(c_tx);
                    if let Ok(peer) = stream.peer_addr() {
                        let incoming_tx = incoming_tx.clone();
                        tokio::spawn(async move { if let Err(e) = handle_blender_client(c_rx, incoming_tx, peer, stream).await {
                            eprintln!("Error: {}", e);
                        }});
                    }
                    continue;
                }
            }
        }
        break;
    }
}

async fn handle_blender_client(
    mut rx: Rx<Protocol>,
    mut tx: Tx<Protocol>,
    _peer: SocketAddr,
    stream: TcpStream,
) -> Result<(), Box<dyn Error>> {
    let mut ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let welcome_msg = json::to_string(&Protocol::WelcomeClient)?;

    ws_stream.send(WSText(welcome_msg)).await?;

    println!("New client connected");

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Some(Protocol::Data(msg)) => {
                        println!("Frame data to client!");
                        println!("msg size is {}", msg.len());
                        let path = String::from("/dev/shm/blender_bevy_frame");
                        let mut fo = tokio::fs::File::create(&path).await?;
                        fo.write_all(&msg).await?;
                        let data = json::to_string(&Protocol::DataLocation(path))?;
                        ws_stream.send(WSText(data)).await?;
                    }
                    Some(msg) => {
                        let msg = json::to_string(&msg)?;
                        ws_stream.send(WSText(msg)).await?;
                    }
                    None => {
                        eprintln!("No message to recv");
                        break;
                    },
                }
            }
            msg = ws_stream.next() => {
                let msg = if let Some(msg) = msg {msg?} else {eprintln!("Got none"); break;};
                let data = if msg.is_text() {
                    json::from_str::<Protocol>(msg.to_string().as_ref())?
                } else if msg.is_ping() {
                    eprintln!("Got ping");
                    ws_stream.send(WSPong(Vec::new())).await?;
                    continue;
                } else {
                    // Must be a heartbeat or something from Python, just ignore.
                    eprintln!("Binary message recieved from client");
                    continue;
                };
                blender_client_sent_protocol(data, &mut tx).await?;
            }
        };
    }
    eprintln!("Client Closed...");
    Ok(())
}

async fn blender_client_sent_protocol(
    _protocol: Protocol,
    _tx: &mut Tx<Protocol>,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn message_system(
    view_image: Query<&camera::ViewImage>,
    tx: Res<Tx<Protocol>>,
    mut rx: ResMut<Rx<Protocol>>,
    images: Res<Assets<Image>>,
) {
    while let Ok(_data) = rx.try_recv() {
        println!("Got data!");
    }
    let image = images.get(&view_image.single().buffer_handle).unwrap();
    let packet = Protocol::Data(image.data.clone());

    tx.send(packet).expect("Failed to send frame data");
}
