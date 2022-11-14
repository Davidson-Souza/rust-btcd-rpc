//! This module implements a minimal and non standard conforming HTTP 1.0
//! round-tripper that works with the bitcoind RPC server. This can be used
//! if minimal dependencies are a goal and synchronous communication is ok.

#[cfg(feature = "proxy")]
use socks::Socks5Stream;
use std::io::{BufRead, BufReader, Write};
#[cfg(not(feature = "proxy"))]
use std::net::TcpStream;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};
use std::{error, fmt, io, net, thread};

use base64;
use serde;
use serde_json;

use crate::client::Transport;
use crate::{Request, Response};

/// The default TCP port to use for connections.
/// Set to 8332, the default RPC port for bitcoind.
pub const DEFAULT_PORT: u16 = 8332;

/// The Default SOCKS5 Port to use for proxy connection.
pub const DEFAULT_PROXY_PORT: u16 = 9050;

/// Simple HTTP transport that implements the necessary subset of HTTP for
/// running a bitcoind RPC client.
#[derive(Clone, Debug)]
pub struct SimpleHttpTransport {
    addr: net::SocketAddr,
    path: String,
    timeout: Duration,
    /// The value of the `Authorization` HTTP header.
    basic_auth: Option<String>,
    #[cfg(feature = "proxy")]
    proxy_addr: net::SocketAddr,
    #[cfg(feature = "proxy")]
    proxy_auth: Option<(String, String)>,
}

impl Default for SimpleHttpTransport {
    fn default() -> Self {
        SimpleHttpTransport {
            addr: net::SocketAddr::new(
                net::IpAddr::V4(net::Ipv4Addr::new(127, 0, 0, 1)),
                DEFAULT_PORT,
            ),
            path: "/".to_owned(),
            timeout: Duration::from_secs(15),
            basic_auth: None,
            #[cfg(feature = "proxy")]
            proxy_addr: net::SocketAddr::new(
                net::IpAddr::V4(net::Ipv4Addr::new(127, 0, 0, 1)),
                DEFAULT_PROXY_PORT,
            ),
            #[cfg(feature = "proxy")]
            proxy_auth: None,
        }
    }
}

impl SimpleHttpTransport {
    /// Construct a new `SimpleHttpTransport` with default parameters
    pub fn new() -> Self {
        SimpleHttpTransport::default()
    }

    /// Returns a builder for `SimpleHttpTransport`
    pub fn builder() -> Builder {
        Builder::new()
    }

    fn request<R>(&self, req: impl serde::Serialize) -> Result<R, Error>
    where
        R: for<'a> serde::de::Deserialize<'a>,
    {
        // Open connection
        let request_deadline = Instant::now() + self.timeout;
        #[cfg(feature = "proxy")]
        let mut sock = if let Some((username, password)) = &self.proxy_auth {
            Socks5Stream::connect_with_password(
                self.proxy_addr,
                self.addr,
                username.as_str(),
                password.as_str(),
            )?
            .into_inner()
        } else {
            Socks5Stream::connect(self.proxy_addr, self.addr)?.into_inner()
        };

        #[cfg(not(feature = "proxy"))]
        let mut sock = TcpStream::connect_timeout(&self.addr, self.timeout)?;

        sock.set_read_timeout(Some(self.timeout))?;
        sock.set_write_timeout(Some(self.timeout))?;

        // Serialize the body first so we can set the Content-Length header.
        let body = serde_json::to_vec(&req)?;

        // Send HTTP request
        sock.write_all(b"POST ")?;
        sock.write_all(self.path.as_bytes())?;
        sock.write_all(b" HTTP/1.1\r\n")?;
        // Write headers
        sock.write_all(b"Connection: Close\r\n")?;
        sock.write_all(b"Content-Type: application/json\r\n")?;
        sock.write_all(b"Content-Length: ")?;
        sock.write_all(body.len().to_string().as_bytes())?;
        sock.write_all(b"\r\n")?;
        //sock.write_all(b"User-Agent: rust-jsonrpc\r\n")?;
        sock.write_all(b"Host: 127.0.0.1:65432\r\n")?;

        if let Some(ref auth) = self.basic_auth {
            sock.write_all(b"Authorization: ")?;
            sock.write_all(auth.as_ref())?;
            sock.write_all(b"\r\n")?;
        }
        // Write body
        sock.write_all(b"\r\n")?;
        sock.write_all(&body)?;
        sock.flush()?;

        // Receive response
        let mut reader = BufReader::new(sock);

        // Parse first HTTP response header line
        let http_response = get_line(&mut reader, request_deadline)?;
        if http_response.len() < 12 || !http_response.starts_with("HTTP/1.1 ") {
            return Err(Error::HttpParseError);
        }
        let response_code = match http_response[9..12].parse::<u16>() {
            Ok(n) => n,
            Err(_) => return Err(Error::HttpParseError),
        };

        // Skip response header fields
        while get_line(&mut reader, request_deadline)? != "\r\n" {}

        if response_code == 401 {
            // There is no body in a 401 response, so don't try to read it
            return Err(Error::HttpErrorCode(response_code));
        }

        // Even if it's != 200, we parse the response as we may get a JSONRPC error instead
        // of the less meaningful HTTP error code.
        let resp_body = get_lines(&mut reader)?;
        match serde_json::from_str(&resp_body) {
            Ok(s) => Ok(s),
            Err(e) => {
                if response_code != 200 {
                    Err(Error::HttpErrorCode(response_code))
                } else {
                    // If it was 200 then probably it was legitimately a parse error
                    Err(e.into())
                }
            }
        }
    }
}

/// Error that can happen when sending requests
#[derive(Debug)]
pub enum Error {
    /// An invalid URL was passed.
    InvalidUrl {
        /// The URL passed.
        url: String,
        /// The reason the URL is invalid.
        reason: &'static str,
    },
    /// An error occurred on the socket layer
    SocketError(io::Error),
    /// The HTTP header of the response couldn't be parsed
    HttpParseError,
    /// Unexpected HTTP error code (non-200)
    HttpErrorCode(u16),
    /// We didn't receive a complete response till the deadline ran out
    Timeout,
    /// JSON parsing error.
    Json(serde_json::Error),
}

impl Error {
    /// Utility method to create [Error::InvalidUrl] variants.
    fn url<U: Into<String>>(url: U, reason: &'static str) -> Error {
        Error::InvalidUrl {
            url: url.into(),
            reason,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Error::InvalidUrl {
                ref url,
                ref reason,
            } => write!(f, "invalid URL '{}': {}", url, reason),
            Error::SocketError(ref e) => write!(f, "Couldn't connect to host: {}", e),
            Error::HttpParseError => f.write_str("Couldn't parse response header."),
            Error::HttpErrorCode(c) => write!(f, "unexpected HTTP code: {}", c),
            Error::Timeout => f.write_str("Didn't receive response data in time, timed out."),
            Error::Json(ref e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use self::Error::*;

        match *self {
            InvalidUrl {
                ..
            }
            | HttpParseError
            | HttpErrorCode(_)
            | Timeout => None,
            SocketError(ref e) => Some(e),
            Json(ref e) => Some(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::SocketError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

impl From<Error> for crate::Error {
    fn from(e: Error) -> crate::Error {
        match e {
            Error::Json(e) => crate::Error::Json(e),
            e => crate::Error::Transport(Box::new(e)),
        }
    }
}

/// Try to read a line from a buffered reader. If no line can be read till the deadline is reached
/// return a timeout error.
fn get_line<R: BufRead>(reader: &mut R, deadline: Instant) -> Result<String, Error> {
    let mut line = String::new();
    while deadline > Instant::now() {
        match reader.read_line(&mut line) {
            // EOF reached for now, try again later
            Ok(0) => thread::sleep(Duration::from_millis(5)),
            // received useful data, return it
            Ok(_) => return Ok(line),
            // io error occurred, abort
            Err(e) => return Err(Error::SocketError(e)),
        }
    }
    Err(Error::Timeout)
}

/// Read all lines from a buffered reader.
fn get_lines<R: BufRead>(reader: &mut R) -> Result<String, Error> {
    let mut body: String = String::new();

    for line in reader.lines() {
        match line {
            Ok(l) => body.push_str(&l),
            // io error occurred, abort
            Err(e) => return Err(Error::SocketError(e)),
        }
    }
    // remove whitespace
    body.retain(|c| !c.is_whitespace());

    Ok(body)
}

/// Do some very basic manual URL parsing because the uri/url crates
/// all have unicode-normalization as a dependency and that's broken.
fn check_url(url: &str) -> Result<(SocketAddr, String), Error> {
    // The fallback port in case no port was provided.
    // This changes when the http or https scheme was provided.
    let mut fallback_port = DEFAULT_PORT;

    // We need to get the hostname and the port.
    // (1) Split scheme
    let after_scheme = {
        let mut split = url.splitn(2, "://");
        let s = split.next().unwrap();
        match split.next() {
            None => s, // no scheme present
            Some(after) => {
                // Check if the scheme is http or https.
                if s == "http" {
                    fallback_port = 80;
                } else if s == "https" {
                    fallback_port = 443;
                } else {
                    return Err(Error::url(url, "scheme should be http or https"));
                }
                after
            }
        }
    };
    // (2) split off path
    let (before_path, path) = {
        if let Some(slash) = after_scheme.find('/') {
            (&after_scheme[0..slash], &after_scheme[slash..])
        } else {
            (after_scheme, "/")
        }
    };
    // (3) split off auth part
    let after_auth = {
        let mut split = before_path.splitn(2, '@');
        let s = split.next().unwrap();
        split.next().unwrap_or(s)
    };

    // (4) Parse into socket address.
    // At this point we either have <host_name> or <host_name_>:<port>
    // `std::net::ToSocketAddrs` requires `&str` to have <host_name_>:<port> format.
    let mut addr = match after_auth.to_socket_addrs() {
        Ok(addr) => addr,
        Err(_) => {
            // Invalid socket address. Try to add port.
            format!("{}:{}", after_auth, fallback_port).to_socket_addrs()?
        }
    };

    match addr.next() {
        Some(a) => Ok((a, path.to_owned())),
        None => Err(Error::url(url, "invalid hostname: error extracting socket address")),
    }
}

impl Transport for SimpleHttpTransport {
    fn send_request(&self, req: Request) -> Result<Response, crate::Error> {
        Ok(self.request(req)?)
    }

    fn send_batch(&self, reqs: &[Request]) -> Result<Vec<Response>, crate::Error> {
        Ok(self.request(reqs)?)
    }

    fn fmt_target(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "http://{}:{}{}", self.addr.ip(), self.addr.port(), self.path)
    }
}

/// Builder for simple bitcoind `SimpleHttpTransport`s
#[derive(Clone, Debug)]
pub struct Builder {
    tp: SimpleHttpTransport,
}

impl Builder {
    /// Construct new `Builder` with default configuration
    pub fn new() -> Builder {
        Builder {
            tp: SimpleHttpTransport::new(),
        }
    }

    /// Sets the timeout after which requests will abort if they aren't finished
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.tp.timeout = timeout;
        self
    }

    /// Set the URL of the server to the transport.
    pub fn url(mut self, url: &str) -> Result<Self, Error> {
        let url = check_url(url)?;
        self.tp.addr = url.0;
        self.tp.path = url.1;
        Ok(self)
    }

    /// Add authentication information to the transport.
    pub fn auth<S: AsRef<str>>(mut self, user: S, pass: Option<S>) -> Self {
        let mut auth = user.as_ref().to_owned();
        auth.push(':');
        if let Some(ref pass) = pass {
            auth.push_str(pass.as_ref());
        }
        self.tp.basic_auth = Some(format!("Basic {}", &base64::encode(auth.as_bytes())));
        self
    }

    /// Add authentication information to the transport using a cookie string ('user:pass')
    pub fn cookie_auth<S: AsRef<str>>(mut self, cookie: S) -> Self {
        self.tp.basic_auth = Some(format!("Basic {}", &base64::encode(cookie.as_ref().as_bytes())));
        self
    }

    #[cfg(feature = "proxy")]
    /// Add proxy address to the transport for SOCKS5 proxy
    pub fn proxy_addr<S: AsRef<str>>(mut self, proxy_addr: S) -> Result<Self, Error> {
        // We don't expect path in proxy address.
        self.tp.proxy_addr = check_url(proxy_addr.as_ref())?.0;
        Ok(self)
    }

    #[cfg(feature = "proxy")]
    /// Add optional proxy authentication as ('username', 'password')
    pub fn proxy_auth<S: AsRef<str>>(mut self, user: S, pass: S) -> Self {
        self.tp.proxy_auth =
            Some((user, pass)).map(|(u, p)| (u.as_ref().to_string(), p.as_ref().to_string()));
        self
    }

    /// Builds the final `SimpleHttpTransport`
    pub fn build(self) -> SimpleHttpTransport {
        self.tp
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder::new()
    }
}

impl crate::Client {
    /// Create a new JSON-RPC client using a bare-minimum HTTP transport.
    pub fn simple_http(
        url: &str,
        user: Option<String>,
        pass: Option<String>,
    ) -> Result<crate::Client, Error> {
        let mut builder = Builder::new().url(url)?;
        if let Some(user) = user {
            builder = builder.auth(user, pass);
        }
        Ok(crate::Client::with_transport(builder.build()))
    }

    #[cfg(feature = "proxy")]
    /// Create a new JSON_RPC client using a HTTP-Socks5 proxy transport.
    pub fn http_proxy(
        url: &str,
        user: Option<String>,
        pass: Option<String>,
        proxy_addr: &str,
        proxy_auth: Option<(&str, &str)>,
    ) -> Result<crate::Client, Error> {
        let mut builder = Builder::new().url(url)?;
        if let Some(user) = user {
            builder = builder.auth(user, pass);
        }
        builder = builder.proxy_addr(proxy_addr)?;
        if let Some((user, pass)) = proxy_auth {
            builder = builder.proxy_auth(user, pass);
        }
        let tp = builder.build();
        Ok(crate::Client::with_transport(tp))
    }
}

#[cfg(test)]
mod tests {
    use std::net;
    #[cfg(feature = "proxy")]
    use std::str::FromStr;

    use super::*;
    use crate::Client;

    #[test]
    fn test_urls() {
        let addr: net::SocketAddr = ("localhost", 22).to_socket_addrs().unwrap().next().unwrap();
        let urls = [
            "localhost:22",
            "http://localhost:22/",
            "https://localhost:22/walletname/stuff?it=working",
            "http://me:weak@localhost:22/wallet",
        ];
        for u in &urls {
            let tp = Builder::new().url(*u).unwrap().build();
            assert_eq!(tp.addr, addr);
        }

        // Default port and 80 and 443 fill-in.
        let addr: net::SocketAddr = ("localhost", 80).to_socket_addrs().unwrap().next().unwrap();
        let tp = Builder::new().url("http://localhost/").unwrap().build();
        assert_eq!(tp.addr, addr);
        let addr: net::SocketAddr = ("localhost", 443).to_socket_addrs().unwrap().next().unwrap();
        let tp = Builder::new().url("https://localhost/").unwrap().build();
        assert_eq!(tp.addr, addr);
        let addr: net::SocketAddr =
            ("localhost", super::DEFAULT_PORT).to_socket_addrs().unwrap().next().unwrap();
        let tp = Builder::new().url("localhost").unwrap().build();
        assert_eq!(tp.addr, addr);

        let valid_urls = [
            "localhost",
            "127.0.0.1:8080",
            "http://127.0.0.1:8080/",
            "http://127.0.0.1:8080/rpc/test",
            "https://127.0.0.1/rpc/test",
            "http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8300",
            "http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]",
        ];
        for u in &valid_urls {
            let (addr, path) = check_url(u).unwrap();
            let builder = Builder::new().url(*u).unwrap_or_else(|_| panic!("error for: {}", u));
            assert_eq!(builder.tp.addr, addr);
            assert_eq!(builder.tp.path, path);
            assert_eq!(builder.tp.timeout, Duration::from_secs(15));
            assert_eq!(builder.tp.basic_auth, None);
            #[cfg(feature = "proxy")]
            assert_eq!(builder.tp.proxy_addr, SocketAddr::from_str("127.0.0.1:9050").unwrap());
        }

        let invalid_urls = [
            "127.0.0.1.0:8080",
            "httpx://127.0.0.1:8080/",
            "ftp://127.0.0.1:8080/rpc/test",
            "http://127.0.0./rpc/test",
            // NB somehow, Rust's IpAddr accepts "127.0.0" and adds the extra 0..
        ];
        for u in &invalid_urls {
            if let Ok(b) = Builder::new().url(*u) {
                let tp = b.build();
                panic!("expected error for url {}, got {:?}", u, tp);
            }
        }
    }

    #[test]
    fn construct() {
        let tp = Builder::new()
            .timeout(Duration::from_millis(100))
            .url("localhost:22")
            .unwrap()
            .auth("user", None)
            .build();
        let _ = Client::with_transport(tp);

        let _ = Client::simple_http("localhost:22", None, None).unwrap();
    }

    #[cfg(feature = "proxy")]
    #[test]
    fn construct_with_proxy() {
        let tp = Builder::new()
            .timeout(Duration::from_millis(100))
            .url("localhost:22")
            .unwrap()
            .auth("user", None)
            .proxy_addr("127.0.0.1:9050")
            .unwrap()
            .build();
        let _ = Client::with_transport(tp);

        let _ = Client::http_proxy(
            "localhost:22",
            None,
            None,
            "127.0.0.1:9050",
            Some(("user", "password")),
        )
        .unwrap();
    }
}
