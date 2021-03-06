extern crate hyper;
extern crate url;
extern crate serde;
extern crate serde_json;

use std::io::{Read, Write};
use std::io::Error as IoError;

use serde::de::Deserialize;
use hyper::client::Request as HyperRequest;
use hyper::client::Response as HyperResponse;
use hyper::method::Method;
use hyper::net::Fresh;

#[doc(no_inline)]
pub use hyper::header::{Headers, Header, HeaderFormat, UserAgent};
#[doc(no_inline)]
pub use url::Url;
#[doc(no_inline)]
pub use hyper::error::Error as HyperError;
#[doc(no_inline)]
pub use hyper::status::StatusCode;

#[derive(Debug)]
pub enum Error {
    UnsuccessfulResponse(Response),
    Json(serde_json::error::Error),
    Hyper(HyperError)
}

impl From<HyperError> for Error {
    fn from(h: HyperError) -> Error {
        Error::Hyper(h)
    }
}

impl From<IoError> for Error {
    fn from(i: IoError) -> Error {
        Error::Hyper(HyperError::Io(i))
    }
}

#[derive(Debug)]
pub struct Response {
    pub hyper_response: HyperResponse,
    pub body: String
}

impl Response {
    fn from_hyper_response(mut hyper_response: HyperResponse) -> Result<Response, IoError> {
        let mut body = String::new();
        hyper_response.read_to_string(&mut body).map(|_| Response{ hyper_response: hyper_response, body: body })
    }

    /// Deserializes the body of the response from JSON into
    /// a `T`.
    pub fn json_as<T: Deserialize>(&self) -> Result<T, Error> {
        serde_json::from_str(&*self.body).map_err(|e| Error::Json(e))
    }
}

#[derive(Clone)]
pub struct Request<'a> {
    url: Url,
    params: Option<Vec<(&'a str, &'a str)>>,
    body: Option<String>,
    headers: Option<Headers>,
}


impl<'a> Request<'a> {
    pub fn new(url: Url) -> Request<'a> {
        Request { url: url, params: None, body: None, headers: None }
    }

    /// Sets one parameter. On a GET or DELETE request, this parameter will
    /// be stored in the URL. On a POST or PUT request, it is stored in the
    /// body of the request. Hence, if you call this method on a POST or
    /// PUT request, you cannot also call `body`.
    pub fn param(&'a mut self, key: &'a str, value: &'a str) -> &'a mut Request<'a> {
        if let Some(ref mut p) = self.params {
            p.push((key, value));
        } else {
            let mut v = Vec::new();
            v.push((key, value));
            self.params = Some(v);
        }
        self
    }

    /// Sets many parameters. On a GET or DELETE request, these parameters will
    /// be stored in the URL. On a POST or PUT request, they are stored in the
    /// body of the request. Hence, if you call this method on a POST or
    /// PUT request, you cannot also call `body`.
    pub fn params<T>(&'a mut self, values: T) -> &'a mut Request<'a>
        where T: IntoIterator<Item = (&'a str, &'a str)>
    {
        if let Some(ref mut p) = self.params {
            for value in values {
                p.push(value);
            }
        } else {
            let mut v = Vec::new();
            for value in values {
                v.push(value);
            }
            self.params = Some(v);
        }
        self
    }

    /// Writes a `String` to the body of the request. Don't call this
    /// method if you also call `param` on a PUT or POST request.
    pub fn body(&'a mut self, body: String) -> &'a mut Request<'a> {
        self.body = Some(body);
        self
    }

    /// Sets a header for the request.
    pub fn header<H: Header + HeaderFormat>(&'a mut self, header: H) -> &'a mut Request<'a> {
        if let Some(ref mut h) = self.headers {
            h.set(header);
        } else {
            let mut v = Headers::new();
            v.set(header);
            self.headers = Some(v);
        }
        self
    }

    fn send_request(&mut self, mut req: HyperRequest<Fresh>) -> Result<Response, Error> {
        if let Some(headers) = self.headers.as_ref() {
            req.headers_mut().extend(headers.iter());
        }

        let mut req = try!(req.start());

        if let Some(body) = self.body.as_ref() {
            try!(req.write_all(body.as_bytes()));
        }

        let resp = try!(req.send());
        let resp = try!(Response::from_hyper_response(resp));

        if resp.hyper_response.status.is_success() {
            Ok(resp)
        } else {
            Err(Error::UnsuccessfulResponse(resp))
        }
    }

    /// Sends a GET request and returns either an error
    /// or a `String` of the response.
    pub fn get(&mut self) -> Result<Response, Error> {
        let mut url = self.url.clone();

        if let Some(ref params) = self.params {
            url.set_query_from_pairs(params.into_iter().map(|&x| x));
        }

        let req = try!(HyperRequest::new(Method::Get, url));
        self.send_request(req)
    }

    /// Sends a DELETE request and returns either an error
    /// or a `String` of the response.
    pub fn delete(&mut self) -> Result<Response, Error> {
        let mut url = self.url.clone();

        if let Some(ref params) = self.params {
            url.set_query_from_pairs(params.into_iter().map(|&x| x));
        }

        let req = try!(HyperRequest::new(Method::Delete, url));
        self.send_request(req)
    }

    /// Sends a POST request and returns either an error
    /// or a `String` of the response.
    pub fn post(&mut self) -> Result<Response, Error> {
        let url = self.url.clone();

        if let Some(ref params) = self.params {
            self.body = Some(url::form_urlencoded::serialize(params.into_iter()));
        }

        let req = try!(HyperRequest::new(Method::Post, url));
        self.send_request(req)
    }

    /// Sends a PUT request and returns either an error
    /// or a `String` of the response.
    pub fn put(&mut self) -> Result<Response, Error> {
        let url = self.url.clone();

        if let Some(ref params) = self.params {
            self.body = Some(url::form_urlencoded::serialize(params.into_iter()));
        }

        let req = try!(HyperRequest::new(Method::Put, url));
        self.send_request(req)
    }
}
