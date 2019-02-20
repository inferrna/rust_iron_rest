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
use iron::{headers, status, IronError};
use iron::modifiers::Header;
use image::{ImageResult, DynamicImage, ImageError};
use bodyparser::BodyError;

use std::io::BufWriter;
use std::io::prelude::*;
use std::fs::{File, read_dir};
use std::cmp::{max, min};
use std::error;
use std::fmt;

const MAXSZ: u32 = 100;    //New size
const PATH: &str = "/tmp/images/";
const TPATH: &str = "/tmp/images/thumbs/";
const EXT: &str = ".jpg";


#[derive(Debug)]
enum CommonError {
    Image(ImageError),
    Server(IronError),
    Json(BodyError),
    Req(reqwest::Error),
    Io(std::io::Error),
    B64(base64::DecodeError),
}

impl fmt::Display for CommonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            // Both underlying errors already impl `Display`, so we defer to
            // their implementations.
            CommonError::Image(ref err) => write!(f, "Image error: {}", err),
            CommonError::Server(ref err) => write!(f, "Server error: {}", err),
            CommonError::Json(ref err) => write!(f, "Json error: {}", err),
            CommonError::Req(ref err) => write!(f, "Reqwest error: {}", err),
            CommonError::Io(ref err) => write!(f, "IO error: {}", err),
            CommonError::B64(ref err) => write!(f, "Base64 error: {}", err),
        }
    }
}

impl error::Error for CommonError {
    fn description(&self) -> &str {
        // Both underlying errors already impl `Error`, so we defer to their
        // implementations.
        match *self {
            CommonError::Image(ref err) => err.description(),
            CommonError::Server(ref err) => err.description(),
            CommonError::Json(ref err)  => err.description(),
            CommonError::Req(ref err)  => err.description(),
            CommonError::Io(ref err)  => err.description(),
            CommonError::B64(ref err)  => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            // N.B. Both of these implicitly cast `err` from their concrete
            // types (either `&io::Error` or `&num::ParseIntError`)
            // to a trait object `&Error`. This works because both error types
            // implement `Error`.
            CommonError::Image(ref err) => Some(err),
            CommonError::Server(ref err) => Some(err),
            CommonError::Json(ref err)  => Some(err),
            CommonError::Req(ref err)  => Some(err),
            CommonError::Io(ref err)  => Some(err),
            CommonError::B64(ref err)  => Some(err),
        }
    }
}

impl From<ImageError> for CommonError {
    fn from(err: ImageError) -> CommonError {
        CommonError::Image(err)
    }
}
impl From<IronError> for CommonError {
    fn from(err: IronError) -> CommonError {
        CommonError::Server(err)
    }
}
impl From<BodyError> for CommonError {
    fn from(err: BodyError) -> CommonError {
        CommonError::Json(err)
    }
}
impl From<reqwest::Error> for CommonError {
    fn from(err: reqwest::Error) -> CommonError {
        CommonError::Req(err)
    }
}
impl From<std::io::Error> for CommonError {
    fn from(err: std::io::Error) -> CommonError {
        CommonError::Io(err)
    }
}
impl From<base64::DecodeError> for CommonError {
    fn from(err: base64::DecodeError) -> CommonError {
        CommonError::B64(err)
    }
}

#[derive(Debug)]
struct StringError(String);

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl error::Error for StringError {
    fn description(&self) -> &str {
        &*self.0
    }
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

//Decode base64 string to DynamicImage
fn decode2image(imstr: &str) -> Result<DynamicImage, CommonError>{
    let mut buf: Vec<u8> = vec![];
    base64::decode_config_buf(imstr, base64::STANDARD, &mut buf)?;
    /* Check correctness of just decoded image file.
    let file = File::create("/tmp/decoded_raw.png")?;
    let mut bw = BufWriter::new(file);
    dbg!(bw.write_all(&buf));
    */
    let result = image::load_from_memory(&buf)?;
    return Ok(result);
}

//Fetch file by link and convert to DynamicImage
fn fetch_image(url: &str) -> Result<DynamicImage, CommonError>{
    let mut resp = reqwest::get(url)?;
    println!("Fetched {}", url);
    if !resp.status().is_success() {
        return Err(CommonError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Unable to load {} with '{}'", url, resp.status()))));
    }
    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf)?;
    let result = image::load_from_memory(&buf)?;
    return Ok(result);
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
fn process_image(img: DynamicImage, name: &str) -> Result<(), CommonError>{

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
        img.save(format!("{}{}", PATH, namext))?;
    }
    return Ok(());
}




fn _submit_image(req :&mut Request) -> Result<Response, CommonError> {
    let req_body = req.get::<bodyparser::Json>()?.expect("Empty json body.");
    let imageobjs = unwrap_or_empty!(req_body.get("images"));  //There can be no "images" field in json data or no image inside it
    for obj in imageobjs {
        let imgstr = iexpect!(obj.as_str());
        let img = decode2image(imgstr)?;
        process_image(img, "decoded")?;
    }

    let urlobjs = unwrap_or_empty!(req_body.get("urls"));     //There can be no "urls" field in json data or no url inside it
    for obj in urlobjs {
        let url = iexpect!(obj.as_str());
        let img = fetch_image(url)?;
        process_image(img, "linked")?;
    }
    return Ok(Response::with((
        status::Ok,
        Header(headers::ContentType::json()), "{\"result\":\"ok\"}"
    )));
}

//Wrapper around _submit_image for catch any possible error in one place
fn submit_image(req :&mut Request) -> IronResult<Response> {
    let res = _submit_image(req);
    if res.is_err(){
        dbg!(&res);
    }
    match res {
        Ok(n) => Ok(n),
        Err(err) => Err(IronError::new(StringError(err.to_string()),
                                       (status::InternalServerError, err.to_string()))),
    }
}

fn main() {
    let router = router!(submit_image: post "/upload" => submit_image);
    let chain = Chain::new(router);
    Iron::new(chain).http("localhost:3000").unwrap();
}
