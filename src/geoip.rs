static EMBEDDED_DB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/dbip-city-lite.mmdb"));

pub enum GeoIpDb {
    None,
    Embedded(maxminddb::Reader<&'static [u8]>),
    File(maxminddb::Reader<Vec<u8>>),
}

impl GeoIpDb {
    pub fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn lookup_city(&self, ip: std::net::IpAddr) -> Option<(f64, f64)> {
        let city: maxminddb::geoip2::City = match self {
            Self::Embedded(r) => r.lookup(ip).ok()?,
            Self::File(r) => r.lookup(ip).ok()?,
            Self::None => return None,
        };
        let loc = city.location.as_ref()?;
        Some((loc.latitude?, loc.longitude?))
    }
}

pub fn open() -> GeoIpDb {
    if let Some(ref path) = *crate::env::GEOIP_DB {
        match maxminddb::Reader::open_readfile(path) {
            Ok(r) => {
                log::info!("GeoIP database loaded from {path}");
                return GeoIpDb::File(r);
            }
            Err(e) => log::warn!("failed to load GeoIP database from {path}: {e}"),
        }
    }
    if !EMBEDDED_DB.is_empty() {
        match maxminddb::Reader::from_source(EMBEDDED_DB) {
            Ok(r) => {
                log::info!("GeoIP database loaded (embedded DB-IP Lite)");
                return GeoIpDb::Embedded(r);
            }
            Err(e) => log::warn!("embedded GeoIP DB invalid: {e}"),
        }
    }
    GeoIpDb::None
}
