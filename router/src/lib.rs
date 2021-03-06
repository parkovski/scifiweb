#![cfg_attr(not(feature = "cargo-clippy"), allow(unknown_lints))]

extern crate futures;
#[cfg(feature = "hyper")]
extern crate hyper;
extern crate route_recognizer;
extern crate url;
extern crate scifi_util as util;

pub mod builder;
mod handlers;
#[cfg(feature = "hyper")]
pub mod hyper_router;
#[allow(module_inception)]
mod router;

pub use self::handlers::{ExtMap, GetAny, GetParam, ParamError, Params, Rejection};
pub use self::router::{Router, RoutePath};

#[cfg(test)]
mod test {
  use std::rc::Rc;
  use std::cell::{Cell, RefCell};
  use futures::future::{self, FutureResult};
  use super::*;
  use super::builder::*;

  struct AppendHandler(&'static str);

  impl<'a> Route<'a, Rc<Cell<String>>> for AppendHandler {
    type Future = FutureResult<(), ()>;

    fn call(&self, req: Rc<Cell<String>>, params: &Params, _ext: &mut ExtMap) -> Self::Future {
      req.set(format!("{}: {:?}\n", self.0, params));
      future::ok(())
    }
  }

  struct BoolFilter(bool);

  impl<'a> Filter<'a, Rc<Cell<String>>, (), ()> for BoolFilter {
    type Future = FutureResult<(), Rejection<(), ()>>;

    fn call(&self, _req: &Rc<Cell<String>>, _params: &Params, _ext: &mut ExtMap) -> Self::Future {
      if self.0 {
        future::ok(())
      } else {
        future::err(Rejection::Response(()))
      }
    }
  }

  struct PasswordFilter(&'static str, &'static str);

  impl<'a> Filter<'a, Rc<Cell<String>>, (), ()> for PasswordFilter {
    type Future = FutureResult<(), Rejection<(), ()>>;

    fn call(&self, req: &Rc<Cell<String>>, params: &Params, _ext: &mut ExtMap) -> Self::Future {
      if let Some(password) = params.find(self.0) {
        if self.1 == password {
          return future::ok(());
        }
        req.set(format!(
          "Denying password '{}' for param '{}'\n",
          password,
          self.0
        ));
      } else {
        req.set("Bug! Param not present.".to_string())
      }
      future::err(Rejection::Response(()))
    }
  }

  #[test]
  fn test_router() {
    let mut output = String::new();
    let error_output = RefCell::new(String::new());
    {
      let builder = RouterBuilder::new((
        |err: ()| {
          *error_output.borrow_mut() += "error\n";
          future::ok(())
        },
        |path: &str| {
          *error_output.borrow_mut() += format!("not found: {}\n", path).as_str();
          future::ok(())
        }
      ));

      let router = builder
        .dir("/test")
        .with_filter(BoolFilter(true))
        .dir(":hi")
        .with_filter(PasswordFilter("hi", "hi"))
        .route("/foo", AppendHandler("/test/:hi/foo"))
        .to_root()
        .route("/:param/hi", AppendHandler("/:param/hi"))
        .dir("/foo")
        .with_filter(BoolFilter(false))
        .route("/bar", AppendHandler("/foo/bar"))
        .build();


      #[allow(unused_must_use)]
      {
        let paths = [
          "/test/hi/foo",
          "/test/bye/foo",
          "/hello/hi",
          "/test/hi",
          "/foo/bar",
          "/test/foo",
          "/notfound",
        ];
        for path in &paths {
          let out = Rc::new(Cell::new(String::new()));
          router.run_for_path(path, out.clone()).poll();
          output += out.take().as_str();
        }
      }
    }

    const EXPECTED: &'static str = r#"/test/:hi/foo: Params { map: {"hi": "hi"} }
Denying password 'bye' for param 'hi'
/:param/hi: Params { map: {"param": "hello"} }
/:param/hi: Params { map: {"param": "test"} }
-----
not found: /test/foo
not found: /notfound
"#;
    output += "-----\n";
    output += error_output.borrow().as_str();
    println!("{}", output);
    assert!(output == EXPECTED);
  }
}
