extern crate bodyparser;
extern crate iron;
extern crate persistent;
#[macro_use]
extern crate router;
extern crate serde;
extern crate serde_json;
extern crate base64;
extern crate image;

use iron::{headers, status};
use iron::modifiers::Header;
use iron::prelude::*;
use router::Router;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use base64::decode_config_slice;

use std::io::BufWriter; 
use std::io::prelude::*;                                                                                                                                              
use std::fs::File;

fn decode2image(imstr: &str) -> image::DynamicImage{
    let mut buffer = vec![];
    base64::decode_config_buf(imstr, base64::STANDARD, &mut buffer).unwrap();
    let file = File::create("/tmp/decoded_raw.png").expect("Can't create file");
    let mut bw = BufWriter::new(file);
    dbg!(bw.write_all(&buffer));
    let result = image::load_from_memory(&buffer).unwrap();
    //let result = image::load_from_memory_with_format(&buffer, image::ImageFormat::PNG).unwrap();
    return result;
}


fn submit_image(req :&mut Request) -> IronResult<Response> {
    let req_body = req.get::<bodyparser::Json>().unwrap().unwrap();
    //dbg!(&req_body);
    let imageobjs = req_body.get("images").unwrap().as_array().unwrap();
    //dbg!(imageobjs);

    for obj in imageobjs {
        let imgstr = obj.as_str().unwrap();
        dbg!(imgstr);
        let image = decode2image(imgstr);
        image.save("/tmp/posted.png").unwrap();
    }
    

    return Ok(Response::with((
        status::Ok,
        Header(headers::ContentType::json()), "{\"result\":\"ok\"}"
    )));
}

fn main() {
    let router = router!(submit_image: post "/upload" => submit_image);
    let chain = Chain::new(router);
    Iron::new(chain).http("localhost:3000").unwrap();
}
