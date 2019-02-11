extern crate bodyparser;
extern crate iron;
extern crate persistent;
#[macro_use]
extern crate router;
extern crate serde;
extern crate serde_json;

use iron::{headers, status};
use iron::modifiers::Header;
use iron::prelude::*;
use router::Router;
use serde::ser::{Serialize, Serializer, SerializeStruct};


fn submit_payment(req :&mut Request) -> IronResult<Response> {
    let req_body = req.get::<bodyparser::Json>().unwrap().unwrap();
    dbg!(&req_body);
    let imageobjs = req_body.get("images").unwrap().as_array().unwrap();
    dbg!(imageobjs);

    for obj in imageobjs {
        let imgstr = obj.as_str().unwrap();
        dbg!(imgstr);
    }

    return Ok(Response::with((
        status::Ok,
        Header(headers::ContentType::json()), "{\"result\":\"ok\"}"
    )));
}

fn main() {
    let router = router!(submit_payment: post "/upload" => submit_payment);
    let chain = Chain::new(router);
    Iron::new(chain).http("localhost:3000").unwrap();
}
