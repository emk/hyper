extern crate hyper;

use std::default::Default;
use std::os;
use std::io::stdout;
use std::io::util::copy;

use hyper::Url;
use hyper::client::Client;
use hyper::net::HttpStream;

fn main() {
    let args = os::args();
    match args.len() {
        2 => (),
        _ => {
            println!("Usage: client <url>");
            return;
        }
    };

    let url = match Url::parse(args[1].as_slice()) {
        Ok(url) => {
            println!("GET {}...", url)
            url
        },
        Err(e) => panic!("Invalid URL: {}", e)
    };

    let mut client: Client<HttpStream> = Default::default();

    let mut res = match client.get(url) {
        Ok(res) => res,
        Err(err) => panic!("Failed to connect: {}", err)
    };

    println!("Response: {}", res.status);
    println!("Headers:\n{}", res.headers);
    match copy(&mut res, &mut stdout()) {
        Ok(..) => (),
        Err(e) => panic!("Stream failure: {}", e)
    };

}
