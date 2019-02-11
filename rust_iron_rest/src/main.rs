extern crate bodyparser;
extern crate iron;
extern crate persistent;
#[macro_use]
extern crate router;
extern crate serde;
extern crate serde_json;

use std::collections::HashMap;
use iron::{headers, status};
use iron::modifiers::Header;
use iron::prelude::*;
use iron::typemap::Key;
use persistent::Write;
use router::Router;
use serde::ser::{Serialize, Serializer, SerializeStruct};

#[derive(Debug, PartialEq)]
pub struct Account {
    key: String,
    balance: f64,
}

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut state = serializer.serialize_struct("Account", 2)?;
        state.serialize_field("account", &self.key)?;
        state.serialize_field("balance", &format!("${:.2}", &self.balance))?;
        return state.end();
    }
}

#[derive(Copy, Clone)]
pub struct Accounts;

impl Key for Accounts {
    type Value = HashMap<String, Account>;
}

fn create_account(req :&mut Request) -> IronResult<Response> {
    let key;
    {
        key = req.extensions.get::<Router>().unwrap().find("key").unwrap().to_string();
        dbg!(key.clone());
    }

    let mutex = req.get::<Write<Accounts>>().unwrap();
    let mut accounts = mutex.lock().unwrap();
    if !accounts.contains_key(&key) {
        (*accounts).insert(key.clone(), Account{key: key.clone(), balance: 0f64});
    }

    return Ok(Response::with((
        status::Ok,
        Header(headers::ContentType::json()),
        &serde_json::to_string(accounts.get(&key).unwrap()).unwrap() as &str
    )));
}

fn submit_payment(req :&mut Request) -> IronResult<Response> {
    let key;
    {
        key = req.extensions.get::<Router>().unwrap().find("key").unwrap().to_string();
    }

    let req_body = req.get::<bodyparser::Json>().unwrap().unwrap();
    let amount: f64 = req_body.get("amount").unwrap().as_f64().unwrap();

    let mutex = req.get::<Write<Accounts>>().unwrap();
    let mut accounts = mutex.lock().unwrap();
    let ref mut account = *accounts.get_mut(&key).unwrap();
    account.balance += amount;

    return Ok(Response::with((
        status::Ok,
        Header(headers::ContentType::json()),
        &serde_json::to_string(&account).unwrap() as &str
    )));
}

fn main() {
    let router = router!(create_account: put "/accounts/:key" => create_account,
                         submit_payment: post "/accounts/:key/payments" => submit_payment);
    let mut chain = Chain::new(router);
    let accounts = HashMap::new();
    chain.link_before(Write::<Accounts>::one(accounts));
    Iron::new(chain).http("localhost:3000").unwrap();
}
