use std::{io::ErrorKind, net::IpAddr, path::Path, sync::Arc};

use maxminddb::{MaxMindDbError, Reader, geoip2};

#[derive(Debug, Clone, Default)]
pub struct Location {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
}

#[derive(Clone)]
pub struct GeoIp {
    reader: Arc<Option<Reader<Vec<u8>>>>,
}

impl GeoIp {
    pub fn open(path: &Path) -> Self {
        let reader = match Reader::open_readfile(path) {
            Ok(reader) => Some(reader),
            Err(MaxMindDbError::Io(error)) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => {
                eprintln!("geoip database unavailable: {error}");
                None
            }
        };

        Self {
            reader: Arc::new(reader),
        }
    }

    pub fn lookup(&self, ip: Option<IpAddr>) -> Location {
        let Some(ip) = ip else {
            return Location::default();
        };

        if is_local_address(ip) {
            return Location::default();
        }

        let Some(reader) = self.reader.as_ref() else {
            return Location::default();
        };

        let Ok(Some(city)) = reader.lookup::<geoip2::City<'_>>(ip) else {
            return Location::default();
        };

        Location {
            country: city
                .country
                .and_then(|country| country.iso_code.map(ToOwned::to_owned)),
            region: city.subdivisions.and_then(|mut subdivisions| {
                subdivisions
                    .pop()
                    .and_then(|region| region.iso_code.map(ToOwned::to_owned))
            }),
            city: city.city.and_then(|city| {
                city.names
                    .and_then(|names| names.get("en").map(|value| (*value).to_string()))
            }),
        }
    }
}

fn is_local_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.octets()[0] == 0
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || matches!(ip.segments()[0] & 0xfe00, 0xfc00 | 0xfe80)
        }
    }
}
