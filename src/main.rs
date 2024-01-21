use arctic::{
    async_trait, Error as ArcticError, EventHandler, HeartRate, NotifyStream, PolarSensor,
};

use rosc::encoder;
use rosc::{OscMessage, OscPacket, OscType};

use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;

use std::env;

fn get_addr_from_arg(arg: &str) -> SocketAddrV4 {
    SocketAddrV4::from_str(arg).unwrap()
}

struct Handler {
    sock: UdpSocket,
    to_addr: SocketAddrV4,
}

impl Handler {
    fn new(sock: UdpSocket, to_addr: SocketAddrV4) -> Self {
        Self { sock, to_addr }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn heart_rate_update(&self, _ctx: &PolarSensor, heartrate: HeartRate) {
        let bpm: &u8 = heartrate.bpm();

        let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
            addr: "/heartrate".to_string(),
            args: vec![OscType::Int(*bpm as i32)],
        }))
        .unwrap();

        self.sock.send_to(&msg_buf, self.to_addr).unwrap();
        println!("Heart rate: {:?}", bpm);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let usage = format!(
        "Usage: {} DEVICE_ID HOST_IP:HOST_PORT CLIENT_IP:CLIENT_PORT",
        &args[0]
    );

    if args.len() < 3 {
        panic!("{}", usage);
    }

    let mut polar = PolarSensor::new(args[1].clone().to_string()).await.unwrap();

    while !polar.is_connected().await {
        match polar.connect().await {
            Err(ArcticError::NoBleAdaptor) => {
                println!("No bluetooth adapter found");
                return Ok(());
            }
            Err(why) => println!("Could not connect: {:?}", why),
            _ => {}
        }
    }

    let host_addr = get_addr_from_arg(&args[2]);
    let to_addr = get_addr_from_arg(&args[3]);
    let sock = UdpSocket::bind(host_addr).unwrap();

    let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
        addr: "/heartrate".to_string(),
        args: vec![],
    }))
    .unwrap();

    sock.send_to(&msg_buf, to_addr).unwrap();

    if let Err(why) = polar.subscribe(NotifyStream::HeartRate).await {
        println!("Could not subscribe to heart rate notifications: {:?}", why)
    }

    polar.event_handler(Handler::new(sock, to_addr));

    let result = polar.event_loop().await;
    println!("No more data: {:?}", result);
    Ok(())
}
