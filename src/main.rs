extern crate core;
extern crate iron;
extern crate router;
extern crate ws;
extern crate either;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate futures;

use iron::prelude::*;
use iron::status;
use router::Router;

use std::rc::Rc;
use std::cell::Cell;

mod auth;
mod instance_graph;
mod instance_manager;
//mod leaderboard;
//mod mm;
mod rule_graph;

use rule_graph::config::{ JsonToGraphConverter, read_json_config };

struct WebSocket {
  out: ws::Sender,
  count: Rc<Cell<u32>>,
}

impl ws::Handler for WebSocket {
  fn on_open(&mut self, _ : ws::Handshake) -> ws::Result<()> {
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

fn main() {
  let json_config = read_json_config(std::path::Path::new("./src/config/example.json")).expect("Couldn't read json config!");
  let mut converter = JsonToGraphConverter::new(json_config);
  let graph = converter.convert().expect("Couldn't convert json config!");

  let mut router = Router::new();

  router.get("/", hello_world, "home");

  fn hello_world(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "Hello world!")))
  }

  std::thread::spawn(move || {
    ws::listen("localhost:3001", |out| {
      WebSocket {
        out: out,
        count: Rc::new(Cell::new(0)),
      }
    }).unwrap();
  });

  Iron::new(router).http("localhost:3000").unwrap();
}