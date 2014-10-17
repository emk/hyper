
macro_rules! mock_stream (
    ($name:ident {
        $($url:expr => $res:expr)*
    }) => (
        struct $name {
            rcvr: ::std::io::MemWriter,
            res: ::std::io::BufReader<'static>,
        }

        impl Clone for $name {
            fn clone(&self) -> $name {
                fail!("cant clone BufReader")
            }
        }

        impl ::net::NetworkStream for $name {
            fn connect(host: &str, port: u16, scheme: &str) -> ::std::io::IoResult<$name> {
                use std::collections::HashMap;
                debug!("MockStream::connect({}, {}, {})", host, port, scheme);
                let mut map = HashMap::new();
                $(map.insert($url, $res);)*


                let key = format!("{}://{}", scheme, host);
                // ignore port for now
                match map.find(&key[]) {
                    Some(res) => Ok($name {
                        rcvr: ::std::io::MemWriter::new(),
                        res: ::std::io::BufReader::new(res.as_bytes())
                    }),
                    None => fail!("mock stream doesn't know url")
                }
            }

            fn peer_name(&mut self) -> ::std::io::IoResult<::std::io::net::ip::SocketAddr> {
                Ok(from_str("127.0.0.1:1337").unwrap())
            }
        }

        impl Reader for $name {
            fn read(&mut self, buf: &mut [u8]) -> ::std::io::IoResult<uint> {
                self.res.read(buf)
            }
        }

        impl Writer for $name {
            fn write(&mut self, msg: &[u8]) -> ::std::io::IoResult<()> {
                self.rcvr.write(msg)
            }
        }
    )
)
