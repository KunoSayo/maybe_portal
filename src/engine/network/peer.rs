use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Waker};

use log::error;
use tokio::io::AsyncWriteExt;
use tokio::select;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_kcp::KcpStream;

use crate::engine::network::{DataHandler, NetworkMessage};
use crate::engine::task::wakers::NeverWaker;

/// The peer
#[derive(Debug, Clone)]
pub struct Peer {
    pub listening: Arc<AtomicBool>,
    /// The remote socket address.
    pub addr: SocketAddr,
    /// sender to send the message to the target
    pub sender: UnboundedSender<NetworkMessage>,
}


impl Peer {
    /// Need call in tokio runtime
    pub fn new(stream: KcpStream, addr: SocketAddr, handler: impl DataHandler) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let this = Self {
            listening: Arc::new(AtomicBool::new(true)),
            addr,
            sender,
        };
        tokio::spawn(this.clone().run_loop(stream, receiver, handler));
        this
    }

    async fn run_loop(self, mut stream: KcpStream, mut receiver: UnboundedReceiver<NetworkMessage>, handler: impl DataHandler) {
        let mut errs = 0;
        macro_rules! got_err {
            () => {
                errs += 1;
                if errs > 5 {
                    break;
                }
            };
        }
        let mut buf = Vec::new();
        buf.resize(65536, 0);
        while self.listening.load(Ordering::Acquire) {
            select! {
                mut msg = receiver.recv() => {
                    loop {
                        match msg {
                            Some(msg) => {
                                match msg {
                                    NetworkMessage::Rely(packet) => {
                                        if let Err(e) = stream.send(&packet[..]).await {
                                            error!("Send packet failed for {:?}", e);
                                            got_err!();
                                        } else {
                                            errs = 0;
                                        }
                                    }
                                    NetworkMessage::Once(packet) => {
                                        match stream.poll_send(&mut Context::from_waker(&Waker::from(Arc::new(NeverWaker))), &packet[..]) {
                                            Poll::Ready(x) => {
                                                match x {
                                                    Ok(n) => {
                                                        if n != packet.len() {
                                                            error!("Tried to send {} bytes but sent {} bytes. Checking it must not be stream mode!", packet.len(), n);
                                                        } else {
                                                            errs = 0;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        error!("Send packet failed for {:?}", e);
                                                        got_err!();
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            None => {
                                break;
                            }
                        }
                        if let Ok(nmsg) = receiver.try_recv() {
                            msg = Some(nmsg);
                        } else {
                            break;
                        }
                    }
                    if let Err(e) = stream.flush().await {
                        error!("Got packet failed for {:?}", e);
                        got_err!();
                    }
                }
                data = stream.recv(&mut buf) => {
                    match data {
                        Ok(n) => {
                            errs = 0;
                            if !handler.handle(&self, &buf[..n]) {
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Receive packet failed for {:?}", e);
                            got_err!();
                        }
                    }
                }

            }
        }
        self.listening.store(false, Ordering::Release);
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.listening.store(false, Ordering::Relaxed);
    }
}