extern crate bodyparser;
#[macro_use] extern crate iron;
#[macro_use] extern crate router;
extern crate serde;
extern crate serde_json;
extern crate base64;
extern crate image;
extern crate reqwest;
extern crate http;

use iron::prelude::*;
use iron::{headers, status, IronError};
use http::StatusCode;
use iron::modifiers::Header;
use iron::error::HttpError;
use image::{ImageResult, DynamicImage};
use router::Router;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use base64::decode_config_slice;
use std::fmt::{self, Debug};

use std::io::BufWriter; 
use std::io::prelude::*;                                                                                                                                            use std::fs::File;


macro_rules! any2image_err {
    ($r:expr, $t:expr) => {
        match $r {
            Ok(v) => v,
            Err(e) => return Err(image::ImageError::FormatError($t.to_string() + &e.to_string())),
        }
    };
}
macro_rules! unwrap_or_empty {
    ($r:expr) => {
        match $r {
            Some(v) => match v.as_array() {
                           Some(v) => v.to_owned(),
                           None => vec![],
                       },
            None => vec![],
        }
    };
}

fn decode2image(imstr: &str) -> ImageResult<DynamicImage>{
    let mut buf: Vec<u8> = vec![];
    any2image_err!(base64::decode_config_buf(imstr, base64::STANDARD, &mut buf), "Can't decode an image. ");
    let file = File::create("/tmp/decoded_raw.png")?;
    let mut bw = BufWriter::new(file);
    dbg!(bw.write_all(&buf));
    let result = image::load_from_memory(&buf);
    //let result = image::load_from_memory_with_format(&buffer, image::ImageFormat::PNG).unwrap();
    return result;
}

fn fetch_image(url: &str) -> ImageResult<DynamicImage>{
    let mut resp = any2image_err!(reqwest::get(url), "Can't fetch an image. ");
    let mut buf: Vec<u8> = vec![];
    any2image_err!(resp.copy_to(&mut buf), "Can't copy decoded image from buffer. ");
    let result = image::load_from_memory(&buf);
    return result;
}

fn process_image(img: DynamicImage, path: &str) -> Result<(), std::io::Error>{
    return img.save(path);
}


fn submit_image(req :&mut Request) -> IronResult<Response> {
    let bad_j = (status::BadRequest, "Not a JSON format");
    let req_body = itry!(itry!(req.get::<bodyparser::Json>(), bad_j).ok_or(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Such an error")), bad_j);
    let imageobjs = unwrap_or_empty!(req_body.get("images"));
    //dbg!(imageobjs);

    for obj in imageobjs {
        let imgstr = iexpect!(obj.as_str());
        //dbg!(imgstr);
        let img = itry!(decode2image(imgstr), (status::InternalServerError, "Unable to decode from base64 data."));
        itry!(process_image(img, "/tmp/decoded.png"), (status::InternalServerError, "Unable to process image."));
    }
    let urlobjs = unwrap_or_empty!(req_body.get("urls"));
    for obj in urlobjs {
        let url = iexpect!(obj.as_str());
        dbg!(url);
        let img = itry!(fetch_image(url), (status::BadGateway, "Unable to fetch ".to_string()+&url.to_string()));
        itry!(process_image(img, "/tmp/linked.png"), (status::InternalServerError, "Unable to process image."));
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
