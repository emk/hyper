//! HTTP Client

use std::default::Default;
//use std::io::util::copy;

use url::UrlParser;

use super::header::common::{Location};
use super::method::{Method, Get};
use super::net::NetworkStream;
use super::status;
use super::{Url, Port, HttpResult, HttpUriError};

pub use self::request::Request;
pub use self::response::Response;

pub mod request;
pub mod response;

/// A Client to use additional features with Requests.
///
/// Clients can handle things such as: redirect policy.
pub struct Client<S: NetworkStream> {
    redirect_policy: RedirectPolicy
}

/// Behavior regarding how to handle redirects within a Client.
pub enum RedirectPolicy {
    /// Don't follow any redirects.
    FollowNone,
    /// Follow all redirects.
    FollowAll,
    /// Follow a redirect if the contained function returns true.
    FollowIf(fn(&Url) -> bool)
}

impl<S: NetworkStream> Default for Client<S> {
    fn default() -> Client<S> {
        Client::new(FollowAll)
    }
}

impl<S: NetworkStream> Client<S> {

    /// Create a new Client.
    pub fn new(redirect_policy: RedirectPolicy) -> Client<S> {
        Client {
            redirect_policy: redirect_policy
        }
    }

    /// Execute a Get request.
    pub fn get(&mut self, url: Url) -> HttpResult<Response> {
        self.request(Get, url)
    }

    /// Execute a request using this Client.
    pub fn request(&mut self, method: Method, mut url: Url, headers: Option<Headers>) -> HttpResult<Response> {
        // self is &mut because in the future, this function will check
        // self.connection_pool, inserting if empty, when keep_alive = true.
        debug!("client.request {} {}", method, url);

        //let mut redirect_count = 0u;

        let mut headers = match headers {
            Some(h) => h,
            None => Headers::new()
        };

        loop {
            let req = try!(Request::with_stream::<S>(method.clone(), url.clone()));
            let streaming = try!(req.start());
            let res = try!(streaming.send());
            if res.status.class() != status::Redirection {
                return Ok(res)
            }
            debug!("redirect code {} for {}", res.status, url);

            let loc = {
                // punching borrowck here
                let loc = match res.headers.get::<Location>() {
                    Some(&Location(ref loc)) => {
                        Some(UrlParser::new().base_url(&url).parse(loc[]))
                    }
                    None => {
                        debug!("no Location header");
                        // could be 304 Not Modified?
                        None
                    }
                };
                match loc {
                    Some(r) => r,
                    None => return Ok(res)
                }
            };
            url = match loc {
                Ok(u) => {
                    debug!("Location: {}", u);
                    u
                },
                Err(e) => {
                    debug!("Location header had invalid URI: {}", e);
                    return Ok(res);
                }
            };
            match self.redirect_policy {
                // separate branches because they cant be one
                FollowAll => (),
                FollowIf(cond) if cond(&url) => (),
                _ => return Ok(res),
            }
            //redirect_count += 1;
        }
    }
}

fn get_host_and_port(url: &Url) -> HttpResult<(String, Port)> {
    let host = match url.serialize_host() {
        Some(host) => host,
        None => return Err(HttpUriError)
    };
    debug!("host={}", host);
    let port = match url.port_or_default() {
        Some(port) => port,
        None => return Err(HttpUriError)
    };
    debug!("port={}", port);
    Ok((host, port))
}

#[cfg(test)]
mod tests {
    use super::super::header::common::Server;
    use super::{Client, FollowAll, FollowNone, FollowIf};
    use url::Url;

    mock_stream!(MockRedirectPolicy {
        "http://mock.follow.all" => "HTTP/1.1 301 Redirect\r\n\
                                     Location: http://mock2.all\r\n\
                                     Server: mock1\r\n\
                                     \r\n\
                                    "
        "http://mock2.all" =>       "HTTP/1.1 302 Found\r\n\
                                     Location: https://mock3.done\r\n\
                                     Server: mock2\r\n\
                                     \r\n\
                                    "
        "https://mock3.done" =>      "HTTP/1.1 200 OK\r\n\
                                     Server: mock3\r\n\
                                     \r\n\
                                    "
    })

    #[test]
    fn test_redirect_followall() {
        let mut client: Client<MockRedirectPolicy> = Client::new(FollowAll);

        let res = client.get(Url::parse("http://mock.follow.all").unwrap()).unwrap();
        assert_eq!(res.headers.get(), Some(&Server("mock3".into_string())));
    }

    #[test]
    fn test_redirect_dontfollow() {
        let mut client: Client<MockRedirectPolicy> = Client::new(FollowNone);
        let res = client.get(Url::parse("http://mock.follow.all").unwrap()).unwrap();
        assert_eq!(res.headers.get(), Some(&Server("mock1".into_string())));
    }

    #[test]
    fn test_redirect_followif() {
        fn follow_if(url: &Url) -> bool {
            !url.serialize()[].contains("mock3.done")
        }
        let mut client: Client<MockRedirectPolicy> = Client::new(FollowIf(follow_if));
        let res = client.get(Url::parse("http://mock.follow.all").unwrap()).unwrap();
        assert_eq!(res.headers.get(), Some(&Server("mock2".into_string())));
    }

}
