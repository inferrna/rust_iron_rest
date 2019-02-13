extern crate bodyparser;
#[macro_use] extern crate iron;
#[macro_use] extern crate router;
extern crate serde;
extern crate serde_json;
extern crate base64;
extern crate image;
extern crate reqwest;
extern crate http;
extern crate md5;

use iron::prelude::*;
use iron::{headers, status, IronError};
use http::StatusCode;
use iron::modifiers::Header;
use iron::error::HttpError;
use image::{ImageResult, DynamicImage, GenericImage};
use router::Router;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use base64::decode_config_slice;
use std::fmt::{self, Debug};

use std::io::BufWriter;
use std::io::prelude::*;
use std::fs::{File, read_dir};

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

const MAXSZ: u32 = 100;
const PATH: &str = "/tmp/images/";
const TPATH: &str = "/tmp/images/thumbs/";
const EXT: &str = ".jpg";

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

fn process_image(img: DynamicImage, name: &str) -> Result<(), std::io::Error>{
    let imres = img.resize(MAXSZ, MAXSZ, image::imageops::CatmullRom);
    let digest = md5::compute(img.raw_pixels());
    let namext = format!("{}_{:x}{}", name.to_string(), digest, EXT);
    let count = read_dir(PATH)?.take_while(Result::is_ok)
                               .map(|e| e.unwrap().file_name().into_string())
                               .take_while(Result::is_ok)
                               .filter(|n| n.to_owned().unwrap().contains(namext.as_str())).count();
    if count > 0 {
        println!("{} already exists", namext);
        return Ok(());
    } else {
        println!("Saving {} to {}", namext, TPATH);
        imres.save(TPATH.to_string()+&namext)?;
        println!("Saving {} to {}", namext, PATH);
        return img.save(PATH.to_string()+&namext);
    }
}




fn _submit_image(req :&mut Request) -> IronResult<Response> {
    let bad_j = (status::BadRequest, "Not a JSON format");
    let req_body = itry!(itry!(req.get::<bodyparser::Json>(), bad_j).ok_or(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Such an error")), bad_j);
    let imageobjs = unwrap_or_empty!(req_body.get("images"));
    //dbg!(imageobjs);

    for obj in imageobjs {
        let imgstr = iexpect!(obj.as_str());
        //dbg!(imgstr);
        let img = itry!(decode2image(imgstr), (status::InternalServerError, "Unable to decode from base64 data."));
        itry!(process_image(img, "decoded"), (status::InternalServerError, "Unable to process image."));
    }
    let urlobjs = unwrap_or_empty!(req_body.get("urls"));
    for obj in urlobjs {
        let url = iexpect!(obj.as_str());
        dbg!(url);
        let img = itry!(fetch_image(url), (status::BadGateway, "Unable to fetch ".to_string()+&url.to_string()));
        itry!(process_image(img, "linked"), (status::InternalServerError, "Unable to process image."));
    }
    return Ok(Response::with((
        status::Ok,
        Header(headers::ContentType::json()), "{\"result\":\"ok\"}"
    )));
}

fn submit_image(req :&mut Request) -> IronResult<Response> {
    let res = _submit_image(req);
    if res.is_err() {
        dbg!(res.as_ref()); //Real error goes to log.
    }
    return res;
}

fn main() {
    let router = router!(submit_image: post "/upload" => submit_image);
    let chain = Chain::new(router);
    Iron::new(chain).http("localhost:3000").unwrap();
}
