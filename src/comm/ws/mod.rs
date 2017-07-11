use std::net::Ipv4Addr;
use std::rc::Rc;
use std::cell::Cell;

use ws;

struct WebSocket {
  out: ws::Sender,
  count: Rc<Cell<u32>>,
}

impl ws::Handler for WebSocket {
  fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
    self.count.set(self.count.get() + 1);
    Ok(())
  }

  fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
    self.out.send(msg)
  }

  fn on_close(&mut self, _code: ws::CloseCode, _reason: &str) {
    self.count.set(self.count.get() - 1)
  }

  fn on_error(&mut self, err: ws::Error) {
    println!("Error: {:?}", err);
  }
}

pub fn start(port: u16) -> ws::Result<()> {
  let addr = (Ipv4Addr::new(127, 0, 0, 1), port);
  ws::listen(addr, |out| {
    WebSocket {
      out: out,
      count: Rc::new(Cell::new(0)),
    }
  })
}