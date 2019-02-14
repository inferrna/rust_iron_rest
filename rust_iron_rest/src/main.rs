extern crate bodyparser;
#[macro_use] extern crate iron;
#[macro_use] extern crate router;
extern crate base64;
extern crate image;
extern crate reqwest;
extern crate md5;
#[cfg(feature = "cvresize")]
extern crate cvtry;

#[cfg(feature = "cvresize")]
use cvtry::*;

use iron::prelude::*;
use iron::{headers, status};
use iron::modifiers::Header;
use image::{ImageResult, DynamicImage};

use std::io::BufWriter;
use std::io::prelude::*;
use std::fs::{File, read_dir};
use std::cmp::{max, min};

macro_rules! any2image_err {
    ($r:expr, $t:expr) => {
        match $r {
            Ok(v) => v,
            Err(e) => return Err(image::ImageError::FormatError(format!("Got '{}' ignited by '{}'", $t, e))),
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

const MAXSZ: u32 = 100;    //New size
const PATH: &str = "/tmp/images/";
const TPATH: &str = "/tmp/images/thumbs/";
const EXT: &str = ".jpg";


//Decode base64 string to DynamicImage
fn decode2image(imstr: &str) -> ImageResult<DynamicImage>{
    let mut buf: Vec<u8> = vec![];
    any2image_err!(base64::decode_config_buf(imstr, base64::STANDARD, &mut buf), "Can't decode an image. ");
    /* Check correctness of just decoded image file.
    let file = File::create("/tmp/decoded_raw.png")?;
    let mut bw = BufWriter::new(file);
    dbg!(bw.write_all(&buf));
    */
    let result = image::load_from_memory(&buf);
    return result;
}

//Fetch file by link and convert to DynamicImage
fn fetch_image(url: &str) -> ImageResult<DynamicImage>{
    let mut resp = any2image_err!(reqwest::get(url), "Can't fetch an image. ");
    let mut buf: Vec<u8> = vec![];
    any2image_err!(resp.copy_to(&mut buf), "Can't copy decoded image from buffer. ");
    let result = image::load_from_memory(&buf);
    return result;
}


#[cfg(feature = "cvresize")]
fn resize_with_opencv(img: &DynamicImage, sz: i32) -> DynamicImage {
    println!("Going to totally unsafe resize with OpenCV. You on your own.");
    let (width, height) = img.to_rgb().dimensions();
    let (imax, imin) = (max(width, height) as i32, min(width, height) as i32);
    let (newmax, newmin) = (sz, sz*imin/imax);
    let (n_width, n_height) = if width > height { (newmax, newmin) }
                                           else { (newmin, newmax) };
    println!("n_width = {}, n_height = {}", n_width, n_height);
    let cvimg = convert_image_to_cv(&img);
    let cvdest = resize_image_cv(cvimg, n_width, n_height);
    let imgdest = convert_image_from_cv(cvdest);
    return imgdest;
}

//Save image and thumbnail to desired storage
fn process_image(img: DynamicImage, name: &str) -> Result<(), std::io::Error>{

    let digest = md5::compute(img.raw_pixels());
    let namext = format!("{}_{:x}{}", name, digest, EXT);
    //Check if file already exists
    let count = read_dir(PATH)?.take_while(Result::is_ok)
                               .map(|e| e.unwrap().file_name().into_string())
                               .take_while(Result::is_ok)
                               .filter(|n| n.to_owned().unwrap().contains(namext.as_str())).count();
    if count > 0 {
        println!("{} already exists", namext);
        return Ok(());
    } else {
#[cfg(not(feature = "cvresize"))]
        let imres = img.resize(MAXSZ, MAXSZ, image::imageops::CatmullRom);

#[cfg(feature = "cvresize")]
        let imres = resize_with_opencv(&img, MAXSZ as i32);

        println!("Saving {} to {}", namext, TPATH);
        imres.save(format!("{}{}", TPATH, namext))?;
        println!("Saving {} to {}", namext, PATH);
        return img.save(format!("{}{}", PATH, namext));
    }
}




fn _submit_image(req :&mut Request) -> IronResult<Response> {
    let bad_j = (status::BadRequest, "Not a JSON format");
    let req_body = itry!(itry!(req.get::<bodyparser::Json>(), bad_j).ok_or(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Such an error")), bad_j);

    let imageobjs = unwrap_or_empty!(req_body.get("images"));  //There can be no "images" field in json data or no image inside it
    for obj in imageobjs {
        let imgstr = iexpect!(obj.as_str());
        let img = itry!(decode2image(imgstr), (status::InternalServerError, "Unable to decode base64 data."));
        itry!(process_image(img, "decoded"), (status::InternalServerError, "Unable to process decoded image."));
    }

    let urlobjs = unwrap_or_empty!(req_body.get("urls"));     //There can be no "urls" field in json data or no url inside it
    for obj in urlobjs {
        let url = iexpect!(obj.as_str());
        let img = itry!(fetch_image(url), (status::BadGateway, format!("Unable to fetch {}.", url)));
        itry!(process_image(img, "linked"), (status::InternalServerError, format!("Unable to process image {}.", url)));
    }
    return Ok(Response::with((
        status::Ok,
        Header(headers::ContentType::json()), "{\"result\":\"ok\"}"
    )));
}

//Wrapper around _submit_image for catch any possible error in one place
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
